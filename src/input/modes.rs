//! Input mode typestate markers
//!
//! This module provides zero-cost type-level markers for input modes,
//! enabling compile-time enforcement of mode-specific key mappings.
//!
//! # Type Safety
//!
//! The typestate pattern ensures that key mappings are only available
//! for the correct input mode:
//!
//! ```ignore
//! // Normal mode keys compile:
//! let cmd = KeyMapping::map_key::<NormalModeKeys>(key);  // Returns Some("h") for 'h'
//!
//! // Insert mode keys compile:
//! let text = KeyMapping::map_key::<InsertModeKeys>(key);  // Returns text input
//! ```

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use helix_trainer::helix::commands::*;

/// Private module for sealing the InputMode trait
mod private {
    pub trait Sealed {}
}

/// Sealed trait for input modes
///
/// This trait can only be implemented by types in this module,
/// ensuring all possible input modes are known at compile time.
pub trait InputMode: private::Sealed {
    /// Get the display name of this input mode
    ///
    /// Used for debugging and display purposes.
    #[allow(dead_code)]
    fn name() -> &'static str;
}

// ============================================================================
// Mode marker types (zero-sized types)
// ============================================================================

/// Normal mode key mapping marker (zero-sized type)
///
/// Represents key mappings valid in Normal mode where commands are executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalModeKeys;

/// Insert mode key mapping marker (zero-sized type)
///
/// Represents key mappings valid in Insert mode where text is being inserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertModeKeys;

/// View mode key mapping marker (zero-sized type)
///
/// Represents key mappings valid in View mode (z prefix commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewModeKeys;

/// Search mode key mapping marker (zero-sized type)
///
/// Represents key mappings valid in Search mode (/ and ? commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchModeKeys;

/// Goto mode key mapping marker (zero-sized type)
///
/// Represents key mappings valid in Goto mode (g prefix commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GotoModeKeys;

/// Match mode key mapping marker (zero-sized type)
///
/// Represents key mappings valid in Match mode (m prefix commands).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchModeKeys;

// ============================================================================
// Sealed trait implementations
// ============================================================================

impl private::Sealed for NormalModeKeys {}
impl private::Sealed for InsertModeKeys {}
impl private::Sealed for ViewModeKeys {}
impl private::Sealed for SearchModeKeys {}
impl private::Sealed for GotoModeKeys {}
impl private::Sealed for MatchModeKeys {}

impl InputMode for NormalModeKeys {
    fn name() -> &'static str {
        "NORMAL"
    }
}

impl InputMode for InsertModeKeys {
    fn name() -> &'static str {
        "INSERT"
    }
}

impl InputMode for ViewModeKeys {
    fn name() -> &'static str {
        "VIEW"
    }
}

impl InputMode for SearchModeKeys {
    fn name() -> &'static str {
        "SEARCH"
    }
}

impl InputMode for GotoModeKeys {
    fn name() -> &'static str {
        "GOTO"
    }
}

impl InputMode for MatchModeKeys {
    fn name() -> &'static str {
        "MATCH"
    }
}

// ============================================================================
// KeyMapper trait and implementations
// ============================================================================

/// Trait for mode-specific key mapping
///
/// This trait provides compile-time safety for key mappings by requiring
/// a specific mode marker type. Each mode has its own set of valid key mappings.
pub trait KeyMapper<M: InputMode> {
    /// The output type for mapped keys
    type Output;

    /// Map a key event to a command or input value
    ///
    /// Returns `Some(output)` if the key is valid for this mode,
    /// `None` if the key is not recognized.
    fn map_key(key: KeyEvent) -> Option<Self::Output>;
}

/// Zero-sized struct that implements KeyMapper for all modes
///
/// Use this struct's associated functions to map keys with type safety:
///
/// ```ignore
/// let cmd = KeyMapping::map_key::<NormalModeKeys>(key);
/// let text = KeyMapping::map_key::<InsertModeKeys>(key);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct KeyMapping;

