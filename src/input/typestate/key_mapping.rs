//! Helper functions for key-to-command mapping
//!
//! Contains utilities for mapping key events to Helix commands and handling
//! insert mode input.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::{
    CMD_ALIGN_SELECTIONS, CMD_APPEND, CMD_APPEND_LINE_END, CMD_ARROW_DOWN, CMD_ARROW_LEFT,
    CMD_ARROW_RIGHT, CMD_ARROW_UP, CMD_BACKSPACE, CMD_CHANGE, CMD_CHANGE_SELECTION_NOYANK,
    CMD_COLLAPSE_SELECTION, CMD_COPY_SELECTION_NEXT, CMD_COPY_SELECTION_PREV, CMD_DEDENT,
    CMD_DELETE_SELECTION, CMD_DELETE_SELECTION_NOYANK, CMD_ESCAPE, CMD_EXTEND_LINE,
    CMD_FLIP_SELECTIONS, CMD_HALF_PAGE_DOWN, CMD_HALF_PAGE_UP, CMD_INDENT, CMD_INSERT,
    CMD_INSERT_ALT_BACKSPACE, CMD_INSERT_DELETE_CHAR_FORWARD, CMD_INSERT_DELETE_KEY,
    CMD_INSERT_DELETE_TO_LINE_END, CMD_INSERT_DELETE_TO_LINE_START,
    CMD_INSERT_DELETE_WORD_BACKWARD, CMD_INSERT_DELETE_WORD_FORWARD, CMD_INSERT_LINE_START,
    CMD_INSERT_TAB, CMD_JOIN_LINES, CMD_JOIN_SELECTIONS_SPACE, CMD_KEEP_MATCHING,
    CMD_KEEP_PRIMARY_SELECTION, CMD_MERGE_CONSECUTIVE, CMD_MERGE_SELECTIONS, CMD_MOVE_DOWN,
    CMD_MOVE_LEFT, CMD_MOVE_LONG_WORD_BACKWARD, CMD_MOVE_LONG_WORD_END, CMD_MOVE_LONG_WORD_FORWARD,
    CMD_MOVE_RIGHT, CMD_MOVE_UP, CMD_MOVE_WORD_BACKWARD, CMD_MOVE_WORD_END, CMD_MOVE_WORD_FORWARD,
    CMD_OPEN_ABOVE, CMD_OPEN_BELOW, CMD_PAGE_DOWN, CMD_PAGE_UP, CMD_PASTE_AFTER, CMD_PASTE_BEFORE,
    CMD_REDO, CMD_REMOVE_MATCHING, CMD_REMOVE_PRIMARY_SELECTION, CMD_REPEAT,
    CMD_REPEAT_LAST_MOTION, CMD_REPLACE_WITH_YANKED, CMD_SEARCH_BACKWARD, CMD_SEARCH_NEXT,
    CMD_SEARCH_PREV, CMD_SEARCH_SELECTION, CMD_SEARCH_WORD, CMD_SELECT_ALL, CMD_SELECT_LINE,
    CMD_SELECT_MODE, CMD_SHRINK_TO_LINE_BOUNDS, CMD_SPLIT_SELECTION_NEWLINES, CMD_SWITCH_CASE,
    CMD_SWITCH_CASE_ALT, CMD_SWITCH_TO_UPPERCASE, CMD_TOGGLE_COMMENTS, CMD_TRIM_SELECTIONS,
    CMD_UNDO, CMD_YANK,
};

