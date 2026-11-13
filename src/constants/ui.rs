//! UI-related constants
//!
//! Dimensions, sizes, and display limits for the user interface.

// Hint popup dimensions
/// Maximum width for hint popup (in cells)
pub(crate) const HINT_POPUP_MAX_WIDTH: u16 = 70;
/// Maximum height for hint popup (in lines)
pub(crate) const HINT_POPUP_MAX_HEIGHT: u16 = 10;

// Key history display
/// Number of keys to show in key history
pub(crate) const KEY_HISTORY_DISPLAY_SIZE: usize = 5;
/// Width of each big text character (in cells)
///
/// The `tui-big-text` glyph is 8 pixels wide; `PixelSize::HalfHeight` only
/// changes the vertical pixel-to-cell ratio, so the horizontal ratio stays
/// 1:1 and each character still occupies a full 8 cells.
pub(crate) const BIG_TEXT_CHAR_WIDTH_CELLS: usize = 8;
/// Minimum width for key history popup
pub(crate) const KEY_HISTORY_MIN_WIDTH: u16 = 30;
/// Cells a popup border consumes along one axis (both edges)
pub(crate) const POPUP_BORDER_CELLS: u16 = 2;
/// Height of big text display (in lines)
pub(crate) const BIG_TEXT_HEIGHT_LINES: u16 = 8;

// Progress bar
/// Width of progress bars (in characters)
pub(crate) const PROGRESS_BAR_WIDTH: usize = 20;

// Notifications
/// Maximum number of visible notifications at once
pub(crate) const MAX_VISIBLE_NOTIFICATIONS: usize = 3;
