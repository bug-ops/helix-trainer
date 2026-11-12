//! Syntax highlighting for code content using syntect

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::helix::{Language, SelectionBounds};
use crate::ui::render::editor::CursorInfo;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

/// Global syntax set (loaded once)
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Global theme set (loaded once)
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Short sample per scenario language.
const PREWARM_SAMPLES: [(Language, &str); 3] = [
    (Language::Rs, "fn main() { let x = 1; }"),
    (
        Language::Md,
        "# Title\n\nSome *emphasis* and `code`.\n\n- item\n\n```rs\nfn f() {}\n```",
    ),
    (Language::Py, "def f(x):\n    return x"),
];

fn prewarm() {
    for (language, sample) in PREWARM_SAMPLES {
        drop(highlight_code_with_multi_cursor(sample, &[], &[], language));
    }
}

/// Spawns a background thread that highlights a short sample of `rs`, `md` and `py`.
///
/// Forces the shared syntax/theme sets and per-language regex compilation off the render
/// thread, so the first scenario view of each language does not hitch. Call it right after
/// the first frame is drawn.
///
/// # Errors
///
/// Returns the OS error when the thread cannot be spawned; the caller may ignore it, as it
/// only costs the warm-up.
pub fn spawn_prewarm() -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("syntect-prewarm".into())
        .spawn(prewarm)
}

/// The file-extension token syntect knows `language` by.
const fn syntect_extension(language: Language) -> &'static str {
    match language {
        Language::Rs => "rs",
        Language::Md => "md",
        Language::Py => "py",
    }
}

fn resolve_syntax(language: Language) -> &'static SyntaxReference {
    SYNTAX_SET
        .find_syntax_by_extension(syntect_extension(language))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
}

/// Convert syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Convert syntect style to ratatui style
fn syntect_to_ratatui_style(style: syntect::highlighting::Style) -> Style {
    let mut ratatui_style = Style::default().fg(syntect_to_ratatui_color(style.foreground));

    if style.font_style.contains(FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(ratatui::style::Modifier::UNDERLINED);
    }

    ratatui_style
}

/// Check if a line has selection (accounting for edge cases)
fn line_has_selection(line_idx: usize, sel: &SelectionBounds) -> bool {
    if line_idx == sel.end_row && sel.end_col == 0 {
        return false;
    }
    line_idx >= sel.start_row && line_idx <= sel.end_row
}

