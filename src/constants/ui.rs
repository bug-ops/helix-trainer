//! UI-related constants
//!
//! Dimensions, sizes, and display limits for the user interface.

// Hint popup dimensions
/// Maximum width for hint popup (in cells)
pub const HINT_POPUP_MAX_WIDTH: u16 = 70;
/// Maximum height for hint popup (in lines)
pub const HINT_POPUP_MAX_HEIGHT: u16 = 10;

// Key history display
/// Number of keys to show in key history
pub const KEY_HISTORY_DISPLAY_SIZE: usize = 5;
/// Width of each big text character (in cells)
pub const BIG_TEXT_CHAR_WIDTH_CELLS: usize = 5;
/// Minimum width for key history popup
pub const KEY_HISTORY_MIN_WIDTH: u16 = 30;
/// Height of big text display (in lines)
pub const BIG_TEXT_HEIGHT_LINES: u16 = 8;

// Progress bar
/// Width of progress bars (in characters)
pub const PROGRESS_BAR_WIDTH: usize = 20;

// Notifications
/// Maximum number of visible notifications at once
pub const MAX_VISIBLE_NOTIFICATIONS: usize = 3;
