//! Embedded scenario TOML files
//!
//! This module embeds all scenario TOML files at compile time using `include_str!()`.
//! This ensures the binary is fully standalone and works from any directory.

// ============================================================================
// Basic scenarios
// ============================================================================

const BASIC_DELETE: &str = include_str!("../../../scenarios/en/basic/delete.toml");
const BASIC_INSERT: &str = include_str!("../../../scenarios/en/basic/insert.toml");
const BASIC_REPLACE: &str = include_str!("../../../scenarios/en/basic/replace.toml");

// ============================================================================
// Clipboard scenarios
// ============================================================================

const CLIPBOARD_UNDO_REDO: &str = include_str!("../../../scenarios/en/clipboard/undo-redo.toml");
const CLIPBOARD_YANK_PASTE: &str = include_str!("../../../scenarios/en/clipboard/yank-paste.toml");

// ============================================================================
// Editing scenarios
// ============================================================================

const EDITING_ADVANCED: &str = include_str!("../../../scenarios/en/editing/advanced-editing.toml");
const EDITING_DELETE_SELECTION: &str =
    include_str!("../../../scenarios/en/editing/delete-selection.toml");
const EDITING_INDENTATION: &str = include_str!("../../../scenarios/en/editing/indentation.toml");
const EDITING_INDENTATION_PYTHON: &str =
    include_str!("../../../scenarios/en/editing/indentation-python.toml");
const EDITING_JOIN: &str = include_str!("../../../scenarios/en/editing/join.toml");
const EDITING_SURROUND: &str = include_str!("../../../scenarios/en/editing/surround.toml");

// ============================================================================
// Movement scenarios
// ============================================================================

const MOVEMENT_BASIC: &str = include_str!("../../../scenarios/en/movement/basic-movement.toml");
const MOVEMENT_COMBINED: &str = include_str!("../../../scenarios/en/movement/combined.toml");
const MOVEMENT_COMMAND_LINE_GOTO: &str =
    include_str!("../../../scenarios/en/movement/command-line-goto.toml");
const MOVEMENT_DOCUMENT: &str = include_str!("../../../scenarios/en/movement/document.toml");
const MOVEMENT_FIND_TILL: &str = include_str!("../../../scenarios/en/movement/find-till.toml");
const MOVEMENT_GOTO_COMMANDS: &str =
    include_str!("../../../scenarios/en/movement/goto-commands.toml");
const MOVEMENT_LINE_NAVIGATION: &str =
    include_str!("../../../scenarios/en/movement/line-navigation.toml");
const MOVEMENT_LINE: &str = include_str!("../../../scenarios/en/movement/line.toml");
const MOVEMENT_MATCH_BRACKETS: &str =
    include_str!("../../../scenarios/en/movement/match-brackets.toml");
const MOVEMENT_PARAGRAPH: &str = include_str!("../../../scenarios/en/movement/paragraph.toml");
const MOVEMENT_PRECISION: &str = include_str!("../../../scenarios/en/movement/precision.toml");
const MOVEMENT_SCROLL: &str = include_str!("../../../scenarios/en/movement/scroll.toml");
const MOVEMENT_WORD_BASICS: &str = include_str!("../../../scenarios/en/movement/word-basics.toml");
const MOVEMENT_WORD: &str = include_str!("../../../scenarios/en/movement/word.toml");
const MOVEMENT_WORD_PYTHON: &str = include_str!("../../../scenarios/en/movement/word-python.toml");

// ============================================================================
// Macro scenarios
// ============================================================================

const MACROS_BASIC: &str = include_str!("../../../scenarios/en/macros/basic-macros.toml");

// ============================================================================
// Register scenarios
// ============================================================================

const REGISTERS_NAMED: &str = include_str!("../../../scenarios/en/registers/named-registers.toml");

// ============================================================================
// Repeat scenarios
// ============================================================================

const REPEAT_BASIC: &str = include_str!("../../../scenarios/en/repeat/basic-repeat.toml");

// ============================================================================
// Search scenarios
// ============================================================================

const SEARCH_BASIC: &str = include_str!("../../../scenarios/en/search/basic-search.toml");

// ============================================================================
// Selection scenarios
// ============================================================================

const SELECTION_ADVANCED: &str =
    include_str!("../../../scenarios/en/selection/advanced-selection.toml");
const SELECTION_LINE: &str = include_str!("../../../scenarios/en/selection/line-selection.toml");
const SELECTION_ALL_REPLACE: &str =
    include_str!("../../../scenarios/en/selection/select-all-replace.toml");
const SELECTION_TEXT_OBJECTS: &str =
    include_str!("../../../scenarios/en/selection/text-objects.toml");

