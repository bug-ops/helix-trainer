//! Tests for key_mapping module

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;
use crate::input::typestate::key_mapping::{
    command_to_key_event, handle_insert_mode_input, is_count_compatible_command,
    map_key_to_helix_command, map_single_key_command, parse_helix_key_string,
};

// ============================================================================
// map_single_key_command tests
// ============================================================================

#[test]
fn test_map_single_key_movement_commands() {
    assert_eq!(
        map_single_key_command('h', KeyModifiers::NONE),
        Some(CMD_MOVE_LEFT)
    );
    assert_eq!(
        map_single_key_command('j', KeyModifiers::NONE),
        Some(CMD_MOVE_DOWN)
    );
    assert_eq!(
        map_single_key_command('k', KeyModifiers::NONE),
        Some(CMD_MOVE_UP)
    );
    assert_eq!(
        map_single_key_command('l', KeyModifiers::NONE),
        Some(CMD_MOVE_RIGHT)
    );
}

#[test]
fn test_map_single_key_word_movement() {
    assert_eq!(
        map_single_key_command('w', KeyModifiers::NONE),
        Some(CMD_MOVE_WORD_FORWARD)
    );
    assert_eq!(
        map_single_key_command('b', KeyModifiers::NONE),
        Some(CMD_MOVE_WORD_BACKWARD)
    );
    assert_eq!(
        map_single_key_command('e', KeyModifiers::NONE),
        Some(CMD_MOVE_WORD_END)
    );
}

#[test]
fn test_map_single_key_long_word_movement() {
    assert_eq!(
        map_single_key_command('W', KeyModifiers::SHIFT),
        Some(CMD_MOVE_LONG_WORD_FORWARD)
    );
    assert_eq!(
        map_single_key_command('B', KeyModifiers::SHIFT),
        Some(CMD_MOVE_LONG_WORD_BACKWARD)
    );
    assert_eq!(
        map_single_key_command('E', KeyModifiers::SHIFT),
        Some(CMD_MOVE_LONG_WORD_END)
    );
}

#[test]
fn test_map_single_key_selection_commands() {
    assert_eq!(
        map_single_key_command('x', KeyModifiers::NONE),
        Some(CMD_SELECT_LINE)
    );
    assert_eq!(
        map_single_key_command('X', KeyModifiers::SHIFT),
        Some(CMD_EXTEND_LINE)
    );
    assert_eq!(
        map_single_key_command('%', KeyModifiers::SHIFT),
        Some(CMD_SELECT_ALL)
    );
    assert_eq!(
        map_single_key_command(';', KeyModifiers::NONE),
        Some(CMD_COLLAPSE_SELECTION)
    );
    assert_eq!(
        map_single_key_command('v', KeyModifiers::NONE),
        Some(CMD_SELECT_MODE)
    );
}

#[test]
fn test_map_single_key_editing_commands() {
    assert_eq!(
        map_single_key_command('d', KeyModifiers::NONE),
        Some(CMD_DELETE_SELECTION)
    );
    assert_eq!(
        map_single_key_command('c', KeyModifiers::NONE),
        Some(CMD_CHANGE)
    );
    assert_eq!(
        map_single_key_command('i', KeyModifiers::NONE),
        Some(CMD_INSERT)
    );
    assert_eq!(
        map_single_key_command('a', KeyModifiers::NONE),
        Some(CMD_APPEND)
    );
    assert_eq!(
        map_single_key_command('I', KeyModifiers::SHIFT),
        Some(CMD_INSERT_LINE_START)
    );
    assert_eq!(
        map_single_key_command('A', KeyModifiers::SHIFT),
        Some(CMD_APPEND_LINE_END)
    );
    assert_eq!(
        map_single_key_command('o', KeyModifiers::NONE),
        Some(CMD_OPEN_BELOW)
    );
    assert_eq!(
        map_single_key_command('O', KeyModifiers::SHIFT),
        Some(CMD_OPEN_ABOVE)
    );
    assert_eq!(
        map_single_key_command('J', KeyModifiers::SHIFT),
        Some(CMD_JOIN_LINES)
    );
}

