//! Per-language rules the simulator applies: indent unit, comment tokens and
//! the new-line indent rule.
//!
//! One table serves `>`, `<`, Insert-mode `Tab` (unit), `J` (comment tokens)
//! and `o`/`O`/Enter (new-line rule), so a language is declared exactly once.

use super::indent::{FOUR_SPACES, IndentUnit, TWO_SPACES};

/// Language-dependent editing rules mirroring the `hx` 25.07.1 `languages.toml`
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::LanguageRules;
///
/// let rust = LanguageRules::for_language("rs").unwrap();
/// assert_eq!(rust.comment_tokens, ["///", "//!", "//"]);
/// assert!(LanguageRules::for_language("unknown").is_none());
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct LanguageRules {
    /// Indent unit used when the document text does not reveal one
    pub indent_unit: IndentUnit,
    /// Line-comment tokens, longest first so `///` wins over `//`
    pub comment_tokens: &'static [&'static str],
    /// Opener and closer pairs that Enter between them splits onto three lines
    pub auto_pairs: &'static [(char, char)],
    /// How `o`/`O`/Enter derive the indent of the new line, or `None` to copy
    /// the current line's indent
    pub newline: Option<NewlineRules>,
}

/// Language-specific inputs of the new-line indent rule
///
/// Brackets `( ) [ ] { }` open an indented block in every language that has
/// these rules; the fields carry what differs between languages.
#[derive(Debug, PartialEq, Eq)]
pub struct NewlineRules {
    /// A line whose code ends in one of these characters opens a block
    pub trailing_openers: &'static [char],
    /// A line starting with one of these words dedents the next line by one unit
    pub dedent_words: &'static [&'static str],
    /// Characters that delimit a single-line string literal
    pub string_quotes: &'static [char],
    /// Whether `'x'` is a character literal and a lone `'` a lifetime
    pub char_literals: bool,
}

const DEFAULT_PAIRS: &[(char, char)] =
    &[('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('`', '`')];

const PAIRS_WITH_SINGLE_QUOTE: &[(char, char)] = &[
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('"', '"'),
    ('\'', '\''),
    ('`', '`'),
];

impl LanguageRules {
    /// Rules for scenarios naming no language or one outside the table: the
    /// default unit, no comment tokens and no new-line rule
    pub const UNLISTED: Self = Self {
        indent_unit: IndentUnit::DEFAULT,
        comment_tokens: &[],
        auto_pairs: DEFAULT_PAIRS,
        newline: None,
    };

    const RUST: Self = Self {
        indent_unit: FOUR_SPACES,
        comment_tokens: &["///", "//!", "//"],
        auto_pairs: DEFAULT_PAIRS,
        newline: Some(NewlineRules {
            trailing_openers: &[],
            dedent_words: &[],
            string_quotes: &['"'],
            char_literals: true,
        }),
    };

    const MARKDOWN: Self = Self {
        indent_unit: TWO_SPACES,
        comment_tokens: &[],
        auto_pairs: PAIRS_WITH_SINGLE_QUOTE,
        newline: None,
    };

    const PYTHON: Self = Self {
        indent_unit: FOUR_SPACES,
        comment_tokens: &["#"],
        auto_pairs: PAIRS_WITH_SINGLE_QUOTE,
        newline: Some(NewlineRules {
            trailing_openers: &[':'],
            dedent_words: &["return", "pass", "break", "continue", "raise"],
            string_quotes: &['"', '\''],
            char_literals: false,
        }),
    };

    /// Rules for `language` (a file-extension token), or `None` when it has
    /// no table entry
    ///
    /// A unit test over the embedded scenarios fails when a scenario uses a
    /// language that is missing here.
    pub fn for_language(language: &str) -> Option<&'static Self> {
        match language {
            "rs" => Some(&Self::RUST),
            "md" => Some(&Self::MARKDOWN),
            "py" => Some(&Self::PYTHON),
            _ => None,
        }
    }

    /// The comment token `text` starts with after leading blanks (the
    /// longest one), if any
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::LanguageRules;
    ///
    /// let rust = LanguageRules::for_language("rs").unwrap();
    /// assert_eq!(rust.leading_comment_token("  /// doc"), Some("///"));
    /// assert_eq!(rust.leading_comment_token("a // b"), None);
    /// ```
    pub fn leading_comment_token(&self, text: &str) -> Option<&'static str> {
        self.comment_token_at(&text.trim_start_matches([' ', '\t']).chars())
    }

    /// The comment token (the longest one) that `rest` starts with, if any
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::LanguageRules;
    ///
    /// let rust = LanguageRules::for_language("rs").unwrap();
    /// assert_eq!(rust.comment_token_at(&"//! a".chars()), Some("//!"));
    /// assert_eq!(rust.comment_token_at(&"a //".chars()), None);
    /// ```
    pub fn comment_token_at(
        &self,
        rest: &(impl Iterator<Item = char> + Clone),
    ) -> Option<&'static str> {
        self.comment_tokens
            .iter()
            .find(|token| {
                let mut rest = rest.clone();
                token.chars().all(|expected| rest.next() == Some(expected))
            })
            .copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_entries_match_hx() {
        assert_eq!(
            LanguageRules::for_language("rs").unwrap().indent_unit,
            FOUR_SPACES
        );
        assert_eq!(
            LanguageRules::for_language("md").unwrap().indent_unit,
            TWO_SPACES
        );
        assert_eq!(
            LanguageRules::for_language("py").unwrap().indent_unit,
            FOUR_SPACES
        );
        assert!(LanguageRules::for_language("js").is_none());
        assert!(LanguageRules::for_language("").is_none());
    }

    #[test]
    fn comment_tokens_are_sorted_longest_first() {
        for language in ["rs", "md", "py"] {
            let tokens = LanguageRules::for_language(language)
                .unwrap()
                .comment_tokens;
            assert!(tokens.is_sorted_by_key(|token| std::cmp::Reverse(token.len())));
        }
    }

    #[test]
    fn leading_comment_token_picks_the_longest_after_blanks() {
        let rust = LanguageRules::for_language("rs").unwrap();
        assert_eq!(rust.leading_comment_token("\t//! a"), Some("//!"));
        assert_eq!(rust.leading_comment_token("//// a"), Some("///"));
        assert_eq!(LanguageRules::UNLISTED.leading_comment_token("// a"), None);
    }
}