// ============================================================================
// Normal mode key mapping
// ============================================================================

impl KeyMapper<NormalModeKeys> for KeyMapping {
    type Output = &'static str;

    fn map_key(key: KeyEvent) -> Option<Self::Output> {
        match (key.code, key.modifiers) {
            // Movement commands
            (KeyCode::Char('h'), KeyModifiers::NONE) => Some(CMD_MOVE_LEFT),
            (KeyCode::Char('j'), KeyModifiers::NONE) => Some(CMD_MOVE_DOWN),
            (KeyCode::Char('k'), KeyModifiers::NONE) => Some(CMD_MOVE_UP),
            (KeyCode::Char('l'), KeyModifiers::NONE) => Some(CMD_MOVE_RIGHT),

            // Word movement
            (KeyCode::Char('w'), KeyModifiers::NONE) => Some(CMD_MOVE_WORD_FORWARD),
            (KeyCode::Char('b'), KeyModifiers::NONE) => Some(CMD_MOVE_WORD_BACKWARD),
            (KeyCode::Char('e'), KeyModifiers::NONE) => Some(CMD_MOVE_WORD_END),

            // WORD movement (whitespace-delimited)
            (KeyCode::Char('W'), KeyModifiers::SHIFT) => Some(CMD_MOVE_LONG_WORD_FORWARD),
            (KeyCode::Char('B'), KeyModifiers::SHIFT) => Some(CMD_MOVE_LONG_WORD_BACKWARD),
            (KeyCode::Char('E'), KeyModifiers::SHIFT) => Some(CMD_MOVE_LONG_WORD_END),

            // Selection commands (basic)
            (KeyCode::Char('X'), KeyModifiers::SHIFT) => Some(CMD_EXTEND_LINE),
            (KeyCode::Char('%'), KeyModifiers::NONE) => Some(CMD_SELECT_ALL),
            (KeyCode::Char(';'), KeyModifiers::NONE) => Some(CMD_COLLAPSE_SELECTION),

            // Selection commands
            (KeyCode::Char('s'), KeyModifiers::NONE) => Some(CMD_SELECT_REGEX),
            (KeyCode::Char('S'), KeyModifiers::SHIFT) => Some(CMD_SPLIT_SELECTION),
            (KeyCode::Char('s'), KeyModifiers::ALT) => Some(CMD_SPLIT_SELECTION_NEWLINES),
            (KeyCode::Char('&'), KeyModifiers::NONE) => Some(CMD_ALIGN_SELECTIONS),
            (KeyCode::Char('_'), KeyModifiers::NONE) => Some(CMD_TRIM_SELECTIONS),
            (KeyCode::Char('-'), KeyModifiers::ALT) => Some(CMD_MERGE_SELECTIONS),
            (KeyCode::Char('_'), KeyModifiers::ALT) => Some(CMD_MERGE_CONSECUTIVE),
            (KeyCode::Char('C'), KeyModifiers::SHIFT) => Some(CMD_COPY_SELECTION_NEXT),
            (KeyCode::Char('C'), KeyModifiers::ALT) => Some(CMD_COPY_SELECTION_PREV),
            (KeyCode::Char('K'), KeyModifiers::SHIFT) => Some(CMD_KEEP_MATCHING),
            (KeyCode::Char('K'), KeyModifiers::ALT) => Some(CMD_REMOVE_MATCHING),
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(CMD_TOGGLE_COMMENTS),

            // Case switching
            (KeyCode::Char('~'), KeyModifiers::NONE) => Some(CMD_SWITCH_CASE),
            (KeyCode::Char('`'), KeyModifiers::NONE) => Some(CMD_SWITCH_CASE_ALT),

            // Selection and deletion commands
            (KeyCode::Char('x'), KeyModifiers::NONE) => Some(CMD_SELECT_LINE),
            (KeyCode::Char('d'), KeyModifiers::NONE) => Some(CMD_DELETE_SELECTION),
            (KeyCode::Char('c'), KeyModifiers::NONE) => Some(CMD_CHANGE),
            (KeyCode::Char('J'), KeyModifiers::SHIFT) => Some(CMD_JOIN_LINES),

            // Indentation
            (KeyCode::Char('>'), KeyModifiers::NONE) => Some(CMD_INDENT),
            (KeyCode::Char('<'), KeyModifiers::NONE) => Some(CMD_DEDENT),

            // Yank and paste
            (KeyCode::Char('y'), KeyModifiers::NONE) => Some(CMD_YANK),
            (KeyCode::Char('p'), KeyModifiers::NONE) => Some(CMD_PASTE_AFTER),
            (KeyCode::Char('P'), KeyModifiers::SHIFT) => Some(CMD_PASTE_BEFORE),

            // Mode changes and editing
            (KeyCode::Char('i'), KeyModifiers::NONE) => Some(CMD_INSERT),
            (KeyCode::Char('a'), KeyModifiers::NONE) => Some(CMD_APPEND),
            (KeyCode::Char('I'), KeyModifiers::SHIFT) => Some(CMD_INSERT_LINE_START),
            (KeyCode::Char('A'), KeyModifiers::SHIFT) => Some(CMD_APPEND_LINE_END),
            (KeyCode::Char('o'), KeyModifiers::NONE) => Some(CMD_OPEN_BELOW),
            (KeyCode::Char('O'), KeyModifiers::SHIFT) => Some(CMD_OPEN_ABOVE),

            // Replace character
            (KeyCode::Char('r'), KeyModifiers::NONE) => Some(CMD_REPLACE),

            // Find/till character (prefix commands - need second char)
            (KeyCode::Char('f'), KeyModifiers::NONE) => Some(CMD_FIND_CHAR),
            (KeyCode::Char('F'), KeyModifiers::SHIFT) => Some(CMD_FIND_CHAR_REVERSE),
            (KeyCode::Char('t'), KeyModifiers::NONE) => Some(CMD_TILL_CHAR),
            (KeyCode::Char('T'), KeyModifiers::SHIFT) => Some(CMD_TILL_CHAR_REVERSE),

            // Match mode (prefix for mm)
            (KeyCode::Char('m'), KeyModifiers::NONE) => Some(CMD_MATCH_MODE),

            // Select mode
            (KeyCode::Char('v'), KeyModifiers::NONE) => Some(CMD_SELECT_MODE),

            // Flip selection direction
            (KeyCode::Char(';'), KeyModifiers::ALT) => Some(CMD_FLIP_SELECTIONS),

            // Search commands
            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(CMD_SEARCH_FORWARD),
            (KeyCode::Char('?'), KeyModifiers::NONE) => Some(CMD_SEARCH_BACKWARD),
            (KeyCode::Char('n'), KeyModifiers::NONE) => Some(CMD_SEARCH_NEXT),
            (KeyCode::Char('N'), KeyModifiers::SHIFT) => Some(CMD_SEARCH_PREV),
            (KeyCode::Char('*'), KeyModifiers::NONE) => Some(CMD_SEARCH_WORD),
            (KeyCode::Char('*'), KeyModifiers::ALT) => Some(CMD_SEARCH_SELECTION),

            // View mode (prefix for zz, zt, zb, zm, zj, zk)
            (KeyCode::Char('z'), KeyModifiers::NONE) => Some(CMD_VIEW_MODE),

            // Undo/Redo
            (KeyCode::Char('u'), KeyModifiers::NONE) => Some(CMD_UNDO),
            (KeyCode::Char('U'), KeyModifiers::SHIFT) => Some(CMD_REDO),
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => Some(CMD_CTRL_R),

            // Repeat last action
            (KeyCode::Char('.'), KeyModifiers::NONE) => Some(CMD_REPEAT),

            // Document movement - 'g' is prefix for goto commands
            (KeyCode::Char('g'), KeyModifiers::NONE) => Some(CMD_GOTO_MODE),

            _ => None,
        }
    }
}

