//! Command string constants for Helix editor commands
//!
//! This module defines const string constants for all Helix commands to avoid
//! string literal duplication and provide type safety.

// Multi-key commands
pub const CMD_GOTO_FILE_START: &str = "gg";

// Single character commands - Movement
pub const CMD_MOVE_LEFT: &str = "h";
pub const CMD_MOVE_DOWN: &str = "j";
pub const CMD_MOVE_UP: &str = "k";
pub const CMD_MOVE_RIGHT: &str = "l";
pub const CMD_MOVE_WORD_FORWARD: &str = "w";
pub const CMD_MOVE_WORD_BACKWARD: &str = "b";
pub const CMD_MOVE_WORD_END: &str = "e";
pub const CMD_MOVE_LONG_WORD_FORWARD: &str = "W";
pub const CMD_MOVE_LONG_WORD_BACKWARD: &str = "B";
pub const CMD_MOVE_LONG_WORD_END: &str = "E";
pub const CMD_MOVE_LINE_START: &str = "0";
pub const CMD_MOVE_LINE_END: &str = "$";
pub const CMD_PAGE_UP: &str = "Ctrl-b";
pub const CMD_PAGE_DOWN: &str = "Ctrl-f";
pub const CMD_HALF_PAGE_UP: &str = "Ctrl-u";
pub const CMD_HALF_PAGE_DOWN: &str = "Ctrl-d";
pub const CMD_GOTO_PREV_PARAGRAPH: &str = "[p";
pub const CMD_GOTO_NEXT_PARAGRAPH: &str = "]p";
pub const CMD_GOTO_FIRST_NONBLANK: &str = "^";

// Editing commands
pub const CMD_DELETE_SELECTION: &str = "d";
pub const CMD_INSERT: &str = "i";
pub const CMD_APPEND: &str = "a";
pub const CMD_INSERT_LINE_START: &str = "I";
pub const CMD_APPEND_LINE_END: &str = "A";
pub const CMD_OPEN_BELOW: &str = "o";
pub const CMD_OPEN_ABOVE: &str = "O";
pub const CMD_CHANGE: &str = "c";
pub const CMD_CHANGE_SELECTION_NOYANK: &str = "Alt-c";
pub const CMD_JOIN_LINES: &str = "J";
pub const CMD_JOIN_SELECTIONS_SPACE: &str = "Alt-J";
pub const CMD_INDENT: &str = ">";
pub const CMD_DEDENT: &str = "<";
pub const CMD_SWITCH_CASE: &str = "~";
pub const CMD_SWITCH_CASE_ALT: &str = "`";
pub const CMD_SWITCH_TO_UPPERCASE: &str = "Alt-`";
pub const CMD_REPLACE_WITH_YANKED: &str = "R";
pub const CMD_SHRINK_TO_LINE_BOUNDS: &str = "Alt-x";

// Clipboard commands
pub const CMD_YANK: &str = "y";
pub const CMD_PASTE_AFTER: &str = "p";
pub const CMD_PASTE_BEFORE: &str = "P";

// Undo/Redo
pub const CMD_UNDO: &str = "u";
pub const CMD_REDO: &str = "U";
pub const CMD_CTRL_R: &str = "Ctrl-r";

// Selection commands
pub const CMD_SELECT_LINE: &str = "x";
pub const CMD_EXTEND_LINE: &str = "X";
pub const CMD_SELECT_ALL: &str = "%";
pub const CMD_COLLAPSE_SELECTION: &str = ";";

// Find/till commands (followed by character)
pub const CMD_FIND_CHAR: &str = "f";
pub const CMD_FIND_CHAR_REVERSE: &str = "F";
pub const CMD_TILL_CHAR: &str = "t";
pub const CMD_TILL_CHAR_REVERSE: &str = "T";
pub const CMD_REPEAT_LAST_MOTION: &str = "Alt-.";

// Selection management commands
pub const CMD_KEEP_PRIMARY_SELECTION: &str = ",";
pub const CMD_REMOVE_PRIMARY_SELECTION: &str = "Alt-,";

// Selection mode
pub const CMD_SELECT_MODE: &str = "v";

// Flip selection direction
pub const CMD_FLIP_SELECTIONS: &str = "Alt-;";

// Match mode (prefix for match commands)
pub const CMD_MATCH_MODE: &str = "m";
pub const CMD_MATCH_BRACKETS: &str = "mm";

// Named register selection (prefix for register-scoped yank/paste: "<reg><op>)
pub const CMD_SELECT_REGISTER: &str = "\"";

// Command-line mode (prefix for `:`-typed commands, e.g. ":goto 3")
pub const CMD_COMMAND_LINE: &str = ":";

// Match mode surround commands (ms{char}, mr{from}{to}, md{char})
pub const CMD_SURROUND_ADD_PREFIX: &str = "ms";
pub const CMD_SURROUND_REPLACE_PREFIX: &str = "mr";
pub const CMD_SURROUND_DELETE_PREFIX: &str = "md";

// Goto mode (prefix for goto commands)
pub const CMD_GOTO_MODE: &str = "g";
pub const CMD_GOTO_LINE_START: &str = "gh";
pub const CMD_GOTO_LINE_END: &str = "gl";
pub const CMD_GOTO_FIRST_NONWHITESPACE: &str = "gs";
pub const CMD_GOTO_LAST_LINE: &str = "ge";

// Special commands
pub const CMD_ESCAPE: &str = "Escape";
pub const CMD_REPEAT: &str = ".";

// Replace command prefix (used with character, e.g., "rx")
pub const CMD_REPLACE: &str = "r";

