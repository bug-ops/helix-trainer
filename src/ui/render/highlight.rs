//! Syntax highlighting for code content using syntect

use std::sync::LazyLock;

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

/// Highlight code content and return styled lines for ratatui
///
/// Uses Rust syntax highlighting with a dark theme suitable for terminals.
#[allow(dead_code)]
pub fn highlight_code(content: &str) -> Vec<Line<'static>> {
    // Use Rust syntax (our scenarios are Rust code)
    let syntax = SYNTAX_SET
        .find_syntax_by_extension("rs")
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    // Use base16-eighties dark theme (good for terminals)
    let theme = &THEME_SET.themes["base16-eighties.dark"];

    let mut highlighter = HighlightLines::new(syntax, theme);

    content
        .lines()
        .map(|line| {
            // Highlight the line
            let highlighted = highlighter
                .highlight_line(line, &SYNTAX_SET)
                .unwrap_or_default();

            // Convert to ratatui spans
            let spans: Vec<Span<'static>> = highlighted
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                })
                .collect();

            Line::from(spans)
        })
        .collect()
}

/// Highlight code with cursor and selection overlay
///
/// Combines syntax highlighting with cursor/selection display.
pub fn highlight_code_with_cursor(
    content: &str,
    cursor_line: usize,
    cursor_col: usize,
    selection: Option<&crate::game::Selection>,
) -> Vec<Line<'static>> {
    use super::helpers::{char_range_to_bytes, split_at_char_index};

    let syntax = SYNTAX_SET
        .find_syntax_by_extension("rs")
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = &THEME_SET.themes["base16-eighties.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    content
        .lines()
        .enumerate()
        .map(|(line_idx, line_text)| {
            // Check for selection on this line
            if let Some(sel) = selection {
                let sel_start_line = sel.start.row;
                let sel_end_line = sel.end.row;

                // Skip if this is the end line but end_col is 0
                let line_has_selection = if line_idx == sel_end_line && sel.end.col == 0 {
                    false
                } else {
                    line_idx >= sel_start_line && line_idx <= sel_end_line
                };

                if line_has_selection {
                    // Get highlighted spans first
                    let highlighted = highlighter
                        .highlight_line(line_text, &SYNTAX_SET)
                        .unwrap_or_default();

                    // Determine selection range for this line
                    let line_start_col = if line_idx == sel_start_line {
                        sel.start.col
                    } else {
                        0
                    };

                    let line_end_col = if line_idx == sel_end_line {
                        sel.end.col
                    } else {
                        line_text.chars().count()
                    };

                    let (start_byte, end_byte) =
                        char_range_to_bytes(line_text, line_start_col, line_end_col);

                    // Build spans with selection overlay
                    let mut spans: Vec<Span<'static>> = Vec::new();
                    let mut current_byte = 0;

                    for (style, text) in highlighted {
                        let text_start = current_byte;
                        let text_end = current_byte + text.len();

                        if text_end <= start_byte || text_start >= end_byte {
                            // Completely outside selection
                            spans.push(Span::styled(
                                text.to_string(),
                                syntect_to_ratatui_style(style),
                            ));
                        } else if text_start >= start_byte && text_end <= end_byte {
                            // Completely inside selection
                            spans.push(Span::styled(
                                text.to_string(),
                                syntect_to_ratatui_style(style)
                                    .bg(super::SELECTION_BG_COLOR)
                                    .fg(Color::White),
                            ));
                        } else {
                            // Partially inside selection - split
                            let sel_start_in_text = start_byte.saturating_sub(text_start);
                            let sel_end_in_text = (end_byte - text_start).min(text.len());

                            if sel_start_in_text > 0 {
                                spans.push(Span::styled(
                                    text[..sel_start_in_text].to_string(),
                                    syntect_to_ratatui_style(style),
                                ));
                            }

                            if sel_end_in_text > sel_start_in_text {
                                spans.push(Span::styled(
                                    text[sel_start_in_text..sel_end_in_text].to_string(),
                                    syntect_to_ratatui_style(style)
                                        .bg(super::SELECTION_BG_COLOR)
                                        .fg(Color::White),
                                ));
                            }

                            if sel_end_in_text < text.len() {
                                spans.push(Span::styled(
                                    text[sel_end_in_text..].to_string(),
                                    syntect_to_ratatui_style(style),
                                ));
                            }
                        }

                        current_byte = text_end;
                    }

                    return Line::from(spans);
                }
            }

            // Check if cursor is on this line
            if line_idx == cursor_line {
                let highlighted = highlighter
                    .highlight_line(line_text, &SYNTAX_SET)
                    .unwrap_or_default();

                let (_before_end, char_start, char_end, after_start) =
                    split_at_char_index(line_text, cursor_col);

                // Build spans with cursor overlay
                let mut spans: Vec<Span<'static>> = Vec::new();
                let mut current_byte = 0;

                for (style, text) in highlighted {
                    let text_start = current_byte;
                    let text_end = current_byte + text.len();

                    if text_end <= char_start || text_start >= after_start {
                        // Outside cursor
                        spans.push(Span::styled(
                            text.to_string(),
                            syntect_to_ratatui_style(style),
                        ));
                    } else if text_start <= char_start && text_end >= after_start {
                        // Cursor is within this span
                        let cursor_start_in_text = char_start - text_start;
                        let cursor_end_in_text = char_end - text_start;

                        if cursor_start_in_text > 0 {
                            spans.push(Span::styled(
                                text[..cursor_start_in_text].to_string(),
                                syntect_to_ratatui_style(style),
                            ));
                        }

                        let cursor_char = &text[cursor_start_in_text..cursor_end_in_text];
                        let cursor_display = if cursor_char.is_empty() {
                            "\u{2588}".to_string()
                        } else {
                            cursor_char.to_string()
                        };
                        spans.push(Span::styled(
                            cursor_display,
                            Style::default()
                                .bg(Color::White)
                                .fg(Color::Black)
                                .add_modifier(ratatui::style::Modifier::BOLD),
                        ));

                        if cursor_end_in_text < text.len() {
                            spans.push(Span::styled(
                                text[cursor_end_in_text..].to_string(),
                                syntect_to_ratatui_style(style),
                            ));
                        }
                    } else {
                        // Partially overlaps cursor - simplified handling
                        spans.push(Span::styled(
                            text.to_string(),
                            syntect_to_ratatui_style(style),
                        ));
                    }

                    current_byte = text_end;
                }

                // Handle empty line cursor
                if line_text.is_empty() {
                    spans.push(Span::styled(
                        "\u{2588}".to_string(),
                        Style::default()
                            .bg(Color::White)
                            .fg(Color::Black)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ));
                }

                return Line::from(spans);
            }

            // Regular line - just highlight
            let highlighted = highlighter
                .highlight_line(line_text, &SYNTAX_SET)
                .unwrap_or_default();

            let spans: Vec<Span<'static>> = highlighted
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                })
                .collect();

            Line::from(spans)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_code() {
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = highlight_code(code);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_highlight_empty_content() {
        let lines = highlight_code("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_syntax_set_loads() {
        // Force lazy initialization
        let _ = SYNTAX_SET.find_syntax_by_extension("rs");
        assert!(SYNTAX_SET.find_syntax_by_extension("rs").is_some());
    }
}