// ============================================================================
// Insert mode key mapping
// ============================================================================

impl KeyMapper<InsertModeKeys> for KeyMapping {
    type Output = Cow<'static, str>;

    fn map_key(key: KeyEvent) -> Option<Self::Output> {
        match key.code {
            KeyCode::Char(c) => Some(Cow::Owned(c.to_string())),
            KeyCode::Enter => Some(Cow::Borrowed("\n")),
            KeyCode::Backspace => Some(Cow::Borrowed(CMD_BACKSPACE)),
            KeyCode::Left => Some(Cow::Borrowed(CMD_ARROW_LEFT)),
            KeyCode::Right => Some(Cow::Borrowed(CMD_ARROW_RIGHT)),
            KeyCode::Up => Some(Cow::Borrowed(CMD_ARROW_UP)),
            KeyCode::Down => Some(Cow::Borrowed(CMD_ARROW_DOWN)),
            KeyCode::Esc => Some(Cow::Borrowed(CMD_ESCAPE)),
            _ => None,
        }
    }
}

// ============================================================================
// View mode key mapping (z prefix submenu)
// ============================================================================

impl KeyMapper<ViewModeKeys> for KeyMapping {
    type Output = &'static str;

    fn map_key(key: KeyEvent) -> Option<Self::Output> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('z'), KeyModifiers::NONE) => Some(CMD_VIEW_CENTER),
            (KeyCode::Char('t'), KeyModifiers::NONE) => Some(CMD_VIEW_TOP),
            (KeyCode::Char('b'), KeyModifiers::NONE) => Some(CMD_VIEW_BOTTOM),
            (KeyCode::Char('m'), KeyModifiers::NONE) => Some(CMD_VIEW_CENTER_HORIZONTAL),
            (KeyCode::Char('j'), KeyModifiers::NONE) => Some(CMD_SCROLL_DOWN),
            (KeyCode::Char('k'), KeyModifiers::NONE) => Some(CMD_SCROLL_UP),
            _ => None,
        }
    }
}

