//! View state management for Helix simulator
//!
//! Provides viewport tracking for view commands (z, zt, zb, zm, zj, zk).
//! The view state tracks the visible portion of the document.

/// Default number of visible lines in the viewport
const DEFAULT_VISIBLE_LINES: usize = 24;

/// Lines `hx` keeps between the cursor and the view edge (`scrolloff`)
const SCROLLOFF: usize = 5;

/// Default number of visible columns in the viewport
const DEFAULT_VISIBLE_COLS: usize = 80;

/// View state for the editor viewport
///
/// Tracks the visible portion of the document for scrolling
/// and viewport alignment operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewState {
    /// First visible line (0-indexed)
    top_line: usize,
    /// Number of visible lines in the viewport
    visible_lines: usize,
    /// First visible column (0-indexed, for horizontal scrolling)
    left_col: usize,
    /// Number of visible columns in the viewport
    visible_cols: usize,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            top_line: 0,
            visible_lines: DEFAULT_VISIBLE_LINES,
            left_col: 0,
            visible_cols: DEFAULT_VISIBLE_COLS,
        }
    }
}

impl ViewState {
    /// Create a new view state with default viewport size
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the first visible line
    pub fn top_line(&self) -> usize {
        self.top_line
    }

    /// Get the number of visible lines
    pub fn visible_lines(&self) -> usize {
        self.visible_lines
    }

    /// Get the first visible column
    pub fn left_col(&self) -> usize {
        self.left_col
    }

    /// Get the number of visible columns
    pub fn visible_cols(&self) -> usize {
        self.visible_cols
    }

    /// Get the last visible line (exclusive)
    ///
    /// Uses `saturating_add` to prevent overflow on extreme values.
    pub fn bottom_line(&self) -> usize {
        self.top_line.saturating_add(self.visible_lines)
    }

    /// Set the viewport size
    #[cfg(test)]
    pub(crate) fn set_size(&mut self, visible_lines: usize, visible_cols: usize) {
        self.visible_lines = visible_lines;
        self.visible_cols = visible_cols;
    }

    /// Center the viewport vertically on a given line
    ///
    /// Adjusts `top_line` so that `cursor_line` is in the center of the viewport.
    pub fn center_on_line(&mut self, cursor_line: usize, total_lines: usize) {
        let half_viewport = self.visible_lines / 2;
        if cursor_line < half_viewport {
            self.top_line = 0;
        } else if cursor_line + half_viewport >= total_lines {
            self.top_line = total_lines.saturating_sub(self.visible_lines);
        } else {
            self.top_line = cursor_line - half_viewport;
        }
    }

    /// Align the viewport so cursor line is at the top
    pub fn align_top(&mut self, cursor_line: usize, _total_lines: usize) {
        self.top_line = cursor_line;
    }

    /// Align the viewport so cursor line is at the bottom
    pub fn align_bottom(&mut self, cursor_line: usize, _total_lines: usize) {
        self.top_line = cursor_line.saturating_sub(self.visible_lines.saturating_sub(1));
    }

    /// Center the viewport horizontally on a given column
    pub fn center_on_col(&mut self, cursor_col: usize, _line_width: usize) {
        let half_viewport = self.visible_cols / 2;
        if cursor_col < half_viewport {
            self.left_col = 0;
        } else {
            self.left_col = cursor_col - half_viewport;
        }
    }

    /// Scroll down by count lines, as far as the last line of the document
    /// (`hx` lets the last line reach the top of the view)
    pub fn scroll_down(&mut self, count: usize, total_lines: usize) {
        let last_line = total_lines.saturating_sub(1);
        self.top_line = self.top_line.saturating_add(count).min(last_line);
    }

    /// Scroll up by count lines
    pub fn scroll_up(&mut self, count: usize) {
        self.top_line = self.top_line.saturating_sub(count);
    }

    /// Lines kept above the cursor: `hx` leaves a gap in the middle of a
    /// view that is too small for the full `scrolloff`
    pub fn top_margin(&self) -> usize {
        SCROLLOFF.min(self.visible_lines.saturating_sub(1) / 2)
    }

    /// Lines kept below the cursor
    pub fn bottom_margin(&self) -> usize {
        SCROLLOFF.min(self.visible_lines / 2)
    }

    /// The line a forward page scroll puts the cursor on when it is above it
    pub fn top_band_line(&self) -> usize {
        self.top_line.saturating_add(self.top_margin())
    }

    /// The line a backward page scroll puts the cursor on when it is below it
    pub fn bottom_band_line(&self) -> usize {
        self.top_line
            .saturating_add(self.visible_lines.saturating_sub(self.top_margin() + 1))
    }

    /// Move the view, never the cursor, until `cursor_line` is inside the
    /// `scrolloff` margins (`hx` `ensure_cursor_in_view`)
    pub fn ensure_cursor_in_view(&mut self, cursor_line: usize) {
        let (top_margin, bottom_margin) = (self.top_margin(), self.bottom_margin());
        if cursor_line < self.top_line.saturating_add(top_margin) {
            self.top_line = cursor_line.saturating_sub(top_margin);
        } else if cursor_line.saturating_add(bottom_margin) >= self.bottom_line() {
            let below = self.visible_lines.saturating_sub(bottom_margin + 1);
            self.top_line = cursor_line.saturating_sub(below);
        }
    }