#[test]
fn test_map_single_key_indentation() {
    assert_eq!(
        map_single_key_command('>', KeyModifiers::SHIFT),
        Some(CMD_INDENT)
    );
    assert_eq!(
        map_single_key_command('<', KeyModifiers::SHIFT),
        Some(CMD_DEDENT)
    );
}

#[test]
fn test_map_single_key_case() {
    assert_eq!(
        map_single_key_command('~', KeyModifiers::SHIFT),
        Some(CMD_SWITCH_CASE)
    );
    assert_eq!(
        map_single_key_command('`', KeyModifiers::NONE),
        Some(CMD_SWITCH_CASE_ALT)
    );
}

#[test]
fn test_map_single_key_clipboard() {
    assert_eq!(
        map_single_key_command('y', KeyModifiers::NONE),
        Some(CMD_YANK)
    );
    assert_eq!(
        map_single_key_command('p', KeyModifiers::NONE),
        Some(CMD_PASTE_AFTER)
    );
    assert_eq!(
        map_single_key_command('P', KeyModifiers::SHIFT),
        Some(CMD_PASTE_BEFORE)
    );
}

#[test]
fn test_map_single_key_undo_redo() {
    assert_eq!(
        map_single_key_command('u', KeyModifiers::NONE),
        Some(CMD_UNDO)
    );
    assert_eq!(
        map_single_key_command('U', KeyModifiers::SHIFT),
        Some(CMD_REDO)
    );
}

#[test]
fn test_map_single_key_repeat() {
    assert_eq!(
        map_single_key_command('.', KeyModifiers::NONE),
        Some(CMD_REPEAT)
    );
}

#[test]
fn test_map_single_key_search() {
    assert_eq!(
        map_single_key_command('/', KeyModifiers::NONE),
        Some(CMD_SEARCH_FORWARD)
    );
    assert_eq!(
        map_single_key_command('?', KeyModifiers::SHIFT),
        Some(CMD_SEARCH_BACKWARD)
    );
    assert_eq!(
        map_single_key_command('n', KeyModifiers::NONE),
        Some(CMD_SEARCH_NEXT)
    );
    assert_eq!(
        map_single_key_command('N', KeyModifiers::SHIFT),
        Some(CMD_SEARCH_PREV)
    );
    assert_eq!(
        map_single_key_command('*', KeyModifiers::SHIFT),
        Some(CMD_SEARCH_WORD)
    );
}

#[test]
fn test_map_single_key_selection_manipulation() {
    assert_eq!(
        map_single_key_command('s', KeyModifiers::NONE),
        Some(CMD_SELECT_REGEX)
    );
    assert_eq!(
        map_single_key_command('S', KeyModifiers::SHIFT),
        Some(CMD_SPLIT_SELECTION)
    );
    assert_eq!(
        map_single_key_command('&', KeyModifiers::SHIFT),
        Some(CMD_ALIGN_SELECTIONS)
    );
    assert_eq!(
        map_single_key_command('_', KeyModifiers::SHIFT),
        Some(CMD_TRIM_SELECTIONS)
    );
    assert_eq!(
        map_single_key_command('C', KeyModifiers::SHIFT),
        Some(CMD_COPY_SELECTION_NEXT)
    );
    assert_eq!(
        map_single_key_command('K', KeyModifiers::SHIFT),
        Some(CMD_KEEP_MATCHING)
    );
}

#[test]
fn test_map_single_key_unknown() {
    assert_eq!(map_single_key_command('q', KeyModifiers::NONE), None);
    assert_eq!(map_single_key_command('Z', KeyModifiers::SHIFT), None);
    assert_eq!(map_single_key_command('1', KeyModifiers::NONE), None);
}

// ============================================================================
// is_count_compatible_command tests
// ============================================================================

#[test]
fn test_count_compatible_movement() {
    assert!(is_count_compatible_command('h', KeyModifiers::NONE));
    assert!(is_count_compatible_command('j', KeyModifiers::NONE));
    assert!(is_count_compatible_command('k', KeyModifiers::NONE));
    assert!(is_count_compatible_command('l', KeyModifiers::NONE));
    assert!(is_count_compatible_command('w', KeyModifiers::NONE));
    assert!(is_count_compatible_command('b', KeyModifiers::NONE));
    assert!(is_count_compatible_command('e', KeyModifiers::NONE));
}

