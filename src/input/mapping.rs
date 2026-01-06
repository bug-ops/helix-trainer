//! Keyboard to Helix command mapping
//!
//! Maps keyboard events to Helix editor commands with compile-time mode safety.
//!
//! # Typestate Pattern
//!
//! This module uses the typestate pattern to ensure key mappings are only used
//! for the correct editor mode at compile time:
//!
//! ```ignore
//! use helix_trainer::input::mapping::{KeyMapping, KeyMapper, NormalModeKeys, InsertModeKeys};
//!
//! // Type-safe normal mode mapping
//! let cmd = <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(key);
//!
//! // Type-safe insert mode mapping
//! let text = <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key);
//! ```
//!
//! # Backward Compatibility
//!
//! The legacy functions `map_key_to_helix_command` and `handle_insert_mode_input`
//! are still available and delegate to the type-safe implementations.

use crossterm::event::KeyEvent;
use std::borrow::Cow;

// Re-export mode types and KeyMapper trait from modes module for public API
// These are intentionally re-exported even if not used locally, to provide
// a unified import path for consumers of this module.
#[allow(unused_imports)]
pub use super::modes::GotoModeKeys;
#[allow(unused_imports)]
pub use super::modes::InputMode;
pub use super::modes::InsertModeKeys;
pub use super::modes::KeyMapper;
pub use super::modes::KeyMapping;
#[allow(unused_imports)]
pub use super::modes::MatchModeKeys;
pub use super::modes::NormalModeKeys;
#[allow(unused_imports)]
pub use super::modes::SearchModeKeys;
#[allow(unused_imports)]
pub use super::modes::ViewModeKeys;

/// Map key to Helix command (Normal mode)
///
/// Returns the command string for valid Helix commands, None for unknown keys.
///
/// # Backward Compatibility
///
/// This function is provided for backward compatibility. New code should use
/// the type-safe `KeyMapper` trait:
///
/// ```ignore
/// use helix_trainer::input::mapping::{KeyMapping, KeyMapper, NormalModeKeys};
///
/// let cmd = <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(key);
/// ```
///
/// # Type Safety
///
/// While this function works at runtime, it provides no compile-time guarantee
/// that the caller is in Normal mode. For compile-time safety, use the
/// `KeyMapper<NormalModeKeys>` implementation instead.
#[inline]
pub fn map_key_to_helix_command(key: KeyEvent) -> Option<&'static str> {
    <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(key)
}

