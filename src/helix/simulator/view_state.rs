//! View state management for Helix simulator
//!
//! Provides viewport tracking for view commands (z, zt, zb, zm, zj, zk).
//! The view state tracks the visible portion of the document.

/// Default number of visible lines in the viewport
const DEFAULT_VISIBLE_LINES: usize = 24;

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

    /// Create a view state with custom viewport size
    pub fn with_size(visible_lines: usize, visible_cols: usize) -> Self {
        Self {
            top_line: 0,
            visible_lines,
            left_col: 0,
            visible_cols,
        }
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
    /// Uses saturating_add to prevent overflow on extreme values.
    pub fn bottom_line(&self) -> usize {
        self.top_line.saturating_add(self.visible_lines)
    }

    /// Get the last visible column (exclusive)
    ///
    /// Uses saturating_add to prevent overflow on extreme values.
    pub fn right_col(&self) -> usize {
        self.left_col.saturating_add(self.visible_cols)
    }

    /// Set the viewport size
    pub fn set_size(&mut self, visible_lines: usize, visible_cols: usize) {
        self.visible_lines = visible_lines;
        self.visible_cols = visible_cols;
    }

    /// Center the viewport vertically on a given line
    ///
    /// Adjusts top_line so that cursor_line is in the center of the viewport.
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

    /// Scroll down by count lines
    pub fn scroll_down(&mut self, count: usize, total_lines: usize) {
        let max_top = total_lines.saturating_sub(self.visible_lines);
        self.top_line = (self.top_line + count).min(max_top);
    }

    /// Scroll up by count lines
    pub fn scroll_up(&mut self, count: usize) {
        self.top_line = self.top_line.saturating_sub(count);
    }

    /// Scroll right by count columns
    ///
    /// Uses saturating_add to prevent overflow.
    /// Note: No upper bound check is performed here since line width varies.
    /// Call site should ensure reasonable bounds if needed.
    pub fn scroll_right(&mut self, count: usize, max_col: Option<usize>) {
        let new_col = self.left_col.saturating_add(count);
        self.left_col = match max_col {
            Some(max) => new_col.min(max.saturating_sub(self.visible_cols)),
            None => new_col,
        };
    }

    /// Scroll left by count columns
    pub fn scroll_left(&mut self, count: usize) {
        self.left_col = self.left_col.saturating_sub(count);
    }

    /// Check if a line is visible in the viewport
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.top_line && line < self.bottom_line()
    }

    /// Ensure a line is visible, scrolling if necessary
    ///
    /// Returns true if scrolling occurred.
    pub fn ensure_line_visible(&mut self, line: usize, total_lines: usize) -> bool {
        if line < self.top_line {
            self.top_line = line;
            true
        } else if line >= self.bottom_line() {
            self.top_line = line.saturating_sub(self.visible_lines.saturating_sub(1));
            self.top_line = self
                .top_line
                .min(total_lines.saturating_sub(self.visible_lines));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_state_new() {
        let state = ViewState::new();
        assert_eq!(state.top_line(), 0);
        assert_eq!(state.left_col(), 0);
        assert_eq!(state.visible_lines(), DEFAULT_VISIBLE_LINES);
        assert_eq!(state.visible_cols(), DEFAULT_VISIBLE_COLS);
    }

    #[test]
    fn test_view_state_with_size() {
        let state = ViewState::with_size(40, 120);
        assert_eq!(state.visible_lines(), 40);
        assert_eq!(state.visible_cols(), 120);
    }

    #[test]
    fn test_center_on_line() {
        let mut state = ViewState::with_size(10, 80);

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
        let mut state = ViewState::with_size(10, 80);
        state.align_top(25, 100);
        assert_eq!(state.top_line(), 25);
    }

    #[test]
    fn test_align_bottom() {
        let mut state = ViewState::with_size(10, 80);
        state.align_bottom(25, 100);
        assert_eq!(state.top_line(), 16);
    }

    #[test]
    fn test_center_on_col() {
        let mut state = ViewState::with_size(10, 80);

        // Column in middle
        state.center_on_col(100, 200);
        assert_eq!(state.left_col(), 60);

        // Column near start
        state.center_on_col(10, 200);
        assert_eq!(state.left_col(), 0);
    }

    #[test]
    fn test_scroll_down() {
        let mut state = ViewState::with_size(10, 80);

        state.scroll_down(5, 100);
        assert_eq!(state.top_line(), 5);

        // Don't scroll past end
        state.scroll_down(100, 100);
        assert_eq!(state.top_line(), 90);
    }

    #[test]
    fn test_scroll_up() {
        let mut state = ViewState::with_size(10, 80);
        state.top_line = 20;

        state.scroll_up(5);
        assert_eq!(state.top_line(), 15);

        // Don't scroll past start
        state.scroll_up(100);
        assert_eq!(state.top_line(), 0);
    }

    #[test]
    fn test_is_line_visible() {
        let mut state = ViewState::with_size(10, 80);
        state.top_line = 20;

        assert!(!state.is_line_visible(19));
        assert!(state.is_line_visible(20));
        assert!(state.is_line_visible(25));
        assert!(state.is_line_visible(29));
        assert!(!state.is_line_visible(30));
    }

    #[test]
    fn test_ensure_line_visible() {
        let mut state = ViewState::with_size(10, 80);
        state.top_line = 20;

        // Line already visible
        assert!(!state.ensure_line_visible(25, 100));
        assert_eq!(state.top_line(), 20);

        // Line above viewport
        assert!(state.ensure_line_visible(10, 100));
        assert_eq!(state.top_line(), 10);

        // Line below viewport
        state.top_line = 20;
        assert!(state.ensure_line_visible(35, 100));
        assert_eq!(state.top_line(), 26);
    }

    #[test]
    fn test_bottom_line() {
        let mut state = ViewState::with_size(10, 80);
        state.top_line = 20;
        assert_eq!(state.bottom_line(), 30);
    }

    #[test]
    fn test_right_col() {
        let mut state = ViewState::with_size(10, 80);
        state.left_col = 20;
        assert_eq!(state.right_col(), 100);
    }

    #[test]
    fn test_set_size() {
        let mut state = ViewState::new();
        state.set_size(50, 100);
        assert_eq!(state.visible_lines(), 50);
        assert_eq!(state.visible_cols(), 100);
    }

    #[test]
    fn test_scroll_right_left() {
        let mut state = ViewState::new();

        state.scroll_right(10, None);
        assert_eq!(state.left_col(), 10);

        state.scroll_left(5);
        assert_eq!(state.left_col(), 5);

        state.scroll_left(100);
        assert_eq!(state.left_col(), 0);
    }

    #[test]
    fn test_scroll_right_with_bounds() {
        let mut state = ViewState::with_size(10, 80);

        // Scroll with max_col bound - within limit
        state.scroll_right(100, Some(200));
        // max_col is 200, visible_cols is 80, so max left_col is 200-80=120
        // Scrolling by 100 from 0 gives 100, which is less than 120, so result is 100
        assert_eq!(state.left_col(), 100);

        // Scroll more to hit the limit
        state.scroll_right(50, Some(200));
        // 100 + 50 = 150, clamped to 120
        assert_eq!(state.left_col(), 120);

        // Reset and test without bounds
        state.left_col = 0;
        state.scroll_right(100, None);
        assert_eq!(state.left_col(), 100);
    }

    #[test]
    fn test_saturating_add_no_overflow() {
        let mut state = ViewState::new();
        state.top_line = usize::MAX - 10;
        state.visible_lines = 20;

        // Should not overflow
        let bottom = state.bottom_line();
        assert_eq!(bottom, usize::MAX);

        state.left_col = usize::MAX - 10;
        state.visible_cols = 20;
        let right = state.right_col();
        assert_eq!(right, usize::MAX);
    }
}
