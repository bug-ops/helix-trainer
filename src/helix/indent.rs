//! Per-language indent unit used by `>`, `<` and Insert-mode `Tab`.

use std::num::NonZeroU8;

/// Tab stop width `hx` uses for tab characters when the unit is a tab.
const TAB_STOP_WIDTH: usize = 4;

/// One level of indentation: a run of spaces or a single tab character
///
/// Mirrors the `indent.unit` of a language in the `hx` 25.07.1
/// `languages.toml`.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::IndentUnit;
///
/// assert_eq!(IndentUnit::for_language("rs").unwrap().to_string(), "    ");
/// assert_eq!(IndentUnit::for_language("md").unwrap().to_string(), "  ");
/// assert_eq!(IndentUnit::for_language("unknown"), None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentUnit {
    /// This many space characters
    Spaces(NonZeroU8),
    /// One tab character
    Tab,
}

const FOUR_SPACES: IndentUnit = IndentUnit::Spaces(NonZeroU8::new(4).unwrap());
const TWO_SPACES: IndentUnit = IndentUnit::Spaces(NonZeroU8::new(2).unwrap());

/// Languages used by bundled scenarios, with the unit `hx` 25.07.1 applies.
///
/// A unit test over the embedded scenarios fails when a scenario uses a
/// language that is missing here.
const LANGUAGE_UNITS: [(&str, IndentUnit); 3] =
    [("rs", FOUR_SPACES), ("md", TWO_SPACES), ("py", FOUR_SPACES)];

impl IndentUnit {
    /// Unit for scenarios that name no language or one outside the table
    ///
    /// ASSUMPTION: matches the default language `rs`; only user-supplied
    /// scenarios can reach this for an unlisted language.
    pub const DEFAULT: Self = FOUR_SPACES;

    /// The unit `hx` 25.07.1 uses for `language` (a file-extension token), or
    /// `None` when the language has no table entry
    pub fn for_language(language: &str) -> Option<Self> {
        LANGUAGE_UNITS
            .iter()
            .find_map(|&(name, unit)| (name == language).then_some(unit))
    }

    /// Width of one unit in columns, as `<` measures leading whitespace
    pub fn width(self) -> usize {
        match self {
            Self::Spaces(width) => usize::from(width.get()),
            Self::Tab => TAB_STOP_WIDTH,
        }
    }
}

impl Default for IndentUnit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl std::fmt::Display for IndentUnit {
    /// The text one unit inserts
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spaces(width) => write!(f, "{:width$}", "", width = usize::from(width.get())),
            Self::Tab => f.write_str("\t"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_entries_match_hx() {
        assert_eq!(IndentUnit::for_language("rs"), Some(FOUR_SPACES));
        assert_eq!(IndentUnit::for_language("md"), Some(TWO_SPACES));
        assert_eq!(IndentUnit::for_language("py"), Some(FOUR_SPACES));
    }

    #[test]
    fn unit_text_and_width() {
        assert_eq!(FOUR_SPACES.to_string(), "    ");
        assert_eq!(FOUR_SPACES.width(), 4);
        assert_eq!(IndentUnit::Tab.to_string(), "\t");
        assert_eq!(IndentUnit::Tab.width(), 4);
    }

    #[test]
    fn unlisted_language_has_no_entry() {
        assert_eq!(IndentUnit::for_language("js"), None);
        assert_eq!(IndentUnit::for_language(""), None);
    }
}