#[test]
fn test_count_compatible_long_word() {
    assert!(is_count_compatible_command('W', KeyModifiers::SHIFT));
    assert!(is_count_compatible_command('B', KeyModifiers::SHIFT));
    assert!(is_count_compatible_command('E', KeyModifiers::SHIFT));
}

#[test]
fn test_count_compatible_selection() {
    assert!(is_count_compatible_command('x', KeyModifiers::NONE));
    assert!(is_count_compatible_command('X', KeyModifiers::SHIFT));
}

#[test]
fn test_count_compatible_editing() {
    assert!(is_count_compatible_command('d', KeyModifiers::NONE));
    assert!(is_count_compatible_command('c', KeyModifiers::NONE));
    assert!(is_count_compatible_command('J', KeyModifiers::SHIFT));
    assert!(is_count_compatible_command('>', KeyModifiers::SHIFT));
    assert!(is_count_compatible_command('<', KeyModifiers::SHIFT));
}

#[test]
fn test_count_compatible_undo_redo() {
    assert!(is_count_compatible_command('u', KeyModifiers::NONE));
    assert!(is_count_compatible_command('U', KeyModifiers::SHIFT));
}

#[test]
fn test_count_compatible_search_nav() {
    assert!(is_count_compatible_command('n', KeyModifiers::NONE));
    assert!(is_count_compatible_command('N', KeyModifiers::SHIFT));
}

#[test]
fn test_count_not_compatible() {
    assert!(!is_count_compatible_command('y', KeyModifiers::NONE));
    assert!(!is_count_compatible_command('p', KeyModifiers::NONE));
    assert!(!is_count_compatible_command('i', KeyModifiers::NONE));
    assert!(!is_count_compatible_command('a', KeyModifiers::NONE));
    assert!(!is_count_compatible_command('o', KeyModifiers::NONE));
}

// ============================================================================
// map_key_to_helix_command tests
// ============================================================================

#[test]
fn test_map_key_char() {
    let key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_MOVE_LEFT));

    let key = KeyEvent::new(KeyCode::Char('W'), KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_helix_command(key),
        Some(CMD_MOVE_LONG_WORD_FORWARD)
    );
}

#[test]
fn test_map_key_special() {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_ESCAPE));

    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_BACKSPACE));

    let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_ARROW_LEFT));

    let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_ARROW_RIGHT));

    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_ARROW_UP));

    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), Some(CMD_ARROW_DOWN));
}

#[test]
fn test_map_key_unknown() {
    let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), None);

    let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
    assert_eq!(map_key_to_helix_command(key), None);
}

// ============================================================================
// handle_insert_mode_input tests
// ============================================================================

#[test]
fn test_insert_mode_char() {
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let result = handle_insert_mode_input(key);
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_ref(), "a");
}

#[test]
fn test_insert_mode_uppercase() {
    let key = KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::SHIFT);
    let result = handle_insert_mode_input(key);
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_ref(), "Z");
}

#[test]
fn test_insert_mode_special() {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(handle_insert_mode_input(key).unwrap().as_ref(), CMD_ESCAPE);

    let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    assert_eq!(
        handle_insert_mode_input(key).unwrap().as_ref(),
        CMD_BACKSPACE
    );

    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    assert_eq!(handle_insert_mode_input(key).unwrap().as_ref(), "\n");

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(handle_insert_mode_input(key).unwrap().as_ref(), "\t");
}

#[test]
fn test_insert_mode_arrows() {
    let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
    assert_eq!(
        handle_insert_mode_input(key).unwrap().as_ref(),
        CMD_ARROW_LEFT
    );

    let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    assert_eq!(
        handle_insert_mode_input(key).unwrap().as_ref(),
        CMD_ARROW_RIGHT
    );

    let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert_eq!(
        handle_insert_mode_input(key).unwrap().as_ref(),
        CMD_ARROW_UP
    );

    let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        handle_insert_mode_input(key).unwrap().as_ref(),
        CMD_ARROW_DOWN
    );
}

