//! Command string constants for Helix editor commands
//!
//! This module defines const string constants for all Helix commands to avoid
//! string literal duplication and provide type safety.

// Multi-key commands
/// Jump to the first line of the file. Key: `gg`.
pub(crate) const CMD_GOTO_FILE_START: &str = "gg";

// Single character commands - Movement
/// Move one character left. Key: `h`.
pub(crate) const CMD_MOVE_LEFT: &str = "h";
/// Move one line down. Key: `j`.
pub(crate) const CMD_MOVE_DOWN: &str = "j";
/// Move one line up. Key: `k`.
pub(crate) const CMD_MOVE_UP: &str = "k";
/// Move one character right. Key: `l`.
pub(crate) const CMD_MOVE_RIGHT: &str = "l";
/// Select to the start of the next word. Key: `w`.
pub(crate) const CMD_MOVE_WORD_FORWARD: &str = "w";
/// Select to the start of the previous word. Key: `b`.
pub(crate) const CMD_MOVE_WORD_BACKWARD: &str = "b";
/// Select to the end of the next word. Key: `e`.
pub(crate) const CMD_MOVE_WORD_END: &str = "e";
/// Like the word motion, but words are whitespace-delimited (WORD). Key: `W`.
pub(crate) const CMD_MOVE_LONG_WORD_FORWARD: &str = "W";
/// Backward counterpart of the whitespace-delimited WORD motion. Key: `B`.
pub(crate) const CMD_MOVE_LONG_WORD_BACKWARD: &str = "B";
/// End-of-word counterpart of the whitespace-delimited WORD motion. Key: `E`.
pub(crate) const CMD_MOVE_LONG_WORD_END: &str = "E";
/// Scroll one full page up. Key: `Ctrl-b`.
pub(crate) const CMD_PAGE_UP: &str = "Ctrl-b";
/// Scroll one full page down. Key: `Ctrl-f`.
pub(crate) const CMD_PAGE_DOWN: &str = "Ctrl-f";
/// Scroll half a page up. Key: `Ctrl-u`.
pub(crate) const CMD_HALF_PAGE_UP: &str = "Ctrl-u";
/// Scroll half a page down. Key: `Ctrl-d`.
pub(crate) const CMD_HALF_PAGE_DOWN: &str = "Ctrl-d";
/// Jump to the previous paragraph. Key: `[p`.
pub(crate) const CMD_GOTO_PREV_PARAGRAPH: &str = "[p";
/// Jump to the next paragraph. Key: `]p`.
pub(crate) const CMD_GOTO_NEXT_PARAGRAPH: &str = "]p";
/// Move to the first non-blank character of the line. Key: `^`.
pub(crate) const CMD_GOTO_FIRST_NONBLANK: &str = "^";

