//! Command string constants for Helix editor commands
//!
//! This module defines static string constants for all Helix commands to avoid
//! string literal duplication and provide type safety.

// Multi-key commands
pub static CMD_DELETE_LINE: &str = "dd";
pub static CMD_GOTO_FILE_START: &str = "gg";
pub static CMD_GOTO_FILE_END: &str = "G";

// Single character commands - Movement
pub static CMD_MOVE_LEFT: &str = "h";
pub static CMD_MOVE_DOWN: &str = "j";
pub static CMD_MOVE_UP: &str = "k";
pub static CMD_MOVE_RIGHT: &str = "l";
pub static CMD_MOVE_WORD_FORWARD: &str = "w";
pub static CMD_MOVE_WORD_BACKWARD: &str = "b";
pub static CMD_MOVE_WORD_END: &str = "e";
pub static CMD_MOVE_LINE_START: &str = "0";
pub static CMD_MOVE_LINE_END: &str = "$";

// Editing commands
pub static CMD_DELETE_CHAR: &str = "x";
pub static CMD_INSERT: &str = "i";
pub static CMD_APPEND: &str = "a";
pub static CMD_INSERT_LINE_START: &str = "I";
pub static CMD_APPEND_LINE_END: &str = "A";
pub static CMD_OPEN_BELOW: &str = "o";
pub static CMD_OPEN_ABOVE: &str = "O";
pub static CMD_CHANGE: &str = "c";
pub static CMD_JOIN_LINES: &str = "J";
pub static CMD_INDENT: &str = ">";
pub static CMD_DEDENT: &str = "<";

// Clipboard commands
pub static CMD_YANK: &str = "y";
pub static CMD_PASTE_AFTER: &str = "p";
pub static CMD_PASTE_BEFORE: &str = "P";

// Undo/Redo
pub static CMD_UNDO: &str = "u";
pub static CMD_REDO: &str = "U";

// Special commands
pub static CMD_ESCAPE: &str = "Escape";
pub static CMD_REPEAT: &str = ".";

// Replace command prefix (used with character, e.g., "rx")
pub static CMD_REPLACE: &str = "r";

// Special keys (used in insert mode and command conversion)
pub static CMD_BACKSPACE: &str = "Backspace";
pub static CMD_ARROW_LEFT: &str = "ArrowLeft";
pub static CMD_ARROW_RIGHT: &str = "ArrowRight";
pub static CMD_ARROW_UP: &str = "ArrowUp";
pub static CMD_ARROW_DOWN: &str = "ArrowDown";
