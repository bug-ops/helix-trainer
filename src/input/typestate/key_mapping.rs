//! Helper functions for key-to-command mapping
//!
//! Contains utilities for mapping key events to Helix commands and handling
//! insert mode input.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;

/// Parse Helix-style key string to KeyEvent
///
/// Supports formats:
/// - "Alt-C" / "A-C" -> Alt + Shift + c
/// - "Alt-c" / "A-c" -> Alt + c
/// - "Ctrl-c" / "C-c" -> Ctrl + c
/// - "C" -> Shift + c (uppercase = shift)
/// - "c" -> c (lowercase = no shift)
/// - Special keys: "Escape", "Backspace", "Enter", "Tab", "Space"
/// - Arrow keys: "Left", "Right", "Up", "Down"
///
/// Reference: Helix uses format like "A-c" for Alt-c, "C-w" for Ctrl-w
pub fn parse_helix_key_string(s: &str) -> Option<KeyEvent> {
    if s.is_empty() {
        return None;
    }

    let mut modifiers = KeyModifiers::NONE;
    let mut remaining = s;

    // Parse modifier prefixes (case-insensitive for modifier names)
    loop {
        if remaining.starts_with("Alt-") || remaining.starts_with("alt-") {
            modifiers |= KeyModifiers::ALT;
            remaining = &remaining[4..];
        } else if remaining.starts_with("A-") {
            modifiers |= KeyModifiers::ALT;
            remaining = &remaining[2..];
        } else if remaining.starts_with("Ctrl-") || remaining.starts_with("ctrl-") {
            modifiers |= KeyModifiers::CONTROL;
            remaining = &remaining[5..];
        } else if remaining.starts_with("C-") {
            modifiers |= KeyModifiers::CONTROL;
            remaining = &remaining[2..];
        } else if remaining.starts_with("Shift-") || remaining.starts_with("shift-") {
            modifiers |= KeyModifiers::SHIFT;
            remaining = &remaining[6..];
        } else if remaining.starts_with("S-") {
            modifiers |= KeyModifiers::SHIFT;
            remaining = &remaining[2..];
        } else {
            break;
        }
    }

    if remaining.is_empty() {
        return None;
    }

    // Parse the key part
    let key_code = parse_key_code(remaining, &mut modifiers)?;

    Some(KeyEvent::new(key_code, modifiers))
}

/// Parse the key code from a string, updating modifiers for uppercase letters
fn parse_key_code(s: &str, modifiers: &mut KeyModifiers) -> Option<KeyCode> {
    // Single character (byte length check is correct for ASCII-only key inputs)
    if s.len() == 1 {
        let c = s.chars().next()?;
        // Uppercase ASCII letter implies Shift modifier
        if c.is_ascii_uppercase() {
            *modifiers |= KeyModifiers::SHIFT;
        }
        return Some(KeyCode::Char(c));
    }

    // Special keys (case-insensitive comparison without heap allocation)
    if s.eq_ignore_ascii_case("escape") || s.eq_ignore_ascii_case("esc") {
        return Some(KeyCode::Esc);
    }
    if s.eq_ignore_ascii_case("backspace") || s.eq_ignore_ascii_case("bs") {
        return Some(KeyCode::Backspace);
    }
    if s.eq_ignore_ascii_case("enter")
        || s.eq_ignore_ascii_case("return")
        || s.eq_ignore_ascii_case("ret")
    {
        return Some(KeyCode::Enter);
    }
    if s.eq_ignore_ascii_case("tab") {
        return Some(KeyCode::Tab);
    }
    if s.eq_ignore_ascii_case("space") {
        return Some(KeyCode::Char(' '));
    }
    if s.eq_ignore_ascii_case("left") {
        return Some(KeyCode::Left);
    }
    if s.eq_ignore_ascii_case("right") {
        return Some(KeyCode::Right);
    }
    if s.eq_ignore_ascii_case("up") {
        return Some(KeyCode::Up);
    }
    if s.eq_ignore_ascii_case("down") {
        return Some(KeyCode::Down);
    }
    if s.eq_ignore_ascii_case("home") {
        return Some(KeyCode::Home);
    }
    if s.eq_ignore_ascii_case("end") {
        return Some(KeyCode::End);
    }
    if s.eq_ignore_ascii_case("pageup") || s.eq_ignore_ascii_case("page_up") {
        return Some(KeyCode::PageUp);
    }
    if s.eq_ignore_ascii_case("pagedown") || s.eq_ignore_ascii_case("page_down") {
        return Some(KeyCode::PageDown);
    }
    if s.eq_ignore_ascii_case("insert") || s.eq_ignore_ascii_case("ins") {
        return Some(KeyCode::Insert);
    }
    if s.eq_ignore_ascii_case("delete") || s.eq_ignore_ascii_case("del") {
        return Some(KeyCode::Delete);
    }
    // Special characters by name
    if s.eq_ignore_ascii_case("minus") {
        return Some(KeyCode::Char('-'));
    }
    if s.eq_ignore_ascii_case("plus") {
        return Some(KeyCode::Char('+'));
    }

    // Function keys (F1-F12) with case-insensitive first character check
    let first_char = s.chars().next()?;
    if first_char.eq_ignore_ascii_case(&'f')
        && s.len() > 1
        && let Ok(n) = s[1..].parse::<u8>()
        && (1..=12).contains(&n)
    {
        return Some(KeyCode::F(n));
    }

    // Multi-character strings that are not recognized special keys
    None
}