// Editing commands
/// Delete the selection, copying it to the default register. Key: `d`.
pub(crate) const CMD_DELETE_SELECTION: &str = "d";
/// Delete the selection without touching the default register. Key: `Alt-d`.
pub(crate) const CMD_DELETE_SELECTION_NOYANK: &str = "Alt-d";
/// Enter Insert mode before the selection. Key: `i`.
pub(crate) const CMD_INSERT: &str = "i";
/// Enter Insert mode after the selection. Key: `a`.
pub(crate) const CMD_APPEND: &str = "a";
/// Enter Insert mode at the start of the line. Key: `I`.
pub(crate) const CMD_INSERT_LINE_START: &str = "I";
/// Enter Insert mode at the end of the line. Key: `A`.
pub(crate) const CMD_APPEND_LINE_END: &str = "A";
/// Open a new line below and enter Insert mode. Key: `o`.
pub(crate) const CMD_OPEN_BELOW: &str = "o";
/// Open a new line above and enter Insert mode. Key: `O`.
pub(crate) const CMD_OPEN_ABOVE: &str = "O";
/// Delete the selection to the default register and enter Insert mode. Key: `c`.
pub(crate) const CMD_CHANGE: &str = "c";
/// Change the selection without touching the default register. Key: `Alt-c`.
pub(crate) const CMD_CHANGE_SELECTION_NOYANK: &str = "Alt-c";
/// Join the selected lines into one. Key: `J`.
pub(crate) const CMD_JOIN_LINES: &str = "J";
/// Join the selected lines and select the inserted spaces. Key: `Alt-J`.
pub(crate) const CMD_JOIN_SELECTIONS_SPACE: &str = "Alt-J";
/// Indent the selected lines. Key: `>`.
pub(crate) const CMD_INDENT: &str = ">";
/// Dedent the selected lines. Key: `<`.
pub(crate) const CMD_DEDENT: &str = "<";
/// Toggle the case of the selected characters. Key: `~`.
pub(crate) const CMD_SWITCH_CASE: &str = "~";
/// Convert the selection to lowercase. Key: `` ` ``.
pub(crate) const CMD_SWITCH_CASE_ALT: &str = "`";
/// Convert the selection to uppercase. Key: `` Alt-` ``.
pub(crate) const CMD_SWITCH_TO_UPPERCASE: &str = "Alt-`";
/// Replace the selection with the yanked text. Key: `R`.
pub(crate) const CMD_REPLACE_WITH_YANKED: &str = "R";
/// Shrink the selection to the line bounds. Key: `Alt-x`.
pub(crate) const CMD_SHRINK_TO_LINE_BOUNDS: &str = "Alt-x";

// Clipboard commands
/// Copy the selection to the default register. Key: `y`.
pub(crate) const CMD_YANK: &str = "y";
/// Paste the register contents after the selection. Key: `p`.
pub(crate) const CMD_PASTE_AFTER: &str = "p";
/// Paste the register contents before the selection. Key: `P`.
pub(crate) const CMD_PASTE_BEFORE: &str = "P";

// Undo/Redo
/// Undo the last change. Key: `u`.
pub(crate) const CMD_UNDO: &str = "u";
/// Redo the last undone change. Key: `U`.
pub(crate) const CMD_REDO: &str = "U";
/// Insert mode: insert the contents of a register. Key: `Ctrl-r`.
pub(crate) const CMD_CTRL_R: &str = "Ctrl-r";

// Selection commands
/// Select the current line, extending on repeat. Key: `x`.
pub(crate) const CMD_SELECT_LINE: &str = "x";
/// Extend the selection to whole lines. Key: `X`.
pub(crate) const CMD_EXTEND_LINE: &str = "X";
/// Select the entire buffer. Key: `%`.
pub(crate) const CMD_SELECT_ALL: &str = "%";
/// Collapse each selection to a single cursor. Key: `;`.
pub(crate) const CMD_COLLAPSE_SELECTION: &str = ";";

// Find/till commands (followed by character)
/// Select up to and including the next occurrence of a character (takes a character argument). Key: `f`.
pub(crate) const CMD_FIND_CHAR: &str = "f";
/// Backward counterpart of the find motion (takes a character argument). Key: `F`.
pub(crate) const CMD_FIND_CHAR_REVERSE: &str = "F";
/// Select up to, excluding, the next occurrence of a character (takes a character argument). Key: `t`.
pub(crate) const CMD_TILL_CHAR: &str = "t";
/// Backward counterpart of the till motion (takes a character argument). Key: `T`.
pub(crate) const CMD_TILL_CHAR_REVERSE: &str = "T";
/// Repeat the last find or till motion. Key: `Alt-.`.
pub(crate) const CMD_REPEAT_LAST_MOTION: &str = "Alt-.";

// Selection management commands
/// Keep only the primary selection. Key: `,`.
pub(crate) const CMD_KEEP_PRIMARY_SELECTION: &str = ",";
/// Remove the primary selection, keeping the others. Key: `Alt-,`.
pub(crate) const CMD_REMOVE_PRIMARY_SELECTION: &str = "Alt-,";