// ============================================================================
// Goto mode key mapping (g prefix submenu)
// ============================================================================

impl KeyMapper<GotoModeKeys> for KeyMapping {
    type Output = &'static str;

    fn map_key(key: KeyEvent) -> Option<Self::Output> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('g'), KeyModifiers::NONE) => Some(CMD_GOTO_FILE_START),
            (KeyCode::Char('h'), KeyModifiers::NONE) => Some(CMD_GOTO_LINE_START),
            (KeyCode::Char('l'), KeyModifiers::NONE) => Some(CMD_GOTO_LINE_END),
            (KeyCode::Char('s'), KeyModifiers::NONE) => Some(CMD_GOTO_FIRST_NONWHITESPACE),
            (KeyCode::Char('e'), KeyModifiers::NONE) => Some(CMD_GOTO_LAST_LINE),
            _ => None,
        }
    }
}

// ============================================================================
// Match mode key mapping (m prefix submenu)
// ============================================================================

impl KeyMapper<MatchModeKeys> for KeyMapping {
    type Output = &'static str;

    fn map_key(key: KeyEvent) -> Option<Self::Output> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('m'), KeyModifiers::NONE) => Some(CMD_MATCH_BRACKETS),
            _ => None,
        }
    }
}

// ============================================================================
// Search mode key mapping (for search input mode)
// ============================================================================

impl KeyMapper<SearchModeKeys> for KeyMapping {
    type Output = Cow<'static, str>;

