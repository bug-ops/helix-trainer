//! Per-language rules the simulator applies: indent unit, comment tokens and
//! the new-line indent rule.
//!
//! One table serves `>`, `<`, Insert-mode `Tab` (unit), `J` (comment tokens)
//! and `o`/`O`/Enter (new-line rule), so a language is declared exactly once.

use serde::{Deserialize, Serialize};

use super::indent::{FOUR_SPACES, IndentUnit, TWO_SPACES, TabWidth};

/// Content language of a scenario, a closed set parsed at scenario load
///
/// An unknown or empty `language` value in a scenario file is a load error.
/// Supporting a further language means adding a variant here together with
/// its [`LanguageRules`] entry.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::Language;
///
/// assert_eq!(Language::Py.rules().comment_tokens, ["#"]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// Rust, the default of scenarios that name no language
    #[serde(rename = "rs")]
    Rs,
    /// Markdown
    #[serde(rename = "md")]
    Md,
    /// Python
    #[serde(rename = "py")]
    Py,
}

impl Language {
    /// The editing rules of this language; total, every variant has an entry
    pub const fn rules(self) -> &'static LanguageRules {
        match self {
            Self::Rs => &LanguageRules::RUST,
            Self::Md => &LanguageRules::MARKDOWN,
            Self::Py => &LanguageRules::PYTHON,
        }
    }
}

/// Where Insert-mode `Tab` goes when text precedes the cursor on its line
///
/// `hx` moves to the end of the parent syntax node; the simulator has no
/// syntax tree, so a language either approximates that with brackets or stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartTab {
    /// The cursor does not move
    Stay,
    /// The cursor moves after the closer of the innermost enclosing bracket
    /// pair, or after the closing brace of the block the line opens
    BracketCloser,
}

/// Language-dependent editing rules mirroring the `hx` 25.07.1 `languages.toml`
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::Language;
///
/// let rust = Language::Rs.rules();
/// assert_eq!(rust.comment_tokens, ["///", "//!", "//"]);
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct LanguageRules {
    /// Indent unit used when the document text does not reveal one
    pub indent_unit: IndentUnit,
    /// Columns a tab character counts for `&`, `<`, `C` and Backspace
    pub tab_width: TabWidth,
    /// What `Tab` does after text on the line
    pub smart_tab: SmartTab,
    /// Line-comment tokens, longest first so `///` wins over `//`
    pub comment_tokens: &'static [&'static str],
    /// Opener and closer pairs typed text closes automatically and Enter
    /// between them splits onto three lines
    pub auto_pairs: &'static [(char, char)],
    /// How `o`/`O`/Enter derive the indent of the new line, or `None` to copy
    /// the current line's indent
    pub newline: Option<NewlineRules>,
}

/// Whether a string literal may span lines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringLines {
    /// An unterminated string ends with its line
    Single,
    /// A string runs until its closing quote, across line breaks
    Multi,
}

/// A string literal form beyond the plain quoted one
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringForm {
    /// `r"..."` and `r#"..."#`: no escapes, closed by the quote and the same hashes
    Raw,
    /// `"""..."""` and `'''...'''`: may span lines
    TripleQuoted,
}