// Selection mode
/// Toggle Select mode. Key: `v`.
pub(crate) const CMD_SELECT_MODE: &str = "v";

// Flip selection direction
/// Swap the anchor and head of each selection. Key: `Alt-;`.
pub(crate) const CMD_FLIP_SELECTIONS: &str = "Alt-;";

// Match mode (prefix for match commands)
/// Jump to the bracket matching the one under the cursor. Key: `mm`.
pub(crate) const CMD_MATCH_BRACKETS: &str = "mm";

// Named register selection (prefix for register-scoped yank/paste: "<reg><op>)
/// Prefix that selects a named register for the next yank, paste or delete. Key: `"`.
pub(crate) const CMD_SELECT_REGISTER: &str = "\"";

// Command-line mode (prefix for `:`-typed commands, e.g. ":goto 3")
/// Prefix that opens the `:` command line. Key: `:`.
pub(crate) const CMD_COMMAND_LINE: &str = ":";

// Match mode surround commands (ms{char}, mr{from}{to}, md{char})
/// Match-mode prefix that surrounds the selection with a character pair. Key: `ms`.
pub const CMD_SURROUND_ADD_PREFIX: &str = "ms";

// Goto mode (prefix for goto commands)
/// Jump to the start of the line. Key: `gh`.
pub(crate) const CMD_GOTO_LINE_START: &str = "gh";
/// Jump to the end of the line. Key: `gl`.
pub(crate) const CMD_GOTO_LINE_END: &str = "gl";
/// Jump to the first non-whitespace character of the line. Key: `gs`.
pub(crate) const CMD_GOTO_FIRST_NONWHITESPACE: &str = "gs";
/// Jump to the last line of the file. Key: `ge`.
pub(crate) const CMD_GOTO_LAST_LINE: &str = "ge";
/// Jump to the line given by the count prefix. Key: `G`.
pub(crate) const CMD_GOTO_LINE: &str = "G";

// Special commands
/// Leave the current mode or cancel a pending command. Key: `Escape`.
pub(crate) const CMD_ESCAPE: &str = "Escape";
/// Repeat the last Insert session (entry command, typed keys, `Escape`), as hx `repeat_last_insert`. Key: `.`.
pub(crate) const CMD_REPEAT: &str = ".";

// Macro recording and replay
/// Start or stop recording a macro. Key: `Q`.
pub const CMD_TOGGLE_MACRO_RECORDING: &str = "Q";
/// Replay the recorded macro. Key: `q`.
pub const CMD_REPLAY_MACRO: &str = "q";

// Replace command prefix (used with character, e.g., "rx")
/// Replace each selected character with the next typed one (takes a character argument). Key: `r`.
pub(crate) const CMD_REPLACE: &str = "r";

// Special keys (used in insert mode and command conversion)
/// Delete the character before the cursor. Key: `Backspace`.
pub(crate) const CMD_BACKSPACE: &str = "Backspace";
/// Left arrow key. Key: `Left`.
pub(crate) const CMD_ARROW_LEFT: &str = "Left";
/// Right arrow key. Key: `Right`.
pub(crate) const CMD_ARROW_RIGHT: &str = "Right";
/// Up arrow key. Key: `Up`.
pub(crate) const CMD_ARROW_UP: &str = "Up";
/// Down arrow key. Key: `Down`.
pub(crate) const CMD_ARROW_DOWN: &str = "Down";