/// Map macOS composed Unicode characters to Alt + base key
///
/// On macOS, when terminal doesn't support kitty keyboard protocol,
/// Option+key produces composed Unicode characters instead of Alt modifier.
/// This function converts them back to proper Alt + key combinations.
///
/// Common macOS Option compositions:
/// - Option+Shift+C = Ç
/// - Option+j = ∆, Option+k = ˚
/// - Option+x = ≈, Option+; = …, Option+, = ≤
pub(crate) fn map_macos_composed_char(ch: char) -> Option<(char, KeyModifiers)> {
    match ch {
        // Alt+lowercase
        // NOTE: ç and ß are intentionally NOT mapped here (issue #381), for
        // the same reason backtick isn't (see the NOTE below): 'ç' is a
        // plain, directly-typeable key on French AZERTY and Portuguese/
        // Brazilian ABNT layouts, and 'ß' is a plain key on German QWERTZ.
        // Rewriting them unconditionally to Alt-c/Alt-s made those real
        // keystrokes indistinguishable from actual Alt-c/Alt-s on macOS's US
        // Option-composition artifacts. This makes Alt-c/Alt-s unreachable
        // via Option-key composition on macOS terminals without kitty
        // keyboard protocol support - tracked separately as #386.
        //
        // 'Ç', '¬', and '˚' below have the same class of collision on other
        // layouts (e.g. Ç is a plain Shift+key on Brazilian ABNT2 and
        // Portuguese; ¬ is Shift-` on British PC) and are NOT addressed by
        // this fix - left as-is, out of scope for #381.
        '∆' => Some(('j', KeyModifiers::ALT)),
        '˚' => Some(('k', KeyModifiers::ALT)),
        '≈' => Some(('x', KeyModifiers::ALT)),
        '…' => Some((';', KeyModifiers::ALT)),
        '≤' => Some((',', KeyModifiers::ALT)),
        '–' => Some(('-', KeyModifiers::ALT)),
        '≥' => Some(('.', KeyModifiers::ALT)),
        // NOTE: backtick is intentionally NOT mapped here. Unlike every other
        // entry in this table, the macOS Option-` dead-key composition
        // produces the same plain '`' char as a real, directly-typeable key
        // (unmodified backtick). Rewriting it unconditionally to ALT made a
        // plain backtick keystroke indistinguishable from real Alt-backtick,
        // silently breaking `switch_case_alt` (see issue #377).
        // Alt+Shift (uppercase or shifted symbols)
        'Ç' => Some(('C', KeyModifiers::ALT)), // Alt-C (copy_selection_prev)
        // Corrected per issue #382: on the standard macOS US keyboard layout,
        // Option+Shift+G produces U+02DD (˝, double acute accent), not
        // Option+Shift+J (which produces Ô, U+00D4, not otherwise mapped
        // here). Verified against the actual "U.S." system keylayout. No
        // stock command is bound to Alt-G (`map_single_key_command` has no
        // ('G', _, true) arm), so this is currently only useful for the
        // custom-keymap overlay.
        '˝' => Some(('G', KeyModifiers::ALT)),
        '\u{F8FF}' => Some(('K', KeyModifiers::ALT)), // Alt-K (remove_matching) - Apple logo
        // Corrected per issue #382: Option+Shift+, produces U+00AF (¯,
        // macron), not Option+Shift+- (which produces U+2014, em dash, not
        // otherwise mapped here). Verified against the actual "U.S." system
        // keylayout. Mapped to the *shifted* char '<' (Shift+,), matching
        // this table's convention for shifted compositions ('Ç'->'C',
        // U+F8FF->'K') - mapping to bare ',' would alias with the existing
        // '≤' => (',', ALT) entry above, making Opt+, and Opt+Shift+,
        // indistinguishable.
        '¯' => Some(('<', KeyModifiers::ALT)),
        '¬' => Some(('l', KeyModifiers::ALT)), // Alt-l if needed
        _ => None,
    }
}

/// Normalize a `KeyEvent` to canonical form (like Helix does)
///
/// This ensures consistent representation regardless of terminal behavior:
/// - macOS composed chars (Ç, ˝, ...) → base char + ALT modifier
/// - lowercase + SHIFT → uppercase (SHIFT removed)
/// - uppercase + SHIFT → uppercase (SHIFT removed, already uppercase)
/// - Shift-`/` → `?`
///
/// This matches Helix's normalization: "C-S-r and C-R are represented by equal `KeyEvents`"
pub fn normalize_key_event(key: KeyEvent) -> KeyEvent {
    // First, try to map macOS composed Unicode characters to Alt combinations
    if let KeyCode::Char(ch) = key.code
        && let Some((base_char, alt_modifier)) = map_macos_composed_char(ch)
    {
        let mut modifiers = key.modifiers;
        modifiers.insert(alt_modifier);
        // Remove SHIFT if the base char is uppercase (it's already implied)
        if base_char.is_ascii_uppercase() {
            modifiers.remove(KeyModifiers::SHIFT);
        }
        return KeyEvent::new(KeyCode::Char(base_char), modifiers);
    }

    // Some terminals report `?` as Shift-`/`
    if key.code == KeyCode::Char('/') && key.modifiers == KeyModifiers::SHIFT {
        return KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
    }

    // Standard normalization for SHIFT + letter
    match key.code {
        KeyCode::Char(ch)
            if ch.is_ascii_lowercase() && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            let mut modifiers = key.modifiers;
            modifiers.remove(KeyModifiers::SHIFT);
            KeyEvent::new(KeyCode::Char(ch.to_ascii_uppercase()), modifiers)
        }
        KeyCode::Char(ch)
            if ch.is_ascii_uppercase() && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            // Already uppercase, just remove redundant SHIFT
            let mut modifiers = key.modifiers;
            modifiers.remove(KeyModifiers::SHIFT);
            KeyEvent::new(KeyCode::Char(ch), modifiers)
        }
        _ => key,
    }
}