// ============================================================================
// Writing scenarios
// ============================================================================

const WRITING_EMPHASIS: &str = include_str!("../../../scenarios/en/writing/emphasis.toml");
const WRITING_LINKS: &str = include_str!("../../../scenarios/en/writing/links.toml");
const WRITING_LIST_INDENT: &str = include_str!("../../../scenarios/en/writing/list-indent.toml");
const WRITING_HEADING_LEVEL: &str =
    include_str!("../../../scenarios/en/writing/heading-level.toml");
const WRITING_BLOCKQUOTE: &str = include_str!("../../../scenarios/en/writing/blockquote.toml");
const WRITING_CODE_FENCE: &str = include_str!("../../../scenarios/en/writing/code-fence.toml");

// ============================================================================
// Public API
// ============================================================================

/// All embedded English scenarios as static string slices.
///
/// Each entry is the raw TOML content of a scenario file.
const EN_SCENARIOS: &[&str] = &[
    // Basic
    BASIC_DELETE,
    BASIC_INSERT,
    BASIC_REPLACE,
    // Clipboard
    CLIPBOARD_UNDO_REDO,
    CLIPBOARD_YANK_PASTE,
    // Editing
    EDITING_ADVANCED,
    EDITING_DELETE_SELECTION,
    EDITING_INDENTATION,
    EDITING_INDENTATION_PYTHON,
    EDITING_JOIN,
    EDITING_SURROUND,
    // Movement
    MOVEMENT_BASIC,
    MOVEMENT_COMBINED,
    MOVEMENT_COMMAND_LINE_GOTO,
    MOVEMENT_DOCUMENT,
    MOVEMENT_FIND_TILL,
    MOVEMENT_GOTO_COMMANDS,
    MOVEMENT_LINE_NAVIGATION,
    MOVEMENT_LINE,
    MOVEMENT_MATCH_BRACKETS,
    MOVEMENT_PARAGRAPH,
    MOVEMENT_PRECISION,
    MOVEMENT_SCROLL,
    MOVEMENT_WORD_BASICS,
    MOVEMENT_WORD,
    MOVEMENT_WORD_PYTHON,
    // Macros
    MACROS_BASIC,
    // Registers
    REGISTERS_NAMED,
    // Repeat
    REPEAT_BASIC,
    // Search
    SEARCH_BASIC,
    // Selection
    SELECTION_ADVANCED,
    SELECTION_LINE,
    SELECTION_ALL_REPLACE,
    SELECTION_TEXT_OBJECTS,
    // Writing
    WRITING_EMPHASIS,
    WRITING_LINKS,
    WRITING_LIST_INDENT,
    WRITING_HEADING_LEVEL,
    WRITING_BLOCKQUOTE,
    WRITING_CODE_FENCE,
];

/// Returns all embedded scenario TOML strings for the specified locale.
///
/// Currently only "en" (English) is supported. Returns an empty slice for
/// unsupported locales.
///
/// # Arguments
///
/// * `locale` - The locale code (e.g., "en")
///
/// # Returns
///
/// A slice of static string references, each containing the raw TOML content
/// of a scenario file.
///
/// # Example
///
/// ```
/// use helix_trainer::config::scenarios::embedded::get_embedded_scenarios;
///
/// let scenarios = get_embedded_scenarios("en");
/// assert!(!scenarios.is_empty());
/// ```
pub fn get_embedded_scenarios(locale: &str) -> &'static [&'static str] {
    match locale {
        "en" => EN_SCENARIOS,
        _ => &[],
    }
}

/// Returns the list of available embedded locales.
///
/// This function returns locale codes for which embedded scenarios exist.
pub fn available_embedded_locales() -> &'static [&'static str] {
    &["en"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_embedded_scenarios_en() {
        let scenarios = get_embedded_scenarios("en");
        assert_eq!(scenarios.len(), 40, "Expected 40 scenario files");
    }

    #[test]
    fn test_get_embedded_scenarios_unknown_locale() {
        let scenarios = get_embedded_scenarios("xx");
        assert!(scenarios.is_empty());
    }

    #[test]
    fn test_available_embedded_locales() {
        let locales = available_embedded_locales();
        assert!(locales.contains(&"en"));
    }

    #[test]
    fn test_all_scenarios_are_valid_toml() {
        for (idx, content) in get_embedded_scenarios("en").iter().enumerate() {
            let result: Result<toml::Value, _> = toml::from_str(content);
            assert!(
                result.is_ok(),
                "Scenario at index {} is not valid TOML: {:?}",
                idx,
                result.err()
            );
        }
    }
}