/// Language-specific inputs of the new-line indent rule
///
/// Brackets `( ) [ ] { }` open an indented block in every language that has
/// these rules; the fields carry what differs between languages.
#[derive(Debug, PartialEq, Eq)]
pub struct NewlineRules {
    /// A line whose code ends in one of these characters opens a block, when
    /// the line starts with one of `block_keywords`
    pub trailing_openers: &'static [char],
    /// A line starting with one of these words dedents the next line by one unit
    pub dedent_words: &'static [&'static str],
    /// Characters that delimit a single-line string literal
    pub string_quotes: &'static [char],
    /// Whether `'x'` is a character literal and a lone `'` a lifetime
    pub char_literals: bool,
    /// Whether an unterminated string literal runs on across line breaks
    pub string_lines: StringLines,
    /// String forms beyond the plain quoted one
    pub string_forms: &'static [StringForm],
    /// Opening and closing token of block comments, which nest
    pub block_comment: Option<(&'static str, &'static str)>,
    /// Words that, as the first word of a line, let a trailing opener
    /// character open a block (at bracket depth 0); empty for languages
    /// whose openers are not tied to a statement
    pub block_keywords: &'static [&'static str],
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
    /// Rules of a simulator created without a language: the default unit, no
    /// comment tokens and no new-line rule; no scenario can select them
    pub const PLAIN: Self = Self {
        indent_unit: IndentUnit::DEFAULT,
        tab_width: TabWidth::FOUR,
        smart_tab: SmartTab::Stay,
        comment_tokens: &[],
        auto_pairs: helix_core::auto_pairs::DEFAULT_PAIRS,
        newline: None,
    };

    pub(crate) const RUST: Self = Self {
        indent_unit: FOUR_SPACES,
        tab_width: TabWidth::FOUR,
        smart_tab: SmartTab::BracketCloser,
        comment_tokens: &["///", "//!", "//"],
        auto_pairs: DEFAULT_PAIRS,
        newline: Some(NewlineRules {
            trailing_openers: &[],
            dedent_words: &[],
            string_quotes: &['"'],
            char_literals: true,
            string_lines: StringLines::Multi,
            string_forms: &[StringForm::Raw],
            block_comment: Some(("/*", "*/")),
            block_keywords: &[],
        }),
    };

    pub(crate) const MARKDOWN: Self = Self {
        indent_unit: TWO_SPACES,
        tab_width: TabWidth::TWO,
        smart_tab: SmartTab::Stay,
        comment_tokens: &[],
        auto_pairs: PAIRS_WITH_SINGLE_QUOTE,
        newline: None,
    };

    pub(crate) const PYTHON: Self = Self {
        indent_unit: FOUR_SPACES,
        tab_width: TabWidth::FOUR,
        smart_tab: SmartTab::Stay,
        comment_tokens: &["#"],
        auto_pairs: PAIRS_WITH_SINGLE_QUOTE,
        newline: Some(NewlineRules {
            trailing_openers: &[':'],
            dedent_words: &["return", "pass", "break", "continue", "raise"],
            string_quotes: &['"', '\''],
            char_literals: false,
            string_lines: StringLines::Single,
            string_forms: &[StringForm::TripleQuoted],
            block_comment: None,
            block_keywords: &[
                "if", "elif", "else", "for", "while", "def", "class", "try", "except", "finally",
                "with", "match", "case",
            ],
        }),
    };

    /// The comment token `text` starts with after leading blanks (the
    /// longest one), if any
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::Language;
    ///
    /// let rust = Language::Rs.rules();
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
    /// use helix_trainer::helix::Language;
    ///
    /// let rust = Language::Rs.rules();
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

    const ALL: [Language; 3] = [Language::Rs, Language::Md, Language::Py];

    #[test]
    fn table_entries_match_hx() {
        assert_eq!(Language::Rs.rules().indent_unit, FOUR_SPACES);
        assert_eq!(Language::Md.rules().indent_unit, TWO_SPACES);
        assert_eq!(Language::Py.rules().indent_unit, FOUR_SPACES);
        assert_eq!(Language::Rs.rules().tab_width, TabWidth::FOUR);
        assert_eq!(Language::Md.rules().tab_width, TabWidth::TWO);
        assert_eq!(Language::Py.rules().tab_width, TabWidth::FOUR);
        assert_eq!(LanguageRules::PLAIN.tab_width, TabWidth::FOUR);
    }

    #[test]
    fn comment_tokens_are_sorted_longest_first() {
        for language in ALL {
            let tokens = language.rules().comment_tokens;
            assert!(tokens.is_sorted_by_key(|token| std::cmp::Reverse(token.len())));
        }
    }

    #[test]
    fn leading_comment_token_picks_the_longest_after_blanks() {
        let rust = Language::Rs.rules();
        assert_eq!(rust.leading_comment_token("\t//! a"), Some("//!"));
        assert_eq!(rust.leading_comment_token("//// a"), Some("///"));
        assert_eq!(LanguageRules::PLAIN.leading_comment_token("// a"), None);
    }

    #[derive(Debug, Deserialize)]
    struct Holder {
        language: Language,
    }

    #[test]
    fn deserializes_exactly_the_extension_tokens() {
        for (token, expected) in [
            ("rs", Language::Rs),
            ("md", Language::Md),
            ("py", Language::Py),
        ] {
            let holder: Holder = toml::from_str(&format!("language = \"{token}\"")).unwrap();
            assert_eq!(holder.language, expected);
        }
    }

    #[test]
    fn rejects_unknown_empty_and_miscased_tokens() {
        for token in ["", "js", "RS", "Rs", "rust", "rs drop", "c++"] {
            assert!(
                toml::from_str::<Holder>(&format!("language = \"{token}\"")).is_err(),
                "{token:?}"
            );
        }
    }
}