/// Parse Helix-style key string to `KeyEvent`
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
    // Single character - counts chars, not bytes, so a non-ASCII char (which
    // is multi-byte in UTF-8) is correctly rejected as "not a single key"
    // rather than silently falling through to the space fallback.
    let mut chars = s.chars();
    if let Some(c) = chars.next()
        && chars.next().is_none()
    {
        // Uppercase ASCII letter implies Shift modifier
        if c.is_ascii_uppercase() {
            *modifiers |= KeyModifiers::SHIFT;
        }
        return Some(KeyCode::Char(c));
    }

    named_key_code(s)
}

/// Resolve a multi-character named key (`"esc"`, `"Enter"`, `"F1"`, ...) to
/// its `KeyCode`.
///
/// This is the single source of truth for the named-key vocabulary this
/// crate understands: [`parse_key_code`] uses it to parse Helix key
/// strings, and `CanonicalKeys::tokens` (`src/input/keymap/keys.rs`) uses
/// [`is_named_key`] (backed by this same function) to recognize a whole
/// canonical-key string as one token rather than splitting it character by
/// character. The two must never drift, since a canonical key produced by
/// the tokenizer is later re-parsed by `parse_helix_key_string`.
pub(crate) fn named_key_code(s: &str) -> Option<KeyCode> {
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

/// Whether `s`, taken as a whole string, is a recognized named key (as
/// opposed to a sequence of single-character tokens). See [`named_key_code`].
pub(crate) fn is_named_key(s: &str) -> bool {
    named_key_code(s).is_some()
}

/// Modifier class of a key press, with `Ctrl` taking precedence over `Alt`.
///
/// Handlers `match` on this exhaustively so a key carrying a modifier can never
/// silently fall into a bare-key arm (the `Alt-s` -> `s` regex prompt bug, #487).
/// `SHIFT` alone is `Plain`: `normalize_key_event` already folds it into the
/// character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chord {
    /// No `Ctrl` or `Alt` modifier.
    Plain,
    /// `Alt` held, `Ctrl` not held.
    Alt,
    /// `Ctrl` held (with or without `Alt`).
    Ctrl,
}

impl Chord {
    /// Classify a modifier set.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::KeyModifiers;
    /// use helix_trainer::input::typestate::Chord;
    ///
    /// assert_eq!(Chord::of(KeyModifiers::SHIFT), Chord::Plain);
    /// assert_eq!(Chord::of(KeyModifiers::ALT), Chord::Alt);
    /// assert_eq!(Chord::of(KeyModifiers::ALT | KeyModifiers::CONTROL), Chord::Ctrl);
    /// ```
    pub fn of(modifiers: KeyModifiers) -> Self {
        if modifiers.contains(KeyModifiers::CONTROL) {
            Self::Ctrl
        } else if modifiers.contains(KeyModifiers::ALT) {
            Self::Alt
        } else {
            Self::Plain
        }
    }
}

