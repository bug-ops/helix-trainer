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
pub fn map_single_key_command(c: char, modifiers: KeyModifiers) -> Option<&'static str> {
    let is_shift = modifiers.contains(KeyModifiers::SHIFT);

    match (c, is_shift) {
        // Movement
        ('h', false) => Some(CMD_MOVE_LEFT),
        ('j', false) => Some(CMD_MOVE_DOWN),
        ('k', false) => Some(CMD_MOVE_UP),
        ('l', false) => Some(CMD_MOVE_RIGHT),

        // Word movement
        ('w', false) => Some(CMD_MOVE_WORD_FORWARD),
        ('b', false) => Some(CMD_MOVE_WORD_BACKWARD),
        ('e', false) => Some(CMD_MOVE_WORD_END),

        // WORD movement (uppercase)
        ('W', _) => Some(CMD_MOVE_LONG_WORD_FORWARD),
        ('B', _) => Some(CMD_MOVE_LONG_WORD_BACKWARD),
        ('E', _) => Some(CMD_MOVE_LONG_WORD_END),

        // Selection
        ('x', false) => Some(CMD_SELECT_LINE),
        ('X', _) => Some(CMD_EXTEND_LINE),
        ('%', _) => Some(CMD_SELECT_ALL),
        (';', false) => Some(CMD_COLLAPSE_SELECTION),
        ('v', false) => Some(CMD_SELECT_MODE),

        // Editing
        ('d', false) => Some(CMD_DELETE_SELECTION),
        ('c', false) => Some(CMD_CHANGE),
        ('i', false) => Some(CMD_INSERT),
        ('a', false) => Some(CMD_APPEND),
        ('I', _) => Some(CMD_INSERT_LINE_START),
        ('A', _) => Some(CMD_APPEND_LINE_END),
        ('o', false) => Some(CMD_OPEN_BELOW),
        ('O', _) => Some(CMD_OPEN_ABOVE),
        ('J', _) => Some(CMD_JOIN_LINES),

        // Indentation
        ('>', _) => Some(CMD_INDENT),
        ('<', _) => Some(CMD_DEDENT),

        // Case
        ('~', _) => Some(CMD_SWITCH_CASE),
        ('`', _) => Some(CMD_SWITCH_CASE_ALT),

        // Clipboard
        ('y', false) => Some(CMD_YANK),
        ('p', false) => Some(CMD_PASTE_AFTER),
        ('P', _) => Some(CMD_PASTE_BEFORE),

        // Undo/Redo
        ('u', false) => Some(CMD_UNDO),
        ('U', _) => Some(CMD_REDO),

        // Repeat
        ('.', _) => Some(CMD_REPEAT),

        // Search
        ('/', _) => Some(CMD_SEARCH_FORWARD),
        ('?', _) => Some(CMD_SEARCH_BACKWARD),
        ('n', false) => Some(CMD_SEARCH_NEXT),
        ('N', _) => Some(CMD_SEARCH_PREV),
        ('*', _) => Some(CMD_SEARCH_WORD),

        // Selection manipulation
        ('s', false) => Some(CMD_SELECT_REGEX),
        ('S', _) => Some(CMD_SPLIT_SELECTION),
        ('&', _) => Some(CMD_ALIGN_SELECTIONS),
        ('_', _) => Some(CMD_TRIM_SELECTIONS),
        ('C', _) => Some(CMD_COPY_SELECTION_NEXT),
        ('K', _) => Some(CMD_KEEP_MATCHING),

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
/// ```
pub fn command_to_key_event(command: &str) -> KeyEvent {
    // Handle single character commands
    if command.len() == 1 {
        let c = command.chars().next().unwrap();
        // Check if it's an uppercase letter (implies Shift)
        let modifiers = if c.is_ascii_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        return KeyEvent::new(KeyCode::Char(c), modifiers);
    }

    // Handle special command strings
    match command {
        "Escape" => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        "Left" => KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        "Right" => KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        "Up" => KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        "Down" => KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        "Backspace" => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        _ => {
            // Default: treat first char as the key
            let c = command.chars().next().unwrap_or(' ');
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
        }
    }
}
