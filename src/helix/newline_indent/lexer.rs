//! One pass over a document that tells code from strings and comments.
//!
//! The state is carried from the start of the document, so a bracket inside a
//! block comment, a raw string, a triple-quoted string or any string that
//! spans lines is not counted, wherever the line asking for an indent is.

use super::{Bracket, classify};
use crate::helix::language::{LanguageRules, NewlineRules, StringForm, StringLines};
use helix_core::Rope;

/// What a document character is part of
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// Code, including the whitespace between tokens
    Code,
    /// A string or character literal, delimiters included
    Str,
    /// A line or block comment, delimiters included
    Comment,
}

/// A document split into code, strings and comments
pub(super) struct Lexed {
    /// Brackets that are code, in document order
    pub(super) brackets: Vec<Bracket>,
    chars: Vec<char>,
    kinds: Vec<Kind>,
    line_starts: Vec<usize>,
}

impl Lexed {
    /// Lex `doc` with the comment tokens of `language` and the literal syntax of `rules`
    pub(super) fn new(doc: &Rope, language: &LanguageRules, rules: &NewlineRules) -> Self {
        let chars: Vec<char> = doc.chars().collect();
        let line_starts: Vec<usize> = (0..doc.len_lines())
            .map(|line| doc.line_to_char(line))
            .collect();
        let mut lexer = Lexer {
            chars: &chars,
            language,
            rules,
            kinds: vec![Kind::Code; chars.len()],
            brackets: Vec::new(),
        };
        let mut at = 0;
        while at < chars.len() {
            at = lexer.step(at);
        }
        let Lexer {
            kinds,
            brackets: found,
            ..
        } = lexer;
        let brackets = found
            .into_iter()
            .map(|(index, shape, side)| {
                let line = line_starts.partition_point(|&start| start <= index) - 1;
                Bracket {
                    line,
                    col: index - line_starts[line],
                    shape,
                    side,
                }
            })
            .collect();
        Self {
            brackets,
            chars,
            kinds,
            line_starts,
        }
    }

    /// Char range of `line`, line break included
    fn line_range(&self, line: usize) -> std::ops::Range<usize> {
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.chars.len());
        start..end
    }

    /// The last character before column `cut` of `line` that is neither
    /// blank nor inside a comment, with its column and kind
    pub(super) fn last_code_char(&self, line: usize, cut: usize) -> Option<(usize, char, Kind)> {
        let range = self.line_range(line);
        let end = range.start.saturating_add(cut).min(range.end);
        (range.start..end).rev().find_map(|index| {
            let ch = self.chars[index];
            (!ch.is_whitespace() && self.kinds[index] != Kind::Comment).then_some((
                index - range.start,
                ch,
                self.kinds[index],
            ))
        })
    }

    /// Whether any comment text sits on `line`
    pub(super) fn has_comment(&self, line: usize) -> bool {
        self.kinds[self.line_range(line)].contains(&Kind::Comment)
    }
}

struct Lexer<'a> {
    chars: &'a [char],
    language: &'a LanguageRules,
    rules: &'a NewlineRules,
    kinds: Vec<Kind>,
    brackets: Vec<(usize, super::Shape, super::Side)>,
}