#[test]
fn test_insert_mode_unknown() {
    let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    assert!(handle_insert_mode_input(key).is_none());

    let key = KeyEvent::new(KeyCode::Home, KeyModifiers::NONE);
    assert!(handle_insert_mode_input(key).is_none());
}

// ============================================================================
// command_to_key_event tests
// ============================================================================

#[test]
fn test_command_to_key_single_lowercase() {
    let key = command_to_key_event("h");
    assert_eq!(key.code, KeyCode::Char('h'));
    assert_eq!(key.modifiers, KeyModifiers::NONE);
}

#[test]
fn test_command_to_key_single_uppercase() {
    let key = command_to_key_event("W");
    assert_eq!(key.code, KeyCode::Char('W'));
    assert_eq!(key.modifiers, KeyModifiers::SHIFT);
}

#[test]
fn test_command_to_key_escape() {
    let key = command_to_key_event("Escape");
    assert_eq!(key.code, KeyCode::Esc);
    assert_eq!(key.modifiers, KeyModifiers::NONE);
}

#[test]
fn test_command_to_key_arrows() {
    let key = command_to_key_event("Left");
    assert_eq!(key.code, KeyCode::Left);

    let key = command_to_key_event("Right");
    assert_eq!(key.code, KeyCode::Right);

    let key = command_to_key_event("Up");
    assert_eq!(key.code, KeyCode::Up);

    let key = command_to_key_event("Down");
    assert_eq!(key.code, KeyCode::Down);
}

#[test]
fn test_command_to_key_backspace() {
    let key = command_to_key_event("Backspace");
    assert_eq!(key.code, KeyCode::Backspace);
}

#[test]
fn test_command_to_key_unknown_multichar() {
    // Unknown multi-character strings that aren't recognized special keys
    // fall back to space (via parse_helix_key_string returning None)
    let key = command_to_key_event("unknown");
    assert_eq!(key.code, KeyCode::Char(' '));
}

#[test]
fn test_command_to_key_empty() {
    // Empty string falls back to space character
    let key = command_to_key_event("");
    assert_eq!(key.code, KeyCode::Char(' '));
}

// ============================================================================
// parse_helix_key_string tests
// ============================================================================

#[test]
fn test_parse_single_lowercase() {
    let key = parse_helix_key_string("c").unwrap();
    assert_eq!(key.code, KeyCode::Char('c'));
    assert_eq!(key.modifiers, KeyModifiers::NONE);
}

