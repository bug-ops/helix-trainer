//! Syntax highlighting for code content using syntect

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::helix::SelectionBounds;
use crate::ui::render::editor::CursorInfo;
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::SyntaxSet,
};

/// Global syntax set (loaded once)
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);

/// Global theme set (loaded once)
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

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
pub fn highlight_code_with_multi_cursor(
    content: &str,
    cursors: &[CursorInfo],
    selections: &[SelectionBounds],
) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_by_extension("rs")
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

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
    fn test_syntax_set_loads() {
        // Force lazy initialization
        let _ = SYNTAX_SET.find_syntax_by_extension("rs");
        assert!(SYNTAX_SET.find_syntax_by_extension("rs").is_some());
    }

    #[test]
    fn test_highlight_with_multi_cursor_no_overlays() {
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = highlight_code_with_multi_cursor(code, &[], &[]);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_highlight_with_multi_cursor_empty() {
        let lines = highlight_code_with_multi_cursor("", &[], &[]);
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
        let lines = highlight_code_with_multi_cursor(code, &[cursor], &[]);
        assert_eq!(lines.len(), 1);
    }
}