// Insert-mode editing keys (Helix 25.07.1 defaults)
/// Insert mode: delete the previous word. Key: `Ctrl-w`.
pub(crate) const CMD_INSERT_DELETE_WORD_BACKWARD: &str = "Ctrl-w";
/// Insert mode: delete back to the start of the line. Key: `Ctrl-u`.
pub(crate) const CMD_INSERT_DELETE_TO_LINE_START: &str = "Ctrl-u";
/// Insert mode: delete to the end of the line. Key: `Ctrl-k`.
pub(crate) const CMD_INSERT_DELETE_TO_LINE_END: &str = "Ctrl-k";
/// Insert mode: delete the character under the cursor. Key: `Ctrl-d`.
pub(crate) const CMD_INSERT_DELETE_CHAR_FORWARD: &str = "Ctrl-d";
/// Insert mode: delete the next word. Key: `Alt-d`.
pub(crate) const CMD_INSERT_DELETE_WORD_FORWARD: &str = "Alt-d";
/// Insert mode: delete the previous word (Alt-Backspace spelling). Key: `Alt-Backspace`.
pub(crate) const CMD_INSERT_ALT_BACKSPACE: &str = "Alt-Backspace";
/// Insert mode: Delete key, removes the character under the cursor. Key: `Delete`.
pub(crate) const CMD_INSERT_DELETE_KEY: &str = "Delete";
/// Insert mode: Tab key, inserts indentation. Key: `Tab`.
pub(crate) const CMD_INSERT_TAB: &str = "\t";

// Selection commands
/// Select every regex match inside the current selection. Key: `s`.
pub const CMD_SELECT_REGEX: &str = "s";
/// Split the selection on every regex match. Key: `S`.
pub const CMD_SPLIT_SELECTION: &str = "S";
/// Split the selection into one selection per line. Key: `Alt-s`.
pub(crate) const CMD_SPLIT_SELECTION_NEWLINES: &str = "Alt-s";
/// Align the selections in columns. Key: `&`.
pub(crate) const CMD_ALIGN_SELECTIONS: &str = "&";
/// Trim leading and trailing whitespace from each selection. Key: `_`.
pub(crate) const CMD_TRIM_SELECTIONS: &str = "_";
/// Merge all selections into one. Key: `Alt--`.
pub(crate) const CMD_MERGE_SELECTIONS: &str = "Alt--";
/// Merge adjacent selections. Key: `Alt-_`.
pub(crate) const CMD_MERGE_CONSECUTIVE: &str = "Alt-_";
/// Duplicate the selection onto the next line. Key: `C`.
pub(crate) const CMD_COPY_SELECTION_NEXT: &str = "C";
/// Duplicate the selection onto the previous line. Key: `Alt-C`.
pub(crate) const CMD_COPY_SELECTION_PREV: &str = "Alt-C";
/// Keep only the selections that match a regex. Key: `K`.
pub(crate) const CMD_KEEP_MATCHING: &str = "K";
/// Remove the selections that match a regex. Key: `Alt-K`.
pub(crate) const CMD_REMOVE_MATCHING: &str = "Alt-K";
/// Comment or uncomment the selected lines. Key: `Ctrl-c`.
pub(crate) const CMD_TOGGLE_COMMENTS: &str = "Ctrl-c";

// Search commands
/// Open the forward search prompt. Key: `/`.
pub(crate) const CMD_SEARCH_FORWARD: &str = "/";
/// Open the backward search prompt. Key: `?`.
pub(crate) const CMD_SEARCH_BACKWARD: &str = "?";
/// Jump to the next search match. Key: `n`.
pub(crate) const CMD_SEARCH_NEXT: &str = "n";
/// Jump to the previous search match. Key: `N`.
pub(crate) const CMD_SEARCH_PREV: &str = "N";
/// Search for the word under the cursor. Key: `*`.
pub(crate) const CMD_SEARCH_WORD: &str = "*";
/// Search for the selection as a literal string. Key: `Alt-*`.
pub(crate) const CMD_SEARCH_SELECTION: &str = "Alt-*";