    fn map_key(key: KeyEvent) -> Option<Self::Output> {
        match key.code {
            KeyCode::Char(c) => Some(Cow::Owned(c.to_string())),
            KeyCode::Backspace => Some(Cow::Borrowed(CMD_BACKSPACE)),
            KeyCode::Enter => Some(Cow::Borrowed("\n")), // Confirm search
            KeyCode::Esc => Some(Cow::Borrowed(CMD_ESCAPE)), // Cancel search
            _ => None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod mode_marker_tests {
        use super::*;
        use std::marker::PhantomData;

        #[test]
        fn test_mode_names() {
            assert_eq!(NormalModeKeys::name(), "NORMAL");
            assert_eq!(InsertModeKeys::name(), "INSERT");
            assert_eq!(ViewModeKeys::name(), "VIEW");
            assert_eq!(SearchModeKeys::name(), "SEARCH");
            assert_eq!(GotoModeKeys::name(), "GOTO");
            assert_eq!(MatchModeKeys::name(), "MATCH");
        }

        #[test]
        fn test_mode_markers_are_zero_sized() {
            assert_eq!(std::mem::size_of::<NormalModeKeys>(), 0);
            assert_eq!(std::mem::size_of::<InsertModeKeys>(), 0);
            assert_eq!(std::mem::size_of::<ViewModeKeys>(), 0);
            assert_eq!(std::mem::size_of::<SearchModeKeys>(), 0);
            assert_eq!(std::mem::size_of::<GotoModeKeys>(), 0);
            assert_eq!(std::mem::size_of::<MatchModeKeys>(), 0);
            assert_eq!(std::mem::size_of::<PhantomData<NormalModeKeys>>(), 0);
        }
    }

    mod normal_mode_key_mapping_tests {
        use super::*;

        #[test]
        fn test_movement_hjkl() {
            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(h),
                Some(CMD_MOVE_LEFT)
            );

            let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(j),
                Some(CMD_MOVE_DOWN)
            );

            let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(k),
                Some(CMD_MOVE_UP)
            );

            let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(l),
                Some(CMD_MOVE_RIGHT)
            );
        }

        #[test]
        fn test_word_movement() {
            let w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(w),
                Some(CMD_MOVE_WORD_FORWARD)
            );

            let b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(b),
                Some(CMD_MOVE_WORD_BACKWARD)
            );

            let e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(e),
                Some(CMD_MOVE_WORD_END)
            );
        }

        #[test]
        fn test_editing_commands() {
            let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(d),
                Some(CMD_DELETE_SELECTION)
            );

            let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(x),
                Some(CMD_SELECT_LINE)
            );

            let y = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(y),
                Some(CMD_YANK)
            );

            let p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(p),
                Some(CMD_PASTE_AFTER)
            );
        }

        #[test]
        fn test_mode_entry_commands() {
            let i = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(i),
                Some(CMD_INSERT)
            );

            let a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(a),
                Some(CMD_APPEND)
            );

            let v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(v),
                Some(CMD_SELECT_MODE)
            );
        }

        #[test]
        fn test_prefix_commands() {
            let g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(g),
                Some(CMD_GOTO_MODE)
            );

            let z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(z),
                Some(CMD_VIEW_MODE)
            );

            let m = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(m),
                Some(CMD_MATCH_MODE)
            );
        }

        #[test]
        fn test_unknown_key_returns_none() {
            let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<NormalModeKeys>>::map_key(f1), None);

            let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<NormalModeKeys>>::map_key(q), None);
        }

        #[test]
        fn test_wrong_modifier_returns_none() {
            let h_ctrl = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(h_ctrl),
                None
            );

            let h_alt = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT);
            assert_eq!(
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(h_alt),
                None
            );
        }
    }

    mod insert_mode_key_mapping_tests {
        use super::*;

        #[test]
        fn test_regular_char() {
            let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key),
                Some(Cow::Owned("a".to_string()))
            );
        }

        #[test]
        fn test_special_chars() {
            let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(space),
                Some(Cow::Owned(" ".to_string()))
            );

            let at = KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(at),
                Some(Cow::Owned("@".to_string()))
            );
        }

        #[test]
        fn test_enter() {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key),
                Some(Cow::Borrowed("\n"))
            );
        }

        #[test]
        fn test_backspace() {
            let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key),
                Some(Cow::Borrowed(CMD_BACKSPACE))
            );
        }

        #[test]
        fn test_arrow_keys() {
            let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(left),
                Some(Cow::Borrowed(CMD_ARROW_LEFT))
            );

            let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(right),
                Some(Cow::Borrowed(CMD_ARROW_RIGHT))
            );

            let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(up),
                Some(Cow::Borrowed(CMD_ARROW_UP))
            );

            let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(down),
                Some(Cow::Borrowed(CMD_ARROW_DOWN))
            );
        }

        #[test]
        fn test_escape() {
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key),
                Some(Cow::Borrowed(CMD_ESCAPE))
            );
        }

        #[test]
        fn test_unknown_returns_none() {
            let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(tab),
                None
            );

            let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<InsertModeKeys>>::map_key(f1), None);
        }
    }

    mod view_mode_key_mapping_tests {
        use super::*;

        #[test]
        fn test_view_commands() {
            let z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(z),
                Some(CMD_VIEW_CENTER)
            );

            let t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(t),
                Some(CMD_VIEW_TOP)
            );

            let b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(b),
                Some(CMD_VIEW_BOTTOM)
            );

            let m = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(m),
                Some(CMD_VIEW_CENTER_HORIZONTAL)
            );

            let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(j),
                Some(CMD_SCROLL_DOWN)
            );

            let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(k),
                Some(CMD_SCROLL_UP)
            );
        }

        #[test]
        fn test_invalid_view_command() {
            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<ViewModeKeys>>::map_key(h), None);
        }
    }

    mod goto_mode_key_mapping_tests {
        use super::*;

        #[test]
        fn test_goto_commands() {
            let g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(g),
                Some(CMD_GOTO_FILE_START)
            );

            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(h),
                Some(CMD_GOTO_LINE_START)
            );

            let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(l),
                Some(CMD_GOTO_LINE_END)
            );

            let s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(s),
                Some(CMD_GOTO_FIRST_NONWHITESPACE)
            );

            let e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(e),
                Some(CMD_GOTO_LAST_LINE)
            );
        }

        #[test]
        fn test_invalid_goto_command() {
            let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<GotoModeKeys>>::map_key(x), None);
        }
    }

    mod match_mode_key_mapping_tests {
        use super::*;

        #[test]
        fn test_match_brackets() {
            let m = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<MatchModeKeys>>::map_key(m),
                Some(CMD_MATCH_BRACKETS)
            );
        }

        #[test]
        fn test_invalid_match_command() {
            let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<MatchModeKeys>>::map_key(x), None);
        }
    }

    mod search_mode_key_mapping_tests {
        use super::*;

        #[test]
        fn test_regular_char() {
            let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<SearchModeKeys>>::map_key(key),
                Some(Cow::Owned("a".to_string()))
            );
        }

        #[test]
        fn test_backspace() {
            let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<SearchModeKeys>>::map_key(key),
                Some(Cow::Borrowed(CMD_BACKSPACE))
            );
        }

        #[test]
        fn test_enter_confirms_search() {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<SearchModeKeys>>::map_key(key),
                Some(Cow::Borrowed("\n"))
            );
        }

        #[test]
        fn test_escape_cancels_search() {
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(
                <KeyMapping as KeyMapper<SearchModeKeys>>::map_key(key),
                Some(Cow::Borrowed(CMD_ESCAPE))
            );
        }

        #[test]
        fn test_unknown_returns_none() {
            let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
            assert_eq!(<KeyMapping as KeyMapper<SearchModeKeys>>::map_key(f1), None);
        }
    }
}