/// Map a single-key command character to its command string
///
/// Returns None for invalid commands or prefix commands.
/// Handles SHIFT and ALT modifiers for proper command dispatch.
pub fn map_single_key_command(c: char, modifiers: KeyModifiers) -> Option<&'static str> {
    // Ctrl commands are checked first and return early: unlike Alt, CONTROL
    // is not otherwise reflected in the (char, is_shift, is_alt) match below,
    // so a Ctrl-modified char would otherwise silently fall into the bare-key
    // arms (e.g. Ctrl-d resolving to plain 'd' / delete_selection).
    if modifiers.contains(KeyModifiers::CONTROL) {
        return match c {
            'c' => Some(CMD_TOGGLE_COMMENTS), // Ctrl-c
            'b' => Some(CMD_PAGE_UP),         // Ctrl-b
            'f' => Some(CMD_PAGE_DOWN),       // Ctrl-f
            'u' => Some(CMD_HALF_PAGE_UP),    // Ctrl-u
            'd' => Some(CMD_HALF_PAGE_DOWN),  // Ctrl-d
            _ => None,
        };
    }

    let is_shift = modifiers.contains(KeyModifiers::SHIFT);
    let is_alt = modifiers.contains(KeyModifiers::ALT);

    // Pattern: (char, is_shift, is_alt)
    // Note: For uppercase letters and shifted punctuation, we use wildcard (_) for is_shift
    // because terminal behavior varies. The normalization function handles this at a
    // higher level, but this function remains flexible for direct testing.
    match (c, is_shift, is_alt) {
        // Alt commands (must be checked first as they have highest specificity)
        ('c', false, true) => Some(CMD_CHANGE_SELECTION_NOYANK), // Alt-c
        ('d', false, true) => Some(CMD_DELETE_SELECTION_NOYANK), // Alt-d
        ('C', _, true) => Some(CMD_COPY_SELECTION_PREV),         // Alt-C (any SHIFT)
        ('J', _, true) => Some(CMD_JOIN_SELECTIONS_SPACE),       // Alt-J (any SHIFT)
        ('K', _, true) => Some(CMD_REMOVE_MATCHING),             // Alt-K (any SHIFT)
        ('s', false, true) => Some(CMD_SPLIT_SELECTION_NEWLINES), // Alt-s
        ('x', false, true) => Some(CMD_SHRINK_TO_LINE_BOUNDS),   // Alt-x
        (',', false, true) => Some(CMD_REMOVE_PRIMARY_SELECTION), // Alt-,
        ('-', false, true) => Some(CMD_MERGE_SELECTIONS),        // Alt--
        ('_', _, true) => Some(CMD_MERGE_CONSECUTIVE),           // Alt-_ (SHIFT varies)
        ('.', false, true) => Some(CMD_REPEAT_LAST_MOTION),      // Alt-.
        ('`', false, true) => Some(CMD_SWITCH_TO_UPPERCASE),     // Alt-`
        (';', false, true) => Some(CMD_FLIP_SELECTIONS),         // Alt-;
        ('*', _, true) => Some(CMD_SEARCH_SELECTION),            // Alt-* (SHIFT varies)

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

        // Indentation (no Alt - Shift produces these chars but may or may not be flagged)
        ('>', _, false) => Some(CMD_INDENT),
        ('<', _, false) => Some(CMD_DEDENT),

        // Case (no Alt)
        ('~', _, false) => Some(CMD_SWITCH_CASE),
        ('`', false, false) => Some(CMD_SWITCH_CASE_ALT),

        // Clipboard (no Alt)
        ('y', false, false) => Some(CMD_YANK),
        ('p', false, false) => Some(CMD_PASTE_AFTER),
        ('P', _, false) => Some(CMD_PASTE_BEFORE),
        ('R', _, false) => Some(CMD_REPLACE_WITH_YANKED),

        // Undo/Redo (no Alt)
        ('u', false, false) => Some(CMD_UNDO),
        ('U', _, false) => Some(CMD_REDO),

        // Repeat (no Alt)
        ('.', false, false) => Some(CMD_REPEAT),

        // Search (no Alt)
        //
        // '/' is NOT mapped here: it opens the search prompt
        // (`SearchPromptPending`) via a dedicated `base.rs` arm (#504).
        ('?', _, false) => Some(CMD_SEARCH_BACKWARD),
        ('n', false, false) => Some(CMD_SEARCH_NEXT),
        ('N', _, false) => Some(CMD_SEARCH_PREV),
        ('*', _, false) => Some(CMD_SEARCH_WORD),

        // Selection manipulation (no Alt)
        //
        // 's'/'S' (select_regex/split_selection) are NOT mapped here: they
        // open the regex-selection prompt (`RegexPromptPending`) rather than
        // executing immediately, handled by a dedicated `base.rs` arm.
        ('&', _, false) => Some(CMD_ALIGN_SELECTIONS),
        ('_', _, false) => Some(CMD_TRIM_SELECTIONS),
        ('C', _, false) => Some(CMD_COPY_SELECTION_NEXT),
        ('K', _, false) => Some(CMD_KEEP_MATCHING),
        (',', false, false) => Some(CMD_KEEP_PRIMARY_SELECTION),

        _ => None,
    }
}

/// Check if a command is compatible with count prefix.
///
/// `r` is not listed: a counted `r` opens `ReplaceCharPending` with the count dropped.
/// A counted `o`/`O` opens one line and swallows the count.
pub fn is_count_compatible_command(c: char, modifiers: KeyModifiers) -> bool {
    let is_shift = modifiers.contains(KeyModifiers::SHIFT);

    matches!(
        (c, is_shift),
        (
            'h' | 'j'
                | 'k'
                | 'l'
                | 'w'
                | 'b'
                | 'e'
                | 'x'
                | 'd'
                | 'c'
                | 'u'
                | 'n'
                | 'y'
                | 'v'
                | 'i'
                | 'a'
                | 'o',
            false
        ) | (
            'W' | 'B' | 'E' | 'X' | 'J' | '>' | '<' | 'U' | 'N' | 'I' | 'A' | 'C' | 'O',
            _
        )
    )
}