// View mode commands
/// Scroll so the cursor line is vertically centered. Key: `zz`.
pub(crate) const CMD_VIEW_CENTER: &str = "zz";
/// Scroll so the cursor line is at the top. Key: `zt`.
pub(crate) const CMD_VIEW_TOP: &str = "zt";
/// Scroll so the cursor line is at the bottom. Key: `zb`.
pub(crate) const CMD_VIEW_BOTTOM: &str = "zb";
/// Scroll so the cursor column is horizontally centered. Key: `zm`.
pub(crate) const CMD_VIEW_CENTER_HORIZONTAL: &str = "zm";
/// Scroll the view down without moving the cursor. Key: `zj`.
pub(crate) const CMD_SCROLL_DOWN: &str = "zj";
/// Scroll the view up without moving the cursor. Key: `zk`.
pub(crate) const CMD_SCROLL_UP: &str = "zk";

/// Canonicalize a raw executed command string to a stable FSRS/quest card id.
///
/// Three families of commands otherwise mint a separate learning card per
/// operand, which fragments spaced-repetition data for what is really one
/// skill:
///
/// - Register-scoped clipboard ops (`"ay`, `"by`, ...) collapse to `"y`
///   (the register letter is not the thing being taught).
/// - Command-line invocations (`:goto 3`, `:g 7`, ...) collapse to their
///   canonical name (`:goto`), folding aliases (`:g` -> `:goto`) so both
///   spellings share one card.
/// - Regex-selection prompt invocations (`s foo`, `s bar`, `S ,`, ...)
///   collapse to the bare command (`s`, `S`) - the pattern is not the thing
///   being taught.
///
/// Every other command string is returned unchanged.
///
/// # Examples
///
/// ```
/// use helix_trainer::grammar::commands::normalize_command_id;
///
/// assert_eq!(normalize_command_id("\"ay"), "\"y");
/// assert_eq!(normalize_command_id(":goto 3"), ":goto");
/// assert_eq!(normalize_command_id(":g 3"), ":goto");
/// assert_eq!(normalize_command_id("s foo"), "s");
/// assert_eq!(normalize_command_id("S ,"), "S");
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

    if cmd
        .strip_prefix(CMD_SELECT_REGEX)
        .and_then(|r| r.strip_prefix(' '))
        .is_some()
    {
        return Cow::Borrowed(CMD_SELECT_REGEX);
    }
    if cmd
        .strip_prefix(CMD_SPLIT_SELECTION)
        .and_then(|r| r.strip_prefix(' '))
        .is_some()
    {
        return Cow::Borrowed(CMD_SPLIT_SELECTION);
    }
    for prompt_command in [CMD_KEEP_MATCHING, CMD_REMOVE_MATCHING] {
        if cmd
            .strip_prefix(prompt_command)
            .and_then(|r| r.strip_prefix(' '))
            .is_some()
        {
            return Cow::Borrowed(prompt_command);
        }
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

    /// Regression test: distinct `s`/`S` patterns must collapse to one FSRS
    /// card per command, mirroring the `:`-command precedent - otherwise
    /// every distinct pattern mints its own permanent learning card.
    #[test]
    fn regex_selection_patterns_normalize_to_bare_command() {
        assert_eq!(normalize_command_id("s foo"), "s");
        assert_eq!(normalize_command_id("s bar"), "s");
        assert_eq!(normalize_command_id("S ,"), "S");
        assert_eq!(normalize_command_id("S foo bar"), "S"); // pattern with a space
    }

    #[test]
    fn keep_and_remove_patterns_normalize_to_bare_command() {
        assert_eq!(normalize_command_id("K foo"), "K");
        assert_eq!(normalize_command_id("Alt-K foo bar"), "Alt-K");
        assert_eq!(normalize_command_id("K"), "K");
    }

    /// Bare `s`/`S` (no space, no pattern) is not the prompt-command shape
    /// and must be left unchanged.
    #[test]
    fn bare_s_and_shift_s_are_unchanged() {
        assert_eq!(normalize_command_id("s"), "s");
        assert_eq!(normalize_command_id("S"), "S");
    }
}
