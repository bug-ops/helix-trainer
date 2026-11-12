//! Indent unit used by `>`, `<`, Insert-mode `Tab` and new-line indentation.

use helix_core::Rope;
use helix_core::indent::{IndentStyle, auto_detect_indent_style};
use std::num::NonZeroU8;

/// One level of indentation: a run of spaces or a single tab character
///
/// Mirrors the `indent.unit` of a language in the `hx` 25.07.1
/// `languages.toml`.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::Language;
///
/// assert_eq!(Language::Rs.rules().indent_unit.to_string(), "    ");
/// assert_eq!(Language::Md.rules().indent_unit.to_string(), "  ");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentUnit {
    /// This many space characters
    Spaces(NonZeroU8),
    /// One tab character
    Tab,
}

/// Tab stop width of a language: how many columns `hx` counts for a tab
/// character
///
/// Mirrors the `indent.tab-width` of a language in the `hx` 25.07.1
/// `languages.toml`; zero is unrepresentable.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::Language;
///
/// let md = Language::Md.rules().tab_width;
/// assert_eq!(md.get(), 2);
/// assert_eq!(md.next_stop(1), 2);
/// assert_eq!(Language::Rs.rules().tab_width.next_stop(4), 8);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabWidth(NonZeroU8);

impl TabWidth {
    /// Four columns, the `hx` default
    pub const FOUR: Self = Self(NonZeroU8::new(4).unwrap());
    /// Two columns
    pub const TWO: Self = Self(NonZeroU8::new(2).unwrap());

    /// The width in columns
    pub fn get(self) -> usize {
        usize::from(self.0.get())
    }

    /// The column a tab character at visual column `col` advances to
    pub fn next_stop(self, col: usize) -> usize {
        (col / self.get() + 1) * self.get()
    }
}

pub(super) const FOUR_SPACES: IndentUnit = IndentUnit::Spaces(NonZeroU8::new(4).unwrap());
pub(super) const TWO_SPACES: IndentUnit = IndentUnit::Spaces(NonZeroU8::new(2).unwrap());

impl IndentUnit {
    /// Unit of a simulator created without a language, matching the default
    /// language `rs`
    pub const DEFAULT: Self = FOUR_SPACES;

    /// The unit `hx` auto-detects from consistently indented `text`, or
    /// `None` when the text reveals none
    ///
    /// `hx` prefers this over the language default, so a document indented
    /// with 8 spaces indents by 8.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_core::Rope;
    /// use helix_trainer::helix::indent::{IndentUnit, TabWidth};
    ///
    /// let text = Rope::from_str("a\n        b\n                c\n        d\n");
    /// assert_eq!(IndentUnit::detect(&text).unwrap().width(TabWidth::FOUR), 8);
    /// assert_eq!(IndentUnit::detect(&Rope::from_str("a\nb\n")), None);
    /// ```
    pub fn detect(text: &Rope) -> Option<Self> {
        match auto_detect_indent_style(text)? {
            IndentStyle::Tabs => Some(Self::Tab),
            IndentStyle::Spaces(width) => NonZeroU8::new(width).map(Self::Spaces),
        }
    }

    /// Width of one unit in columns, a tab counting `tab_width`
    pub fn width(self, tab_width: TabWidth) -> usize {
        match self {
            Self::Spaces(width) => usize::from(width.get()),
            Self::Tab => tab_width.get(),
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
    fn unit_text_and_width() {
        assert_eq!(FOUR_SPACES.to_string(), "    ");
        assert_eq!(FOUR_SPACES.width(TabWidth::TWO), 4);
        assert_eq!(IndentUnit::Tab.to_string(), "\t");
        assert_eq!(IndentUnit::Tab.width(TabWidth::TWO), 2);
        assert_eq!(TabWidth::FOUR.next_stop(0), 4);
        assert_eq!(TabWidth::FOUR.next_stop(3), 4);
        assert_eq!(TabWidth::FOUR.next_stop(4), 8);
    }
}