/// Map a key event to a Helix command string in normal mode
///
/// This is a convenience function for use in handlers that maps
/// simple key events to their corresponding Helix command strings.
/// It handles single-key commands but not multi-key sequences
/// (those require the `InputStateMachine`).
///
/// Note: This function normalizes the key event before mapping.
pub fn map_key_to_helix_command(key: KeyEvent) -> Option<&'static str> {
    let key = normalize_key_event(key);
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
/// This handles printable characters, escape, backspace, arrow keys and the
/// Helix insert-mode editing chords (`Ctrl-w`, `Ctrl-u`, `Ctrl-k`, `Ctrl-d`,
/// `Alt-d`, `Alt-Backspace`, `Delete`). A `Ctrl`/`Alt`-modified character with
/// no insert-mode binding yields `None`: it must never be typed as text (#489).
pub fn handle_insert_mode_input(key: KeyEvent) -> Option<Cow<'static, str>> {
    let chord = Chord::of(key.modifiers);
    let borrowed = |cmd: &'static str| Some(Cow::Borrowed(cmd));
    match (chord, key.code) {
        (Chord::Plain, KeyCode::Char(c)) => Some(Cow::Owned(c.to_string())),
        // crossterm on Windows reports AltGr (`@`, `{`, `[` on German/Polish
        // layouts) as Ctrl+Alt without stripping either flag.
        (_, KeyCode::Char(c))
            if key
                .modifiers
                .contains(KeyModifiers::ALT | KeyModifiers::CONTROL) =>
        {
            Some(Cow::Owned(c.to_string()))
        }
        (Chord::Ctrl, KeyCode::Char('w')) => borrowed(CMD_INSERT_DELETE_WORD_BACKWARD),
        (Chord::Ctrl, KeyCode::Char('u')) => borrowed(CMD_INSERT_DELETE_TO_LINE_START),
        (Chord::Ctrl, KeyCode::Char('k')) => borrowed(CMD_INSERT_DELETE_TO_LINE_END),
        (Chord::Ctrl, KeyCode::Char('d')) => borrowed(CMD_INSERT_DELETE_CHAR_FORWARD),
        (Chord::Alt, KeyCode::Char('d')) => borrowed(CMD_INSERT_DELETE_WORD_FORWARD),
        (Chord::Alt, KeyCode::Backspace) => borrowed(CMD_INSERT_ALT_BACKSPACE),
        (Chord::Ctrl | Chord::Alt, KeyCode::Char(_)) => None,
        (_, KeyCode::Esc) => borrowed(CMD_ESCAPE),
        (_, KeyCode::Backspace) => borrowed(CMD_BACKSPACE),
        (_, KeyCode::Delete) => borrowed(CMD_INSERT_DELETE_KEY),
        (_, KeyCode::Enter) => borrowed("\n"),
        (_, KeyCode::Tab) => borrowed(CMD_INSERT_TAB),
        (_, KeyCode::Left) => borrowed(CMD_ARROW_LEFT),
        (_, KeyCode::Right) => borrowed(CMD_ARROW_RIGHT),
        (_, KeyCode::Up) => borrowed(CMD_ARROW_UP),
        (_, KeyCode::Down) => borrowed(CMD_ARROW_DOWN),
        _ => None,
    }
}

/// Convert a command string to a `KeyEvent` for the input state machine
///
/// Bridges command strings from the input layer to the `KeyEvent`-based
/// state machine via `parse_helix_key_string()`, which supports modifiers
/// like Alt-C, Ctrl-w, etc. Returns `None` for a token that is not a key, so
/// callers drop it instead of feeding the machine a different key.
///
/// # Examples
///
/// ```
/// use crossterm::event::{KeyCode, KeyModifiers};
/// use helix_trainer::input::typestate::command_to_key_event;
///
/// let key = command_to_key_event("h").unwrap();
/// assert_eq!(key.code, KeyCode::Char('h'));
///
/// let key = command_to_key_event("Escape").unwrap();
/// assert_eq!(key.code, KeyCode::Esc);
///
/// let key = command_to_key_event("Alt-C").unwrap();
/// assert!(key.modifiers.contains(KeyModifiers::ALT));
/// assert!(key.modifiers.contains(KeyModifiers::SHIFT));
///
/// assert!(command_to_key_event("not-a-key").is_none());
/// ```
pub fn command_to_key_event(command: &str) -> Option<KeyEvent> {
    parse_helix_key_string(command)
}