#[test]
fn test_parse_single_uppercase() {
    let key = parse_helix_key_string("C").unwrap();
    assert_eq!(key.code, KeyCode::Char('C'));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn test_parse_alt_lowercase() {
    let key = parse_helix_key_string("Alt-c").unwrap();
    assert_eq!(key.code, KeyCode::Char('c'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(!key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn test_parse_alt_uppercase() {
    let key = parse_helix_key_string("Alt-C").unwrap();
    assert_eq!(key.code, KeyCode::Char('C'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn test_parse_alt_short_form() {
    let key = parse_helix_key_string("A-c").unwrap();
    assert_eq!(key.code, KeyCode::Char('c'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(!key.modifiers.contains(KeyModifiers::SHIFT));

    let key = parse_helix_key_string("A-C").unwrap();
    assert_eq!(key.code, KeyCode::Char('C'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn test_parse_ctrl_lowercase() {
    let key = parse_helix_key_string("Ctrl-c").unwrap();
    assert_eq!(key.code, KeyCode::Char('c'));
    assert!(key.modifiers.contains(KeyModifiers::CONTROL));
    assert!(!key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn test_parse_ctrl_short_form() {
    let key = parse_helix_key_string("C-w").unwrap();
    assert_eq!(key.code, KeyCode::Char('w'));
    assert!(key.modifiers.contains(KeyModifiers::CONTROL));
}

#[test]
fn test_parse_shift_explicit() {
    let key = parse_helix_key_string("Shift-a").unwrap();
    assert_eq!(key.code, KeyCode::Char('a'));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));

    let key = parse_helix_key_string("S-a").unwrap();
    assert_eq!(key.code, KeyCode::Char('a'));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn test_parse_special_keys() {
    let key = parse_helix_key_string("Escape").unwrap();
    assert_eq!(key.code, KeyCode::Esc);

    let key = parse_helix_key_string("Backspace").unwrap();
    assert_eq!(key.code, KeyCode::Backspace);

    let key = parse_helix_key_string("Enter").unwrap();
    assert_eq!(key.code, KeyCode::Enter);

    let key = parse_helix_key_string("Tab").unwrap();
    assert_eq!(key.code, KeyCode::Tab);

    let key = parse_helix_key_string("Space").unwrap();
    assert_eq!(key.code, KeyCode::Char(' '));
}

#[test]
fn test_parse_arrow_keys() {
    let key = parse_helix_key_string("Left").unwrap();
    assert_eq!(key.code, KeyCode::Left);

    let key = parse_helix_key_string("Right").unwrap();
    assert_eq!(key.code, KeyCode::Right);

    let key = parse_helix_key_string("Up").unwrap();
    assert_eq!(key.code, KeyCode::Up);

    let key = parse_helix_key_string("Down").unwrap();
    assert_eq!(key.code, KeyCode::Down);
}

#[test]
fn test_parse_special_keys_short_form() {
    let key = parse_helix_key_string("esc").unwrap();
    assert_eq!(key.code, KeyCode::Esc);

    let key = parse_helix_key_string("bs").unwrap();
    assert_eq!(key.code, KeyCode::Backspace);

    let key = parse_helix_key_string("ret").unwrap();
    assert_eq!(key.code, KeyCode::Enter);
}

#[test]
fn test_parse_function_keys() {
    let key = parse_helix_key_string("F1").unwrap();
    assert_eq!(key.code, KeyCode::F(1));

    let key = parse_helix_key_string("F12").unwrap();
    assert_eq!(key.code, KeyCode::F(12));
}

#[test]
fn test_parse_navigation_keys() {
    let key = parse_helix_key_string("Home").unwrap();
    assert_eq!(key.code, KeyCode::Home);

    let key = parse_helix_key_string("End").unwrap();
    assert_eq!(key.code, KeyCode::End);

    let key = parse_helix_key_string("PageUp").unwrap();
    assert_eq!(key.code, KeyCode::PageUp);

    let key = parse_helix_key_string("PageDown").unwrap();
    assert_eq!(key.code, KeyCode::PageDown);

    let key = parse_helix_key_string("Insert").unwrap();
    assert_eq!(key.code, KeyCode::Insert);

    let key = parse_helix_key_string("Delete").unwrap();
    assert_eq!(key.code, KeyCode::Delete);
}

#[test]
fn test_parse_modifier_with_special_key() {
    let key = parse_helix_key_string("Ctrl-Space").unwrap();
    assert_eq!(key.code, KeyCode::Char(' '));
    assert!(key.modifiers.contains(KeyModifiers::CONTROL));

    let key = parse_helix_key_string("Alt-Enter").unwrap();
    assert_eq!(key.code, KeyCode::Enter);
    assert!(key.modifiers.contains(KeyModifiers::ALT));
}

#[test]
fn test_parse_special_characters() {
    let key = parse_helix_key_string("-").unwrap();
    assert_eq!(key.code, KeyCode::Char('-'));

    let key = parse_helix_key_string("Alt--").unwrap();
    assert_eq!(key.code, KeyCode::Char('-'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));

    let key = parse_helix_key_string("Alt-,").unwrap();
    assert_eq!(key.code, KeyCode::Char(','));
    assert!(key.modifiers.contains(KeyModifiers::ALT));

    let key = parse_helix_key_string("Alt-.").unwrap();
    assert_eq!(key.code, KeyCode::Char('.'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));

    let key = parse_helix_key_string("Alt-;").unwrap();
    assert_eq!(key.code, KeyCode::Char(';'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));

    let key = parse_helix_key_string("Alt-`").unwrap();
    assert_eq!(key.code, KeyCode::Char('`'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));

    let key = parse_helix_key_string("Alt-*").unwrap();
    assert_eq!(key.code, KeyCode::Char('*'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
}

#[test]
fn test_parse_empty_string() {
    assert!(parse_helix_key_string("").is_none());
}

#[test]
fn test_parse_invalid_string() {
    // Multi-character strings that are not recognized special keys
    assert!(parse_helix_key_string("unknown").is_none());
    assert!(parse_helix_key_string("xyz").is_none());
}

#[test]
fn test_parse_modifier_only() {
    // Modifier prefix without key should return None
    assert!(parse_helix_key_string("Alt-").is_none());
    assert!(parse_helix_key_string("Ctrl-").is_none());
    assert!(parse_helix_key_string("A-").is_none());
    assert!(parse_helix_key_string("C-").is_none());
}

#[test]
fn test_parse_case_insensitive_modifiers() {
    let key1 = parse_helix_key_string("Alt-c").unwrap();
    let key2 = parse_helix_key_string("alt-c").unwrap();
    assert_eq!(key1.code, key2.code);
    assert_eq!(key1.modifiers, key2.modifiers);

    let key1 = parse_helix_key_string("Ctrl-c").unwrap();
    let key2 = parse_helix_key_string("ctrl-c").unwrap();
    assert_eq!(key1.code, key2.code);
    assert_eq!(key1.modifiers, key2.modifiers);
}

#[test]
fn test_parse_case_insensitive_special_keys() {
    let key1 = parse_helix_key_string("Escape").unwrap();
    let key2 = parse_helix_key_string("escape").unwrap();
    assert_eq!(key1.code, key2.code);

    let key1 = parse_helix_key_string("ESCAPE").unwrap();
    assert_eq!(key1.code, KeyCode::Esc);
}

// ============================================================================
// Alt command mapping tests (map_single_key_command with ALT modifier)
// ============================================================================

#[test]
fn test_map_alt_c_copy_selection_prev() {
    // Alt-C: copy_selection_on_prev_line
    assert_eq!(
        map_single_key_command('C', KeyModifiers::ALT | KeyModifiers::SHIFT),
        Some(CMD_COPY_SELECTION_PREV)
    );
}

#[test]
fn test_map_alt_j_join_selections_space() {
    // Alt-J: join_selections_space
    assert_eq!(
        map_single_key_command('J', KeyModifiers::ALT | KeyModifiers::SHIFT),
        Some(CMD_JOIN_SELECTIONS_SPACE)
    );
}

#[test]
fn test_map_alt_k_remove_matching() {
    // Alt-K: remove_selections
    assert_eq!(
        map_single_key_command('K', KeyModifiers::ALT | KeyModifiers::SHIFT),
        Some(CMD_REMOVE_MATCHING)
    );
}

#[test]
fn test_map_alt_s_split_on_newlines() {
    // Alt-s: split_selection_on_newline (lowercase)
    assert_eq!(
        map_single_key_command('s', KeyModifiers::ALT),
        Some(CMD_SPLIT_SELECTION_NEWLINES)
    );
}

#[test]
fn test_map_alt_x_shrink_to_line_bounds() {
    // Alt-x: shrink_to_line_bounds (lowercase)
    assert_eq!(
        map_single_key_command('x', KeyModifiers::ALT),
        Some(CMD_SHRINK_TO_LINE_BOUNDS)
    );
}

#[test]
fn test_map_alt_comma_remove_primary() {
    // Alt-,: remove_primary_selection
    assert_eq!(
        map_single_key_command(',', KeyModifiers::ALT),
        Some(CMD_REMOVE_PRIMARY_SELECTION)
    );
}

#[test]
fn test_map_alt_minus_merge_selections() {
    // Alt--: merge_selections
    assert_eq!(
        map_single_key_command('-', KeyModifiers::ALT),
        Some(CMD_MERGE_SELECTIONS)
    );
}

#[test]
fn test_map_alt_underscore_merge_consecutive() {
    // Alt-_: merge_consecutive_selections
    assert_eq!(
        map_single_key_command('_', KeyModifiers::ALT | KeyModifiers::SHIFT),
        Some(CMD_MERGE_CONSECUTIVE)
    );
}

#[test]
fn test_map_alt_dot_repeat_last_motion() {
    // Alt-.: repeat_last_motion
    assert_eq!(
        map_single_key_command('.', KeyModifiers::ALT),
        Some(CMD_REPEAT_LAST_MOTION)
    );
}

#[test]
fn test_map_alt_backtick_switch_uppercase() {
    // Alt-`: switch_to_uppercase
    assert_eq!(
        map_single_key_command('`', KeyModifiers::ALT),
        Some(CMD_SWITCH_TO_UPPERCASE)
    );
}

#[test]
fn test_map_alt_semicolon_flip_selections() {
    // Alt-;: flip_selections
    assert_eq!(
        map_single_key_command(';', KeyModifiers::ALT),
        Some(CMD_FLIP_SELECTIONS)
    );
}

#[test]
fn test_map_alt_asterisk_search_selection() {
    // Alt-*: search_selection
    assert_eq!(
        map_single_key_command('*', KeyModifiers::ALT | KeyModifiers::SHIFT),
        Some(CMD_SEARCH_SELECTION)
    );
}

#[test]
fn test_alt_does_not_affect_normal_commands() {
    // Without Alt, 'x' should be select_line, not shrink_to_line_bounds
    assert_eq!(
        map_single_key_command('x', KeyModifiers::NONE),
        Some(CMD_SELECT_LINE)
    );

    // Without Alt, 's' should be select_regex, not split_on_newlines
    assert_eq!(
        map_single_key_command('s', KeyModifiers::NONE),
        Some(CMD_SELECT_REGEX)
    );

    // Without Alt, ';' should be collapse_selection, not flip_selections
    assert_eq!(
        map_single_key_command(';', KeyModifiers::NONE),
        Some(CMD_COLLAPSE_SELECTION)
    );

    // Without Alt, 'C' (Shift) should be copy_selection_next, not copy_selection_prev
    assert_eq!(
        map_single_key_command('C', KeyModifiers::SHIFT),
        Some(CMD_COPY_SELECTION_NEXT)
    );

    // Without Alt, 'J' (Shift) should be join_lines, not join_selections_space
    assert_eq!(
        map_single_key_command('J', KeyModifiers::SHIFT),
        Some(CMD_JOIN_LINES)
    );

    // Without Alt, 'K' (Shift) should be keep_matching, not remove_matching
    assert_eq!(
        map_single_key_command('K', KeyModifiers::SHIFT),
        Some(CMD_KEEP_MATCHING)
    );
}

#[test]
fn test_parse_all_alt_commands_from_plan() {
    // Alt-C: copy_selection_on_prev_line
    let key = parse_helix_key_string("Alt-C").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
    assert_eq!(key.code, KeyCode::Char('C'));

    // Alt-J: join_selections_space
    let key = parse_helix_key_string("Alt-J").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
    assert_eq!(key.code, KeyCode::Char('J'));

    // Alt-K: remove_selections
    let key = parse_helix_key_string("Alt-K").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
    assert_eq!(key.code, KeyCode::Char('K'));

    // Alt-s: split_selection_on_newline (lowercase)
    let key = parse_helix_key_string("Alt-s").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(!key.modifiers.contains(KeyModifiers::SHIFT));
    assert_eq!(key.code, KeyCode::Char('s'));

    // Alt-x: shrink_to_line_bounds (lowercase)
    let key = parse_helix_key_string("Alt-x").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert!(!key.modifiers.contains(KeyModifiers::SHIFT));
    assert_eq!(key.code, KeyCode::Char('x'));

    // Alt-,: remove_primary_selection
    let key = parse_helix_key_string("Alt-,").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char(','));

    // Alt--: merge_selections
    let key = parse_helix_key_string("Alt--").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char('-'));

    // Alt-_: merge_consecutive_selections
    let key = parse_helix_key_string("Alt-_").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char('_'));

    // Alt-.: repeat_last_motion
    let key = parse_helix_key_string("Alt-.").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char('.'));

    // Alt-`: switch_to_uppercase
    let key = parse_helix_key_string("Alt-`").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char('`'));

    // Alt-;: flip_selections
    let key = parse_helix_key_string("Alt-;").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char(';'));

    // Alt-*: search_selection
    let key = parse_helix_key_string("Alt-*").unwrap();
    assert!(key.modifiers.contains(KeyModifiers::ALT));
    assert_eq!(key.code, KeyCode::Char('*'));
}