/// Map a single-key command character to its command string
///
/// Returns None for invalid commands or prefix commands.
/// Handles SHIFT and ALT modifiers for proper command dispatch.
pub fn map_single_key_command(c: char, modifiers: KeyModifiers) -> Option<&'static str> {
    let is_shift = modifiers.contains(KeyModifiers::SHIFT);
    let is_alt = modifiers.contains(KeyModifiers::ALT);

    // Pattern: (char, is_shift, is_alt)
    match (c, is_shift, is_alt) {
        // Alt commands (must be checked first as they have highest specificity)
        // Note: Some terminals send uppercase chars without SHIFT modifier set,
        // so we match both ('C', _, true) and ('c', true, true) patterns for uppercase Alt commands
        ('c', false, true) => Some(CMD_CHANGE_SELECTION_NOYANK), // Alt-c
        ('C', _, true) => Some(CMD_COPY_SELECTION_PREV),         // Alt-C (with or without SHIFT flag)
        ('J', _, true) => Some(CMD_JOIN_SELECTIONS_SPACE),       // Alt-J
        ('K', _, true) => Some(CMD_REMOVE_MATCHING),             // Alt-K
        ('s', false, true) => Some(CMD_SPLIT_SELECTION_NEWLINES), // Alt-s
        ('x', false, true) => Some(CMD_SHRINK_TO_LINE_BOUNDS),   // Alt-x
        (',', false, true) => Some(CMD_REMOVE_PRIMARY_SELECTION), // Alt-,
        ('-', false, true) => Some(CMD_MERGE_SELECTIONS),        // Alt--
        ('_', _, true) => Some(CMD_MERGE_CONSECUTIVE),           // Alt-_
        ('.', false, true) => Some(CMD_REPEAT_LAST_MOTION),      // Alt-.
        ('`', false, true) => Some(CMD_SWITCH_TO_UPPERCASE),     // Alt-`
        (';', false, true) => Some(CMD_FLIP_SELECTIONS),         // Alt-;
        ('*', _, true) => Some(CMD_SEARCH_SELECTION),            // Alt-*

        // Movement (no Alt)
        ('h', false, false) => Some(CMD_MOVE_LEFT),
        ('j', false, false) => Some(CMD_MOVE_DOWN),
        ('k', false, false) => Some(CMD_MOVE_UP),
        ('l', false, false) => Some(CMD_MOVE_RIGHT),

        // Word movement (no Alt)
        ('w', false, false) => Some(CMD_MOVE_WORD_FORWARD),
        ('b', false, false) => Some(CMD_MOVE_WORD_BACKWARD),
        ('e', false, false) => Some(CMD_MOVE_WORD_END),

        // WORD movement (uppercase, no Alt)
        ('W', _, false) => Some(CMD_MOVE_LONG_WORD_FORWARD),
        ('B', _, false) => Some(CMD_MOVE_LONG_WORD_BACKWARD),
        ('E', _, false) => Some(CMD_MOVE_LONG_WORD_END),

        // Selection (no Alt)
        ('x', false, false) => Some(CMD_SELECT_LINE),
        ('X', _, false) => Some(CMD_EXTEND_LINE),
        ('%', _, false) => Some(CMD_SELECT_ALL),
        (';', false, false) => Some(CMD_COLLAPSE_SELECTION),
        ('v', false, false) => Some(CMD_SELECT_MODE),

        // Editing (no Alt)
        ('d', false, false) => Some(CMD_DELETE_SELECTION),
        ('c', false, false) => Some(CMD_CHANGE),
        ('i', false, false) => Some(CMD_INSERT),
        ('a', false, false) => Some(CMD_APPEND),
        ('I', _, false) => Some(CMD_INSERT_LINE_START),
        ('A', _, false) => Some(CMD_APPEND_LINE_END),
        ('o', false, false) => Some(CMD_OPEN_BELOW),
        ('O', _, false) => Some(CMD_OPEN_ABOVE),
        ('J', _, false) => Some(CMD_JOIN_LINES),

        // Indentation (no Alt)
        ('>', _, false) => Some(CMD_INDENT),
        ('<', _, false) => Some(CMD_DEDENT),

        // Case (no Alt)
        ('~', _, false) => Some(CMD_SWITCH_CASE),
        ('`', _, false) => Some(CMD_SWITCH_CASE_ALT),

        // Clipboard (no Alt)
        ('y', false, false) => Some(CMD_YANK),
        ('p', false, false) => Some(CMD_PASTE_AFTER),
        ('P', _, false) => Some(CMD_PASTE_BEFORE),

        // Undo/Redo (no Alt)
        ('u', false, false) => Some(CMD_UNDO),
        ('U', _, false) => Some(CMD_REDO),

        // Repeat (no Alt)
        ('.', _, false) => Some(CMD_REPEAT),

        // Search (no Alt)
        ('/', _, false) => Some(CMD_SEARCH_FORWARD),
        ('?', _, false) => Some(CMD_SEARCH_BACKWARD),
        ('n', false, false) => Some(CMD_SEARCH_NEXT),
        ('N', _, false) => Some(CMD_SEARCH_PREV),
        ('*', _, false) => Some(CMD_SEARCH_WORD),

        // Selection manipulation (no Alt)
        ('s', false, false) => Some(CMD_SELECT_REGEX),
        ('S', _, false) => Some(CMD_SPLIT_SELECTION),
        ('&', _, false) => Some(CMD_ALIGN_SELECTIONS),
        ('_', _, false) => Some(CMD_TRIM_SELECTIONS),
        ('C', _, false) => Some(CMD_COPY_SELECTION_NEXT),
        ('K', _, false) => Some(CMD_KEEP_MATCHING),

        _ => None,
    }
}

