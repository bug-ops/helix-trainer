//! Embedded quest assets compiled into the binary
//!
//! This module provides compile-time embedding of quest TOML files using
//! the `include_str!()` macro, similar to how sound assets are embedded
//! with `include_bytes!()`.
//!
//! # Benefits
//!
//! - **Standalone binary**: No external files needed at runtime
//! - **Consistent behavior**: Same data regardless of working directory
//! - **Fast loading**: No filesystem I/O required
//!
//! # Usage
//!
//! ```ignore
//! use helix_trainer::config::quests::embedded::{get_embedded_quests, available_embedded_locales};
//!
//! // Get quest content for English locale
//! if let Some(content) = get_embedded_quests("en") {
//!     // Parse TOML content
//! }
//!
//! // List available locales
//! for locale in available_embedded_locales() {
//!     println!("Available locale: {}", locale);
//! }
//! ```

// =============================================================================
// EMBEDDED QUEST FILES
// =============================================================================

/// English quest templates
const EN_DAILY: &str = include_str!("../../../quests/en/daily.toml");

// =============================================================================
// PUBLIC API
// =============================================================================

/// Get embedded quest content for a given locale
///
/// Returns the raw TOML content string for the daily quests file,
/// or `None` if the locale is not available.
///
/// # Arguments
///
/// * `locale` - Locale code (e.g., "en")
///
/// # Returns
///
/// The embedded TOML content as a static string, or `None` if locale not found.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::config::quests::embedded::get_embedded_quests;
///
/// if let Some(content) = get_embedded_quests("en") {
///     assert!(content.contains("[metadata]"));
/// }
///
/// assert!(get_embedded_quests("invalid").is_none());
/// ```
pub fn get_embedded_quests(locale: &str) -> Option<&'static str> {
    match locale {
        "en" => Some(EN_DAILY),
        _ => None,
    }
}

/// Get list of available embedded locales
///
/// Returns all locale codes that have embedded quest content.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::config::quests::embedded::available_embedded_locales;
///
/// let locales = available_embedded_locales();
/// assert!(locales.contains(&"en"));
/// ```
pub fn available_embedded_locales() -> &'static [&'static str] {
    &["en"]
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_embedded_quests_en() {
        let content = get_embedded_quests("en");
        assert!(content.is_some(), "English quests should be embedded");

        let toml_content = content.unwrap();
        assert!(
            toml_content.contains("[metadata]"),
            "Quest file should have metadata section"
        );
        assert!(
            toml_content.contains("[[quests]]"),
            "Quest file should have quest definitions"
        );
    }

    #[test]
    fn test_get_embedded_quests_unknown_locale() {
        assert!(get_embedded_quests("invalid").is_none());
        assert!(get_embedded_quests("").is_none());
        assert!(get_embedded_quests("de").is_none());
    }

    #[test]
    fn test_available_embedded_locales() {
        let locales = available_embedded_locales();
        assert!(
            !locales.is_empty(),
            "At least one locale should be embedded"
        );
        assert!(locales.contains(&"en"), "English should be available");
    }

    #[test]
    fn test_embedded_quests_are_valid_toml() {
        for locale in available_embedded_locales() {
            let content = get_embedded_quests(locale).expect("Locale should have content");

            // Verify it parses as valid TOML
            let parsed: Result<toml::Value, _> = toml::from_str(content);
            assert!(
                parsed.is_ok(),
                "Quest content for locale '{}' should be valid TOML: {:?}",
                locale,
                parsed.err()
            );
        }
    }

    #[test]
    fn test_embedded_quests_have_required_structure() {
        use crate::config::quests::QuestsFile;

        for locale in available_embedded_locales() {
            let content = get_embedded_quests(locale).expect("Locale should have content");

            // Verify it parses as a valid QuestsFile
            let parsed: Result<QuestsFile, _> = toml::from_str(content);
            assert!(
                parsed.is_ok(),
                "Quest content for locale '{}' should parse as QuestsFile: {:?}",
                locale,
                parsed.err()
            );

            let quests_file = parsed.unwrap();
            assert!(
                !quests_file.quests.is_empty(),
                "Quest file for locale '{}' should have at least one quest",
                locale
            );
        }
    }
}