/// Highlight code with syntax highlighting, multi-cursor and multi-selection support.
///
/// Uses syntect for syntax highlighting while overlaying cursor and selection styles.
/// Primary cursor uses white background, secondary cursors use cyan background.
/// Selections use blue background.
///
/// `language` is resolved to its syntax in syntect's bundled `SyntaxSet` by file extension;
/// a test asserts every [`Language`] variant resolves to a non-plain syntax.
#[expect(
    clippy::similar_names,
    reason = "parallel names mirror the values they bind"
)]
pub(crate) fn highlight_code_with_multi_cursor(
    content: &str,
    cursors: &[CursorInfo],
    selections: &[SelectionBounds],
    language: Language,
) -> Vec<Line<'static>> {
    let syntax = resolve_syntax(language);

    let theme = &THEME_SET.themes["base16-eighties.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    content
        .lines()
        .enumerate()
        .map(|(line_idx, line_text)| {
            // Get highlighted spans from syntect
            let highlighted = highlighter
                .highlight_line(line_text, &SYNTAX_SET)
                .unwrap_or_default();

            // Check for selections on this line
            let line_has_any_selection = selections
                .iter()
                .any(|sel| line_has_selection(line_idx, sel));

            // Get cursors on this line
            let cursors_on_line: Vec<&CursorInfo> =
                cursors.iter().filter(|c| c.row == line_idx).collect();

            // If no overlays needed, just return syntax highlighted line
            if !line_has_any_selection && cursors_on_line.is_empty() {
                let spans: Vec<Span<'static>> = highlighted
                    .into_iter()
                    .map(|(style, text)| {
                        Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                    })
                    .collect();
                return Line::from(spans);
            }

            // Build per-character style map from syntax highlighting
            let mut char_styles: Vec<Style> = Vec::new();
            for (style, text) in &highlighted {
                let ratatui_style = syntect_to_ratatui_style(*style);
                for _ in text.chars() {
                    char_styles.push(ratatui_style);
                }
            }

            // Build set of highlighted columns from selections
            let line_chars: Vec<char> = line_text.chars().collect();
            let mut highlighted_cols: HashSet<usize> = HashSet::new();
            for sel in selections {
                if !line_has_selection(line_idx, sel) {
                    continue;
                }
                let start_col = if line_idx == sel.start_row {
                    sel.start_col
                } else {
                    0
                };
                let end_col = if line_idx == sel.end_row {
                    sel.end_col
                } else {
                    line_chars.len()
                };
                for col in start_col..end_col {
                    highlighted_cols.insert(col);
                }
            }

            let mut spans: Vec<Span<'static>> = Vec::new();

            for (col, ch) in line_chars.iter().enumerate() {
                // Check if cursor is at this position
                let cursor_at_pos = cursors_on_line.iter().find(|c| c.col == col);

                if let Some(cursor) = cursor_at_pos {
                    // Cursor takes priority
                    spans.push(Span::styled(ch.to_string(), cursor.style()));
                } else if highlighted_cols.contains(&col) {
                    // Selection highlight - preserve syntax foreground color
                    let fg_color = char_styles
                        .get(col)
                        .and_then(|s| s.fg)
                        .unwrap_or(Color::White);
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().bg(super::SELECTION_BG_COLOR).fg(fg_color),
                    ));
                } else {
                    // Regular character with syntax highlighting
                    let style = char_styles.get(col).copied().unwrap_or_else(Style::default);
                    spans.push(Span::styled(ch.to_string(), style));
                }
            }

            // Handle cursor at end of line
            for cursor in &cursors_on_line {
                if cursor.col >= line_chars.len() {
                    spans.push(Span::styled("\u{2588}".to_string(), cursor.style()));
                }
            }

            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prewarm_samples_resolve_to_real_syntaxes() {
        let plain = SYNTAX_SET.find_syntax_plain_text().name.clone();
        for (language, _) in PREWARM_SAMPLES {
            assert_ne!(resolve_syntax(language).name, plain, "{language:?}");
        }
    }

    #[test]
    fn prewarm_samples_produce_several_styles() {
        for (language, sample) in PREWARM_SAMPLES {
            let lines = highlight_code_with_multi_cursor(sample, &[], &[], language);
            let styles: HashSet<Style> = lines
                .iter()
                .flat_map(|l| l.spans.iter().map(|s| s.style))
                .collect();
            assert!(styles.len() > 1, "{language:?}");
        }
    }

    #[test]
    fn spawn_prewarm_thread_completes_without_panic() {
        let handle = spawn_prewarm().expect("thread spawns");
        handle.join().expect("pre-warm thread must not panic");
    }

    #[test]
    fn test_syntax_set_loads() {
        // Force lazy initialization
        let _ = SYNTAX_SET.find_syntax_by_extension("rs");
        assert!(SYNTAX_SET.find_syntax_by_extension("rs").is_some());
    }

    #[test]
    fn test_highlight_with_multi_cursor_no_overlays() {
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = highlight_code_with_multi_cursor(code, &[], &[], Language::Rs);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn test_highlight_with_multi_cursor_empty() {
        let lines = highlight_code_with_multi_cursor("", &[], &[], Language::Rs);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_highlight_with_cursor() {
        let code = "hello";
        let cursor = CursorInfo {
            row: 0,
            col: 2,
            is_primary: true,
        };
        let lines = highlight_code_with_multi_cursor(code, &[cursor], &[], Language::Rs);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_highlight_with_markdown_language() {
        // FR-003: a bundled non-Rust extension resolves to its own syntax definition.
        assert!(SYNTAX_SET.find_syntax_by_extension("md").is_some());
        let lines = highlight_code_with_multi_cursor("# Heading", &[], &[], Language::Md);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_highlight_rust_produces_multiple_styled_tokens() {
        // Baseline for the plain-text-fallback assertions above: real Rust highlighting of
        // the same content splits it into several differently-styled spans.
        let lines = highlight_code_with_multi_cursor("fn main() {}", &[], &[], Language::Rs);
        assert!(lines[0].spans.len() > 1);
    }
}
