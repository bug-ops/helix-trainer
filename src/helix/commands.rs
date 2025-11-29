//! Command string constants for Helix editor commands
//!
//! This module defines const string constants for all Helix commands to avoid
//! string literal duplication and provide type safety.

// Multi-key commands
pub const CMD_DELETE_LINE: &str = "dd";
pub const CMD_GOTO_FILE_START: &str = "gg";
pub const CMD_GOTO_FILE_END: &str = "G";

// Single character commands - Movement
pub const CMD_MOVE_LEFT: &str = "h";
pub const CMD_MOVE_DOWN: &str = "j";
pub const CMD_MOVE_UP: &str = "k";
pub const CMD_MOVE_RIGHT: &str = "l";
pub const CMD_MOVE_WORD_FORWARD: &str = "w";
pub const CMD_MOVE_WORD_BACKWARD: &str = "b";
pub const CMD_MOVE_WORD_END: &str = "e";
pub const CMD_MOVE_LINE_START: &str = "0";
pub const CMD_MOVE_LINE_END: &str = "$";

// Editing commands
pub const CMD_DELETE_CHAR: &str = "x";
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

// Clipboard commands
pub const CMD_YANK: &str = "y";
pub const CMD_PASTE_AFTER: &str = "p";
pub const CMD_PASTE_BEFORE: &str = "P";

// Undo/Redo
pub const CMD_UNDO: &str = "u";
pub const CMD_REDO: &str = "U";

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
