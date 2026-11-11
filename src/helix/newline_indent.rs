//! Indent of the line `o`, `O` and Enter create, by a per-language rule that
//! needs no syntax tree.
//!
//! `hx` derives this indent from tree-sitter queries. The rule here
//! approximates them with a bracket scan over the document and a few
//! per-language line patterns; see the helix-core readiness spec (3.8.4) for
//! the cases it still does not match.

mod lexer;

use super::indent::IndentUnit;
use super::language::{LanguageRules, NewlineRules};
use helix_core::Rope;
use helix_core::line_ending::line_end_char_index;
use lexer::{Kind, Lexed};

/// Whether a line opened above a comment line continues that comment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentContinuation {
    /// `O`: the new line starts with the comment token
    Continue,
    /// A change of whole lines: the new line takes plain indentation
    Skip,
}

/// What a new line starts with: its indent and, when it continues a comment,
/// the comment token
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewLineStart {
    pub indent: String,
    pub comment_token: Option<&'static str>,
}

impl NewLineStart {
    fn plain(indent: String) -> Self {
        Self {
            indent,
            comment_token: None,
        }
    }

    /// Text the new line starts with: the indent, then `token ` when the line
    /// continues a comment
    pub fn text(&self) -> String {
        match self.comment_token {
            Some(token) => format!("{}{token} ", self.indent),
            None => self.indent.clone(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Paren,
    Brace,
    Square,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Open,
    Close,
}

/// A bracket outside strings and comments, at a document position
struct Bracket {
    line: usize,
    col: usize,
    shape: Shape,
    side: Side,
}

fn classify(ch: char) -> Option<(Shape, Side)> {
    match ch {
        '(' => Some((Shape::Paren, Side::Open)),
        ')' => Some((Shape::Paren, Side::Close)),
        '{' => Some((Shape::Brace, Side::Open)),
        '}' => Some((Shape::Brace, Side::Close)),
        '[' => Some((Shape::Square, Side::Open)),
        ']' => Some((Shape::Square, Side::Close)),
        _ => None,
    }
}

/// The identifier-like word `text` starts with after leading blanks
fn first_word(text: &str) -> String {
    text.trim_start()
        .chars()
        .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
        .collect()
}

/// The run of spaces and tabs at the start of `line`
pub(crate) fn leading_whitespace(doc: &Rope, line: usize) -> String {
    doc.line(line)
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .collect()
}

/// Indent rule for one document and language
pub(crate) struct NewlineIndent<'a> {
    pub doc: &'a Rope,
    pub language: &'a LanguageRules,
    pub unit: IndentUnit,
}

impl NewlineIndent<'_> {
    fn line_text(&self, line: usize) -> String {
        self.doc
            .line(line)
            .to_string()
            .trim_end_matches(['\n', '\r'])
            .to_string()
    }

    fn leading_whitespace(&self, line: usize) -> String {
        leading_whitespace(self.doc, line)
    }

    /// Start of the line `o` opens below `line`; `text_end` is the char index
    /// where the line's text ends for this purpose (the cursor for Enter)
    pub fn below(&self, line: usize, text_end: usize) -> NewLineStart {
        let text = self.line_text(line);
        let own_indent = self.leading_whitespace(line);
        let cut = text_end.saturating_sub(self.doc.line_to_char(line));
        let prefix: String = text.chars().take(cut).collect();

        if let Some(token) = self.language.leading_comment_token(&text) {
            return NewLineStart {
                indent: own_indent,
                comment_token: Some(token),
            };
        }
        let Some(rules) = &self.language.newline else {
            return NewLineStart::plain(own_indent);
        };
        if prefix.trim().is_empty() {
            return NewLineStart::plain(self.indent_below_blank(line, own_indent));
        }
        NewLineStart::plain(self.code_indent(line, cut, rules))
    }

    /// A blank line takes the indent `o` gives below the nearest non-blank
    /// line above it
    fn indent_below_blank(&self, line: usize, own_indent: String) -> String {
        (0..line)
            .rev()
            .find(|&above| !self.line_text(above).trim().is_empty())
            .map_or(own_indent, |above| {
                let end = line_end_char_index(&self.doc.slice(..), above);
                self.below(above, end).indent
            })
    }

    fn code_indent(&self, line: usize, cut: usize, rules: &NewlineRules) -> String {
        let lexed = Lexed::new(self.doc, self.language, rules);
        let (opens_bracket, base_line) = Self::bracket_state(&lexed, line, cut);
        let last_code = lexed
            .last_code_char(line, cut)
            .filter(|&(_, _, kind)| kind == Kind::Code);
        let opens_by_ending = last_code.is_some_and(|(col, last, _)| {
            rules.trailing_openers.contains(&last) && self.starts_block(&lexed, line, col, rules)
        });
        // An unclosed trailing bracket still opens: approximates hx auto-pair inserting the closer.
        let ends_with_opener =
            last_code.is_some_and(|(_, last, _)| matches!(classify(last), Some((_, Side::Open))));

        let base = self.leading_whitespace(base_line);
        if opens_bracket || opens_by_ending || ends_with_opener {
            return format!("{base}{}", self.unit);
        }
        let base_text = self.line_text(base_line);
        let first_word = first_word(&base_text);
        if !lexed.has_comment(base_line) && rules.dedent_words.contains(&first_word.as_str()) {
            return self.dedent(&base);
        }
        base
    }

    /// Whether a trailing opener at `(line, col)` starts a block: the line
    /// begins with a compound-statement keyword and the opener is outside
    /// every bracket (so `lambda:`, dict and slice colons do not open)
    fn starts_block(&self, lexed: &Lexed, line: usize, col: usize, rules: &NewlineRules) -> bool {
        let text = self.line_text(line);
        rules.block_keywords.contains(&first_word(&text).as_str())
            && Self::bracket_depth(lexed, line, col) == 0
    }

    /// Number of brackets still open just before `(line, col)`
    fn bracket_depth(lexed: &Lexed, line: usize, col: usize) -> usize {
        let split = lexed
            .brackets
            .partition_point(|b| (b.line, b.col) < (line, col));
        let mut stack: Vec<Shape> = Vec::new();
        for bracket in &lexed.brackets[..split] {
            match bracket.side {
                Side::Open => stack.push(bracket.shape),
                Side::Close => {
                    if stack.last() == Some(&bracket.shape) {
                        stack.pop();
                    }
                }
            }
        }
        stack.len()
    }

    /// Whether the text before `(line, cut)` leaves a bracket opened on this
    /// line that the rest of the document closes, and the line whose indent
    /// the new line is relative to (the opener's line when this line closes a
    /// bracket opened above)
    fn bracket_state(lexed: &Lexed, line: usize, cut: usize) -> (bool, usize) {
        let brackets = &lexed.brackets;
        let split = brackets.partition_point(|b| (b.line, b.col) < (line, cut));
        let mut stack: Vec<(Shape, usize)> = Vec::new();
        let mut base_line = line;
        for bracket in &brackets[..split] {
            match bracket.side {
                Side::Open => stack.push((bracket.shape, bracket.line)),
                Side::Close => {
                    if let Some(&(_, opened)) = stack.last().filter(|(s, _)| *s == bracket.shape) {
                        stack.pop();
                        if bracket.line == line && opened < line {
                            base_line = opened;
                        }
                    }
                }
            }
        }
        let opened_here = stack.last().is_some_and(|&(_, opened)| opened == line);
        if !opened_here {
            return (false, base_line);
        }
        let depth = stack.len();
        for bracket in &brackets[split..] {
            match bracket.side {
                Side::Open => stack.push((bracket.shape, bracket.line)),
                Side::Close => {
                    if stack.last().is_some_and(|(s, _)| *s == bracket.shape) {
                        stack.pop();
                    }
                }
            }
            if stack.len() < depth {
                return (true, base_line);
            }
        }
        (false, base_line)
    }

    /// `indent` minus one unit (as much of it as exists)
    fn dedent(&self, indent: &str) -> String {
        match self.unit {
            IndentUnit::Tab => indent.strip_prefix('\t').unwrap_or(indent).to_string(),
            IndentUnit::Spaces(_) => {
                let spaces = indent.chars().take_while(|ch| *ch == ' ').count();
                let keep = spaces.saturating_sub(self.unit.width());
                format!("{}{}", " ".repeat(keep), &indent[spaces..])
            }
        }
    }

    /// Start of the line `O` opens above `line`
    ///
    /// With [`CommentContinuation::Continue`] a comment line continues its
    /// comment; otherwise the new line takes what `o` would give below the
    /// previous line (the line's own indent for the first line and for
    /// languages without a rule).
    pub fn above(&self, line: usize, comments: CommentContinuation) -> NewLineStart {
        let own_indent = self.leading_whitespace(line);
        if comments == CommentContinuation::Continue
            && let Some(token) = self.language.leading_comment_token(&self.line_text(line))
        {
            return NewLineStart {
                indent: own_indent,
                comment_token: Some(token),
            };
        }
        match line.checked_sub(1) {
            Some(previous) if self.language.newline.is_some() => {
                let end = line_end_char_index(&self.doc.slice(..), previous);
                NewLineStart::plain(self.below(previous, end).indent)
            }
            _ => NewLineStart::plain(own_indent),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::indent::FOUR_SPACES;

    fn below(language: &str, text: &str, line: usize) -> String {
        let doc = Rope::from_str(text);
        let rules = LanguageRules::for_language(language).unwrap();
        let rule = NewlineIndent {
            doc: &doc,
            language: rules,
            unit: FOUR_SPACES,
        };
        let end = line_end_char_index(&doc.slice(..), line);
        rule.below(line, end).text()
    }

    #[test]
    fn brackets_inside_strings_and_comments_do_not_open() {
        assert_eq!(below("rs", "x(\"(\");\n", 0), "");
        assert_eq!(below("rs", "x = '(';\n", 0), "");
        assert_eq!(below("rs", "x(1); // f(\n", 0), "");
        assert_eq!(below("rs", "fn f<'a>(x: &'a u8) {\n}\n", 0), "    ");
    }

    #[test]
    fn dedent_keeps_what_is_left_of_a_short_indent() {
        let doc = Rope::from_str("");
        let rule = NewlineIndent {
            doc: &doc,
            language: &LanguageRules::UNLISTED,
            unit: FOUR_SPACES,
        };
        assert_eq!(rule.dedent("        "), "    ");
        assert_eq!(rule.dedent("  "), "");
        assert_eq!(rule.dedent(""), "");
    }

    fn above(language: &str, text: &str, line: usize) -> NewLineStart {
        let doc = Rope::from_str(text);
        let rule = NewlineIndent {
            doc: &doc,
            language: LanguageRules::for_language(language).unwrap(),
            unit: FOUR_SPACES,
        };
        rule.above(line, CommentContinuation::Continue)
    }

    #[test]
    fn above_uses_the_previous_line_or_continues_a_comment() {
        let text = "fn f() {\n    a;\n    // c\n}\n";
        assert_eq!(above("rs", text, 1).text(), "    ");
        assert_eq!(above("rs", text, 0).text(), "");
        assert_eq!(above("rs", text, 2).text(), "    // ");
        assert_eq!(above("rs", text, 3).text(), "    ");
        assert_eq!(above("md", "  a\n b\n", 1).text(), " ");
    }

    #[test]
    fn crlf_line_endings_do_not_change_the_rule() {
        assert_eq!(below("rs", "fn f() {\r\n}\r\n", 0), "    ");
        assert_eq!(below("rs", "// c\r\n", 0), "// ");
        assert_eq!(below("py", "if a:\r\n", 0), "    ");
        assert_eq!(below("rs", "\tx;\r\n", 0), "\t");
    }

    #[test]
    fn tabs_extend_by_the_unit_and_dedent_removes_one_tab() {
        let doc = Rope::from_str("");
        let rule = NewlineIndent {
            doc: &doc,
            language: &LanguageRules::UNLISTED,
            unit: IndentUnit::Tab,
        };
        assert_eq!(rule.dedent("\t\t"), "\t");
        assert_eq!(rule.dedent(""), "");
    }

    proptest::proptest! {
        #[test]
        fn rule_never_panics_on_arbitrary_text(
            text in "[ \\t\\r\\n(){}\\[\\]\"'a/#:\\\\]{0,60}",
            line in 0usize..8,
            language in proptest::sample::select(vec!["rs", "py", "md"]),
        ) {
            let doc = Rope::from_str(&text);
            let line = line.min(doc.len_lines() - 1);
            let rules = LanguageRules::for_language(language).unwrap();
            let rule = NewlineIndent { doc: &doc, language: rules, unit: FOUR_SPACES };
            let end = line_end_char_index(&doc.slice(..), line);
            let _ = rule.below(line, end);
            let _ = rule.below(line, doc.line_to_char(line));
            let _ = rule.above(line, CommentContinuation::Continue);
            if let Some(newline) = &rules.newline {
                let lexed = Lexed::new(&doc, rules, newline);
                let _ = NewlineIndent::bracket_state(&lexed, line, 0);
                let _ = NewlineIndent::bracket_state(&lexed, line, usize::MAX / 2);
                let _ = lexed.last_code_char(line, usize::MAX);
            }
        }
    }

    #[test]
    fn closing_line_is_relative_to_the_line_that_opened_the_bracket() {
        let text = "fn f() {\n    let a = foo(1,\n        2);\n}\n";
        assert_eq!(below("rs", text, 2), "    ");
    }
}