impl Lexer<'_> {
    /// Classify what starts at `at`; returns the index to continue from
    fn step(&mut self, at: usize) -> usize {
        let rest = &self.chars[at..];
        if self
            .language
            .comment_token_at(&rest.iter().copied())
            .is_some()
        {
            return self.mark(at, self.line_end(at), Kind::Comment);
        }
        if let Some((open, close)) = self.rules.block_comment
            && starts_with(rest, open)
        {
            return self.mark(at, self.block_comment_end(at, open, close), Kind::Comment);
        }
        if self.rules.string_forms.contains(&StringForm::Raw)
            && let Some(end) = self.raw_string_end(at)
        {
            return self.mark(at, end, Kind::Str);
        }
        if self.rules.string_forms.contains(&StringForm::TripleQuoted)
            && let Some(end) = self.triple_quoted_end(at)
        {
            return self.mark(at, end, Kind::Str);
        }
        let ch = self.chars[at];
        if self.rules.string_quotes.contains(&ch) {
            return self.mark(at, self.string_end(at), Kind::Str);
        }
        if self.rules.char_literals && ch == '\'' {
            let len = self.char_literal_len(at).unwrap_or(1);
            return self.mark(at, at + len, Kind::Str);
        }
        if let Some((shape, side)) = classify(ch) {
            self.brackets.push((at, shape, side));
        }
        at + 1
    }

    fn mark(&mut self, start: usize, end: usize, kind: Kind) -> usize {
        let end = end.min(self.chars.len());
        self.kinds[start..end].fill(kind);
        end
    }

    /// Index of the line break ending the line that holds `at`, or the document end
    fn line_end(&self, at: usize) -> usize {
        self.chars[at..]
            .iter()
            .position(|&ch| ch == '\n')
            .map_or(self.chars.len(), |offset| at + offset)
    }

    /// Index just past the string literal that starts at `start`
    ///
    /// With [`StringLines::Single`] an unterminated string ends at its line.
    fn string_end(&self, start: usize) -> usize {
        let quote = self.chars[start];
        let mut at = start + 1;
        while at < self.chars.len() {
            match self.chars[at] {
                '\\' => at += 2,
                ch if ch == quote => return at + 1,
                '\n' if self.rules.string_lines == StringLines::Single => return at,
                _ => at += 1,
            }
        }
        self.chars.len()
    }

    /// Index just past the triple-quoted string that starts at `start`, if one does
    fn triple_quoted_end(&self, start: usize) -> Option<usize> {
        let quote = self.chars[start];
        let triple = [quote; 3];
        if !matches!(quote, '"' | '\'') || !self.chars[start..].starts_with(&triple) {
            return None;
        }
        let mut at = start + 3;
        while at < self.chars.len() {
            if self.chars[at] == '\\' {
                at += 2;
            } else if self.chars[at..].starts_with(&triple) {
                return Some(at + 3);
            } else {
                at += 1;
            }
        }
        Some(self.chars.len())
    }

    /// Index just past the raw string (`r"..."`, `r#"..."#`, `br"..."`) that starts at `start`
    fn raw_string_end(&self, start: usize) -> Option<usize> {
        if self.chars[start] != 'r' {
            return None;
        }
        let prefix_start = match start.checked_sub(1) {
            Some(previous) if self.chars[previous] == 'b' => previous,
            _ => start,
        };
        let continues_identifier = prefix_start
            .checked_sub(1)
            .is_some_and(|at| self.chars[at].is_alphanumeric() || self.chars[at] == '_');
        if continues_identifier {
            return None;
        }
        let hashes = self.chars[start + 1..]
            .iter()
            .take_while(|&&ch| ch == '#')
            .count();
        if self.chars.get(start + 1 + hashes) != Some(&'"') {
            return None;
        }
        let mut closer = vec!['"'];
        closer.extend(std::iter::repeat_n('#', hashes));
        let body = start + hashes + 2;
        let end = (body..self.chars.len())
            .find(|&at| self.chars[at..].starts_with(&closer))
            .map_or(self.chars.len(), |at| at + closer.len());
        Some(end)
    }

    /// Index just past the (nested) block comment that starts at `start`
    fn block_comment_end(&self, start: usize, open: &str, close: &str) -> usize {
        let mut depth = 0usize;
        let mut at = start;
        while at < self.chars.len() {
            let rest = &self.chars[at..];
            if starts_with(rest, open) {
                depth += 1;
                at += open.chars().count();
            } else if starts_with(rest, close) {
                depth -= 1;
                at += close.chars().count();
                if depth == 0 {
                    return at;
                }
            } else {
                at += 1;
            }
        }
        self.chars.len()
    }

    /// Length of the Rust character literal at `start`, if one starts there (a
    /// lone `'` is a lifetime)
    fn char_literal_len(&self, start: usize) -> Option<usize> {
        let at = |offset: usize| self.chars.get(start + offset).copied();
        match (at(1), at(2), at(3)) {
            (Some('\\'), _, Some('\'')) => Some(4),
            (Some(ch), Some('\''), _) if ch != '\'' => Some(3),
            _ => None,
        }
    }
}

fn starts_with(rest: &[char], text: &str) -> bool {
    let mut rest = rest.iter();
    text.chars().all(|expected| rest.next() == Some(&expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::Language;

    fn lex(language: Language, text: &str) -> Lexed {
        let language = language.rules();
        let rules = language.newline.as_ref().unwrap();
        Lexed::new(&Rope::from_str(text), language, rules)
    }

    fn bracket_cols(lexed: &Lexed) -> Vec<(usize, usize)> {
        lexed.brackets.iter().map(|b| (b.line, b.col)).collect()
    }

    #[test]
    fn block_comments_nest_and_hide_brackets() {
        let lexed = lex(Language::Rs, "a(/* ( /* ) */ ) */ b)\n");
        assert_eq!(bracket_cols(&lexed), vec![(0, 1), (0, 21)]);
    }

    #[test]
    fn raw_strings_end_at_the_matching_hashes() {
        let lexed = lex(Language::Rs, "r##\"(\"#)\"## ( bar\"(\"\n");
        assert_eq!(bracket_cols(&lexed), vec![(0, 12)]);
    }

    #[test]
    fn rust_strings_span_lines_but_python_strings_do_not() {
        assert_eq!(
            bracket_cols(&lex(Language::Rs, "\"a\n(\" (\n")),
            vec![(1, 3)]
        );
        assert_eq!(
            bracket_cols(&lex(Language::Py, "\"a\n( (\n")),
            vec![(1, 0), (1, 2)]
        );
    }

    #[test]
    fn python_triple_quotes_hide_brackets_across_lines() {
        let lexed = lex(Language::Py, "x = (\"\"\"\n)\n\"\"\")\n");
        assert_eq!(bracket_cols(&lexed), vec![(0, 4), (2, 3)]);
    }

    #[test]
    fn last_code_char_skips_comments_and_reports_the_kind() {
        let lexed = lex(Language::Rs, "a {  // c\n\"{\" /* x */\n");
        assert_eq!(lexed.last_code_char(0, 99), Some((2, '{', Kind::Code)));
        assert_eq!(lexed.last_code_char(1, 99), Some((2, '"', Kind::Str)));
        assert!(lexed.has_comment(0));
    }
}
