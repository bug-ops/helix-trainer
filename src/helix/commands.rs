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

// Editing commands
pub const CMD_DELETE_SELECTION: &str = "d";
pub const CMD_INSERT: &str = "i";
pub const CMD_APPEND: &str = "a";
pub const CMD_INSERT_LINE_START: &str = "I";
pub const CMD_APPEND_LINE_END: &str = "A";
pub const CMD_OPEN_BELOW: &str = "o";
pub const CMD_OPEN_ABOVE: &str = "O";
pub const CMD_CHANGE: &str = "c";
pub const CMD_JOIN_LINES: &str = "J";
pub const CMD_INDENT: &str = ">";
pub const CMD_DEDENT: &str = "<";
pub const CMD_SWITCH_CASE: &str = "~";
pub const CMD_SWITCH_CASE_ALT: &str = "`";

// Clipboard commands
pub const CMD_YANK: &str = "y";
pub const CMD_PASTE_AFTER: &str = "p";
pub const CMD_PASTE_BEFORE: &str = "P";

// Undo/Redo
pub const CMD_UNDO: &str = "u";
pub const CMD_REDO: &str = "U";
pub const CMD_CTRL_R: &str = "ctrl-r";

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

// Selection mode
pub const CMD_SELECT_MODE: &str = "v";

// Flip selection direction
pub const CMD_FLIP_SELECTIONS: &str = "Alt-;";

// Match mode (prefix for match commands)
pub const CMD_MATCH_MODE: &str = "m";
pub const CMD_MATCH_BRACKETS: &str = "mm";

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

// Special markers (not actual Helix commands)
pub const CMD_CANCEL: &str = "<cancel>";