/// Check if a command is compatible with count prefix
pub fn is_count_compatible_command(c: char, modifiers: KeyModifiers) -> bool {
    let is_shift = modifiers.contains(KeyModifiers::SHIFT);

    matches!(
        (c, is_shift),
        // Movement commands support count
        ('h', false)
            | ('j', false)
            | ('k', false)
            | ('l', false)
            | ('w', false)
            | ('b', false)
            | ('e', false)
            | ('W', _)
            | ('B', _)
            | ('E', _)
            // Selection
            | ('x', false)
            | ('X', _)
            // Some editing commands
            | ('d', false)
            | ('c', false)
            | ('J', _)
            | ('>', _)
            | ('<', _)
            // Undo/Redo
            | ('u', false)
            | ('U', _)
            // Search navigation
            | ('n', false)
            | ('N', _)
    )
}

/// Map a key event to a Helix command string in normal mode
///
/// This is a convenience function for use in handlers that maps
/// simple key events to their corresponding Helix command strings.
/// It handles single-key commands but not multi-key sequences
/// (those require the InputStateMachine).
pub fn map_key_to_helix_command(key: KeyEvent) -> Option<&'static str> {
    match key.code {
        KeyCode::Char(c) => map_single_key_command(c, key.modifiers),
        KeyCode::Esc => Some(CMD_ESCAPE),
        KeyCode::Backspace => Some(CMD_BACKSPACE),
        KeyCode::Left => Some(CMD_ARROW_LEFT),
        KeyCode::Right => Some(CMD_ARROW_RIGHT),
        KeyCode::Up => Some(CMD_ARROW_UP),
        KeyCode::Down => Some(CMD_ARROW_DOWN),
        _ => None,
    }
}

/// Handle insert mode input and convert to command string
///
/// Returns the character/command to insert or execute in insert mode.
/// This handles printable characters, escape, backspace, and arrow keys.
pub fn handle_insert_mode_input(key: KeyEvent) -> Option<Cow<'static, str>> {
    match key.code {
        KeyCode::Char(c) => Some(Cow::Owned(c.to_string())),
        KeyCode::Esc => Some(Cow::Borrowed(CMD_ESCAPE)),
        KeyCode::Backspace => Some(Cow::Borrowed(CMD_BACKSPACE)),
        KeyCode::Enter => Some(Cow::Borrowed("\n")),
        KeyCode::Tab => Some(Cow::Borrowed("\t")),
        KeyCode::Left => Some(Cow::Borrowed(CMD_ARROW_LEFT)),
        KeyCode::Right => Some(Cow::Borrowed(CMD_ARROW_RIGHT)),
        KeyCode::Up => Some(Cow::Borrowed(CMD_ARROW_UP)),
        KeyCode::Down => Some(Cow::Borrowed(CMD_ARROW_DOWN)),
        _ => None,
    }
}

/// Convert a command string to a KeyEvent for the input state machine
///
/// This is a helper function to bridge the gap between command strings
/// coming from the input layer and the KeyEvent-based state machine.
/// Uses `parse_helix_key_string()` for parsing, which supports modifiers
/// like Alt-C, Ctrl-w, etc.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::input::typestate::command_to_key_event;
///
/// let key = command_to_key_event("h");
/// assert_eq!(key.code, KeyCode::Char('h'));
///
/// let key = command_to_key_event("Escape");
/// assert_eq!(key.code, KeyCode::Esc);
///
/// let key = command_to_key_event("Alt-C");
/// assert!(key.modifiers.contains(KeyModifiers::ALT));
/// assert!(key.modifiers.contains(KeyModifiers::SHIFT));
/// ```
pub fn command_to_key_event(command: &str) -> KeyEvent {
    parse_helix_key_string(command)
        .unwrap_or_else(|| KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
}