// Special keys (used in insert mode and command conversion)
pub const CMD_BACKSPACE: &str = "Backspace";
pub const CMD_ARROW_LEFT: &str = "Left";
pub const CMD_ARROW_RIGHT: &str = "Right";
pub const CMD_ARROW_UP: &str = "Up";
pub const CMD_ARROW_DOWN: &str = "Down";

// Selection commands
pub const CMD_SELECT_REGEX: &str = "s";
pub const CMD_SPLIT_SELECTION: &str = "S";
pub const CMD_SPLIT_SELECTION_NEWLINES: &str = "Alt-s";
pub const CMD_ALIGN_SELECTIONS: &str = "&";
pub const CMD_TRIM_SELECTIONS: &str = "_";
pub const CMD_MERGE_SELECTIONS: &str = "Alt--";
pub const CMD_MERGE_CONSECUTIVE: &str = "Alt-_";
pub const CMD_COPY_SELECTION_NEXT: &str = "C";
pub const CMD_COPY_SELECTION_PREV: &str = "Alt-C";
pub const CMD_KEEP_MATCHING: &str = "K";
pub const CMD_REMOVE_MATCHING: &str = "Alt-K";
pub const CMD_TOGGLE_COMMENTS: &str = "Ctrl-c";

// Search commands
pub const CMD_SEARCH_FORWARD: &str = "/";
pub const CMD_SEARCH_BACKWARD: &str = "?";
pub const CMD_SEARCH_NEXT: &str = "n";
pub const CMD_SEARCH_PREV: &str = "N";
pub const CMD_SEARCH_WORD: &str = "*";
pub const CMD_SEARCH_SELECTION: &str = "Alt-*";

// View mode commands
pub const CMD_VIEW_MODE: &str = "z";
pub const CMD_VIEW_CENTER: &str = "zz";
pub const CMD_VIEW_TOP: &str = "zt";
pub const CMD_VIEW_BOTTOM: &str = "zb";
pub const CMD_VIEW_CENTER_HORIZONTAL: &str = "zm";
pub const CMD_SCROLL_DOWN: &str = "zj";
pub const CMD_SCROLL_UP: &str = "zk";

/// Canonicalize a raw executed command string to a stable FSRS/quest card id.
///
/// Two families of commands otherwise mint a separate learning card per
/// operand, which fragments spaced-repetition data for what is really one
/// skill:
///
/// - Register-scoped clipboard ops (`"ay`, `"by`, ...) collapse to `"y`
///   (the register letter is not the thing being taught).
/// - Command-line invocations (`:goto 3`, `:g 7`, ...) collapse to their
///   canonical name (`:goto`), folding aliases (`:g` -> `:goto`) so both
///   spellings share one card.
///
/// Every other command string is returned unchanged.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::commands::normalize_command_id;
///
/// assert_eq!(normalize_command_id("\"ay"), "\"y");
/// assert_eq!(normalize_command_id(":goto 3"), ":goto");
/// assert_eq!(normalize_command_id(":g 3"), ":goto");
/// assert_eq!(normalize_command_id("h"), "h");
/// ```
pub fn normalize_command_id(cmd: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;

    if cmd.starts_with(CMD_SELECT_REGISTER) {
        // Destructure via `chars()`, not `len() == 3` (bytes) + `.nth(2)`:
        // a multi-byte register char would make `len()` 3 for only 2 chars,
        // and `.nth(2).expect(...)` would then panic on `None`. This form
        // only matches when there are exactly 3 chars total.
        let mut chars = cmd.chars();
        chars.next(); // the leading '"', already confirmed by starts_with
        if let (Some(_register), Some(op), None) = (chars.next(), chars.next(), chars.next()) {
            return Cow::Owned(format!("{CMD_SELECT_REGISTER}{op}"));
        }
    }

    if let Some(body) = cmd.strip_prefix(CMD_COMMAND_LINE) {
        let name = body.split_whitespace().next().unwrap_or("");
        let canonical = match name {
            "g" => "goto",
            other => other,
        };
        return Cow::Owned(format!("{CMD_COMMAND_LINE}{canonical}"));
    }

    Cow::Borrowed(cmd)
}

#[cfg(test)]
mod normalize_command_id_tests {
    use super::*;

    #[test]
    fn register_op_normalizes_to_bare_op() {
        assert_eq!(normalize_command_id("\"ay"), "\"y");
        assert_eq!(normalize_command_id("\"bp"), "\"p");
    }

    #[test]
    fn command_line_normalizes_to_canonical_name() {
        assert_eq!(normalize_command_id(":goto 3"), ":goto");
        assert_eq!(normalize_command_id(":g 3"), ":goto");
        assert_eq!(normalize_command_id(":g"), ":goto");
    }

    /// Regression test: a multi-byte register char used to panic via
    /// `len() == 3` (bytes) succeeding while `.nth(2)` (chars) returned
    /// `None`.
    #[test]
    fn malformed_register_strings_do_not_panic() {
        assert_eq!(normalize_command_id("\"é"), "\"é"); // 2 chars, 3 bytes - no op char
        assert_eq!(normalize_command_id("\""), "\"");
        assert_eq!(normalize_command_id("\"aay"), "\"aay");
    }

    #[test]
    fn register_op_with_non_ascii_register_still_normalizes() {
        // 3 chars (quote + register + op), 4 bytes - well-formed despite
        // the multi-byte register char.
        assert_eq!(normalize_command_id("\"éy"), "\"y");
    }

    #[test]
    fn unrelated_commands_are_unchanged() {
        assert_eq!(normalize_command_id("h"), "h");
        assert_eq!(normalize_command_id("gg"), "gg");
        assert_eq!(normalize_command_id("ms("), "ms(");
    }
}