/// Convert key to text input for Insert mode
///
/// Returns the text representation of the key for insertion.
///
/// # Backward Compatibility
///
/// This function is provided for backward compatibility. New code should use
/// the type-safe `KeyMapper` trait:
///
/// ```ignore
/// use helix_trainer::input::mapping::{KeyMapping, KeyMapper, InsertModeKeys};
///
/// let text = <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key);
/// ```
///
/// # Type Safety
///
/// While this function works at runtime, it provides no compile-time guarantee
/// that the caller is in Insert mode. For compile-time safety, use the
/// `KeyMapper<InsertModeKeys>` implementation instead.
#[inline]
pub fn handle_insert_mode_input(key: KeyEvent) -> Option<Cow<'static, str>> {
    <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use helix_trainer::helix::commands::*;

    // Unit tests for map_key_to_helix_command()
    mod map_key_to_helix_command_tests {
        use super::*;

        // Movement commands
        #[test]
        fn test_map_key_movement_hjkl() {
            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(h), Some(CMD_MOVE_LEFT));

            let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(j), Some(CMD_MOVE_DOWN));

            let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(k), Some(CMD_MOVE_UP));

            let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(l), Some(CMD_MOVE_RIGHT));
        }

        #[test]
        fn test_map_key_word_movement() {
            let w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(w), Some(CMD_MOVE_WORD_FORWARD));

            let b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(b), Some(CMD_MOVE_WORD_BACKWARD));

            let e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(e), Some(CMD_MOVE_WORD_END));
        }

        #[test]
        fn test_map_key_line_movement() {
            // Note: '0' is NOT a command in Helix - use 'gh' for goto line start
            let zero = KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(zero), None);

            // Note: '$' is NOT a line end command in Helix - use 'gl' for goto line end
            let dollar = KeyEvent::new(KeyCode::Char('$'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(dollar), None);
        }

        // Selection commands
        #[test]
        fn test_map_key_selection() {
            let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(x), Some(CMD_SELECT_LINE));

            let x_shift = KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT);
            assert_eq!(map_key_to_helix_command(x_shift), Some(CMD_EXTEND_LINE));

            let percent = KeyEvent::new(KeyCode::Char('%'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(percent), Some(CMD_SELECT_ALL));

            let semicolon = KeyEvent::new(KeyCode::Char(';'), KeyModifiers::NONE);
            assert_eq!(
                map_key_to_helix_command(semicolon),
                Some(CMD_COLLAPSE_SELECTION)
            );
        }

        // Deletion commands
        #[test]
        fn test_map_key_deletion() {
            let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(d), Some(CMD_DELETE_SELECTION));

            let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(c), Some(CMD_CHANGE));

            let j_shift = KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT);
            assert_eq!(map_key_to_helix_command(j_shift), Some(CMD_JOIN_LINES));
        }

        // Case switching
        #[test]
        fn test_map_key_case_switch() {
            let tilde = KeyEvent::new(KeyCode::Char('~'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(tilde), Some(CMD_SWITCH_CASE));

            let backtick = KeyEvent::new(KeyCode::Char('`'), KeyModifiers::NONE);
            assert_eq!(
                map_key_to_helix_command(backtick),
                Some(CMD_SWITCH_CASE_ALT)
            );
        }

        // WORD movement
        #[test]
        fn test_map_key_word_movement_caps() {
            let w_shift = KeyEvent::new(KeyCode::Char('W'), KeyModifiers::SHIFT);
            assert_eq!(
                map_key_to_helix_command(w_shift),
                Some(CMD_MOVE_LONG_WORD_FORWARD)
            );

            let b_shift = KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT);
            assert_eq!(
                map_key_to_helix_command(b_shift),
                Some(CMD_MOVE_LONG_WORD_BACKWARD)
            );

            let e_shift = KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT);
            assert_eq!(
                map_key_to_helix_command(e_shift),
                Some(CMD_MOVE_LONG_WORD_END)
            );
        }

        // Indentation
        #[test]
        fn test_map_key_indentation() {
            let gt = KeyEvent::new(KeyCode::Char('>'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(gt), Some(CMD_INDENT));

            let lt = KeyEvent::new(KeyCode::Char('<'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(lt), Some(CMD_DEDENT));
        }

        // Clipboard
        #[test]
        fn test_map_key_clipboard() {
            let y = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(y), Some(CMD_YANK));

            let p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(p), Some(CMD_PASTE_AFTER));

            let p_shift = KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT);
            assert_eq!(map_key_to_helix_command(p_shift), Some(CMD_PASTE_BEFORE));
        }

        // Mode changes
        #[test]
        fn test_map_key_mode_changes() {
            let i = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(i), Some(CMD_INSERT));

            let a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(a), Some(CMD_APPEND));

            let i_shift = KeyEvent::new(KeyCode::Char('I'), KeyModifiers::SHIFT);
            assert_eq!(
                map_key_to_helix_command(i_shift),
                Some(CMD_INSERT_LINE_START)
            );

            let a_shift = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
            assert_eq!(map_key_to_helix_command(a_shift), Some(CMD_APPEND_LINE_END));

            let o = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(o), Some(CMD_OPEN_BELOW));

            let o_shift = KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT);
            assert_eq!(map_key_to_helix_command(o_shift), Some(CMD_OPEN_ABOVE));
        }

        // Replace
        #[test]
        fn test_map_key_replace() {
            let r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(r), Some(CMD_REPLACE));
        }

        // Undo/Redo
        #[test]
        fn test_map_key_undo_redo() {
            let u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(u), Some(CMD_UNDO));

            let u_shift = KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT);
            assert_eq!(map_key_to_helix_command(u_shift), Some(CMD_REDO));

            let r_ctrl = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
            assert_eq!(map_key_to_helix_command(r_ctrl), Some(CMD_CTRL_R));
        }

        // Repeat
        #[test]
        fn test_map_key_repeat() {
            let dot = KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(dot), Some(CMD_REPEAT));
        }

        // Document movement
        #[test]
        fn test_map_key_document_movement() {
            // 'g' is the prefix for goto commands (gg, ge, gh, gl, gs)
            let g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(g), Some("g"));
        }

        // Edge cases
        #[test]
        fn test_map_key_unknown_returns_none() {
            // 'z' is now a known command (view mode prefix)
            let z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(z), Some(CMD_VIEW_MODE));

            // F1 is still unknown
            let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(f1), None);

            // Some random letter that's not mapped
            let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            assert_eq!(map_key_to_helix_command(q), None);
        }

        #[test]
        fn test_map_key_with_wrong_modifier() {
            let h_ctrl = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
            assert_eq!(map_key_to_helix_command(h_ctrl), None);

            let h_alt = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::ALT);
            assert_eq!(map_key_to_helix_command(h_alt), None);
        }
    }

    // Unit tests for handle_insert_mode_input()
    mod handle_insert_mode_input_tests {
        use super::*;

        #[test]
        fn test_handle_insert_mode_input_regular_char() {
            let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Owned("a".to_string())));
        }

        #[test]
        fn test_handle_insert_mode_input_space() {
            let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Owned(" ".to_string())));
        }

        #[test]
        fn test_handle_insert_mode_input_digit() {
            let key = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Owned("5".to_string())));
        }

        #[test]
        fn test_handle_insert_mode_input_special_char() {
            let key = KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Owned("@".to_string())));
        }

        #[test]
        fn test_handle_insert_mode_input_enter() {
            let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed("\n")));
        }

        #[test]
        fn test_handle_insert_mode_input_backspace() {
            let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed(CMD_BACKSPACE)));
        }

        #[test]
        fn test_handle_insert_mode_input_arrow_left() {
            let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed(CMD_ARROW_LEFT)));
        }

        #[test]
        fn test_handle_insert_mode_input_arrow_right() {
            let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed(CMD_ARROW_RIGHT)));
        }

        #[test]
        fn test_handle_insert_mode_input_arrow_up() {
            let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed(CMD_ARROW_UP)));
        }

        #[test]
        fn test_handle_insert_mode_input_arrow_down() {
            let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed(CMD_ARROW_DOWN)));
        }

        #[test]
        fn test_handle_insert_mode_input_escape() {
            // Escape exits insert mode and returns to normal mode
            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, Some(Cow::Borrowed(CMD_ESCAPE)));
        }

        #[test]
        fn test_handle_insert_mode_input_tab_returns_none() {
            let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, None);
        }

        #[test]
        fn test_handle_insert_mode_input_f_keys_return_none() {
            let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
            let result = handle_insert_mode_input(key);
            assert_eq!(result, None);
        }
    }

    // Unit tests for typestate-based KeyMapper API
    mod typestate_api_tests {
        use super::*;

        #[test]
        fn test_normal_mode_keymapper_type_safety() {
            // Demonstrate that NormalModeKeys returns &'static str
            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            let cmd: Option<&'static str> = <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(h);
            assert_eq!(cmd, Some(CMD_MOVE_LEFT));
        }

        #[test]
        fn test_insert_mode_keymapper_type_safety() {
            // Demonstrate that InsertModeKeys returns Cow<'static, str>
            let a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            let text: Option<Cow<'static, str>> =
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(a);
            assert_eq!(text, Some(Cow::Owned("a".to_string())));
        }

        #[test]
        fn test_view_mode_keymapper() {
            let z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
            let cmd = <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(z);
            assert_eq!(cmd, Some(CMD_VIEW_CENTER));

            let t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
            let cmd = <KeyMapping as KeyMapper<ViewModeKeys>>::map_key(t);
            assert_eq!(cmd, Some(CMD_VIEW_TOP));
        }

        #[test]
        fn test_goto_mode_keymapper() {
            let g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
            let cmd = <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(g);
            assert_eq!(cmd, Some(CMD_GOTO_FILE_START));

            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            let cmd = <KeyMapping as KeyMapper<GotoModeKeys>>::map_key(h);
            assert_eq!(cmd, Some(CMD_GOTO_LINE_START));
        }

        #[test]
        fn test_match_mode_keymapper() {
            let m = KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE);
            let cmd = <KeyMapping as KeyMapper<MatchModeKeys>>::map_key(m);
            assert_eq!(cmd, Some(CMD_MATCH_BRACKETS));
        }

        #[test]
        fn test_search_mode_keymapper() {
            let a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            let text = <KeyMapping as KeyMapper<SearchModeKeys>>::map_key(a);
            assert_eq!(text, Some(Cow::Owned("a".to_string())));

            let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            let text = <KeyMapping as KeyMapper<SearchModeKeys>>::map_key(esc);
            assert_eq!(text, Some(Cow::Borrowed(CMD_ESCAPE)));
        }

        #[test]
        fn test_backward_compat_delegates_to_typestate() {
            // Verify that legacy functions produce same results as typestate API
            let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
            assert_eq!(
                map_key_to_helix_command(h),
                <KeyMapping as KeyMapper<NormalModeKeys>>::map_key(h)
            );

            let a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(
                handle_insert_mode_input(a),
                <KeyMapping as KeyMapper<InsertModeKeys>>::map_key(a)
            );
        }
    }
}