    /// Center the view on `cursor_line` unless it is already inside the view,
    /// margins aside (`hx` `ensure_cursor_in_view_center`, used after a search)
    pub fn center_if_outside(&mut self, cursor_line: usize) {
        if cursor_line < self.top_line || cursor_line >= self.bottom_line() {
            self.top_line = cursor_line.saturating_sub(self.visible_lines.saturating_sub(1) / 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sized(visible_lines: usize, visible_cols: usize) -> ViewState {
        ViewState {
            visible_lines,
            visible_cols,
            ..ViewState::default()
        }
    }

    #[test]
    fn test_view_state_new() {
        let state = ViewState::new();
        assert_eq!(state.top_line(), 0);
        assert_eq!(state.left_col(), 0);
        assert_eq!(state.visible_lines(), DEFAULT_VISIBLE_LINES);
        assert_eq!(state.visible_cols(), DEFAULT_VISIBLE_COLS);
    }

    #[test]
    fn test_center_on_line() {
        let mut state = sized(10, 80);

        // Cursor in middle of document
        state.center_on_line(50, 100);
        assert_eq!(state.top_line(), 45);

        // Cursor near start
        state.center_on_line(3, 100);
        assert_eq!(state.top_line(), 0);

        // Cursor near end
        state.center_on_line(98, 100);
        assert_eq!(state.top_line(), 90);
    }

    #[test]
    fn test_align_top() {
        let mut state = sized(10, 80);
        state.align_top(25, 100);
        assert_eq!(state.top_line(), 25);
    }

    #[test]
    fn test_align_bottom() {
        let mut state = sized(10, 80);
        state.align_bottom(25, 100);
        assert_eq!(state.top_line(), 16);
    }

    #[test]
    fn test_center_on_col() {
        let mut state = sized(10, 80);

        // Column in middle
        state.center_on_col(100, 200);
        assert_eq!(state.left_col(), 60);

        // Column near start
        state.center_on_col(10, 200);
        assert_eq!(state.left_col(), 0);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = sized(10, 80);

        state.scroll_down(5, 100);
        assert_eq!(state.top_line(), 5);

        // The last line may reach the top of the view
        state.scroll_down(100, 100);
        assert_eq!(state.top_line(), 99);
    }

    #[test]
    fn test_scroll_up() {
        let mut state = sized(10, 80);
        state.top_line = 20;

        state.scroll_up(5);
        assert_eq!(state.top_line(), 15);

        // Don't scroll past start
        state.scroll_up(100);
        assert_eq!(state.top_line(), 0);
    }

    #[test]
    fn margins_leave_a_gap_in_a_small_view() {
        assert_eq!(sized(24, 80).top_margin(), 5);
        assert_eq!(sized(24, 80).bottom_margin(), 5);
        assert_eq!(
            (sized(6, 80).top_margin(), sized(6, 80).bottom_margin()),
            (2, 3)
        );
        assert_eq!(
            (sized(1, 80).top_margin(), sized(1, 80).bottom_margin()),
            (0, 0)
        );
    }

    #[test]
    fn band_lines_follow_the_margins() {
        let mut state = sized(24, 80);
        state.top_line = 10;
        assert_eq!(state.top_band_line(), 15);
        assert_eq!(state.bottom_band_line(), 28);
    }

    #[test]
    fn ensure_cursor_in_view_moves_the_view_just_enough() {
        let mut state = sized(24, 80);
        state.ensure_cursor_in_view(10);
        assert_eq!(state.top_line(), 0);
        state.ensure_cursor_in_view(29);
        assert_eq!(state.top_line(), 11);
        state.ensure_cursor_in_view(12);
        assert_eq!(state.top_line(), 7);
        state.ensure_cursor_in_view(0);
        assert_eq!(state.top_line(), 0);
    }

    #[test]
    fn center_if_outside_leaves_a_visible_cursor_alone() {
        let mut state = sized(24, 80);
        state.top_line = 10;
        state.center_if_outside(12);
        assert_eq!(state.top_line(), 10);
        state.center_if_outside(50);
        assert_eq!(state.top_line(), 39);
        state.center_if_outside(3);
        assert_eq!(state.top_line(), 0);
    }

    #[test]
    fn test_bottom_line() {
        let mut state = sized(10, 80);
        state.top_line = 20;
        assert_eq!(state.bottom_line(), 30);
    }

    #[test]
    fn test_set_size() {
        let mut state = ViewState::new();
        state.set_size(50, 100);
        assert_eq!(state.visible_lines(), 50);
        assert_eq!(state.visible_cols(), 100);
    }

    #[test]
    fn test_saturating_add_no_overflow() {
        let mut state = ViewState::new();
        state.top_line = usize::MAX - 10;
        state.visible_lines = 20;

        // Should not overflow
        let bottom = state.bottom_line();
        assert_eq!(bottom, usize::MAX);
    }
}
