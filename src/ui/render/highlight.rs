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
