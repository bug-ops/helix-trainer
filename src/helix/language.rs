//! Per-language rules the simulator applies: indent unit, comment tokens and
//! the lines that open an indented block.
//!
//! One table serves `>`, `<`, Insert-mode `Tab` (unit), `J` (comment tokens)
//! and `o`/`O`/Enter (block openers), so a language is declared exactly once.

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
    /// A line ending in one of these characters (ignoring trailing blanks)
    /// opens a block, so a new line below it gains one indent unit
    pub block_openers: &'static [char],
}

impl LanguageRules {
    /// Rules for scenarios naming no language or one outside the table: the
    /// default unit, no comment tokens and no block openers
    pub const UNLISTED: Self = Self {
        indent_unit: IndentUnit::DEFAULT,
        comment_tokens: &[],
        block_openers: &[],
    };

    const RUST: Self = Self {
        indent_unit: FOUR_SPACES,
        comment_tokens: &["///", "//!", "//"],
        block_openers: &['{', '(', '['],
    };

    const MARKDOWN: Self = Self {
        indent_unit: TWO_SPACES,
        comment_tokens: &[],
        block_openers: &[],
    };

    const PYTHON: Self = Self {
        indent_unit: FOUR_SPACES,
        comment_tokens: &["#"],
        block_openers: &[':'],
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

    /// Whether `line` ends in a block-opening character of this language
    pub fn opens_block(&self, line: &str) -> bool {
        line.trim_end_matches([' ', '\t', '\r', '\n'])
            .chars()
            .next_back()
            .is_some_and(|last| self.block_openers.contains(&last))
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
    fn opens_block_ignores_trailing_blanks() {
        let rust = LanguageRules::for_language("rs").unwrap();
        assert!(rust.opens_block("fn f() {  \t"));
        assert!(rust.opens_block("let a = ["));
        assert!(!rust.opens_block("let a = 1;"));
        assert!(!rust.opens_block(""));
        assert!(
            LanguageRules::for_language("py")
                .unwrap()
                .opens_block("if x:\n")
        );
        assert!(!LanguageRules::UNLISTED.opens_block("{"));
    }
}
