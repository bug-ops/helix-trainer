//! Pure layout arithmetic shared by view-state normalization and rendering
//!
//! The visible rows of the menu and the achievements list depend on the terminal size. State
//! normalization (`AppState::normalize_view`) and the renderers both derive them from the
//! functions here, so the clamped state and the drawn frame cannot disagree.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Terminal size in cells, kept in UI state and updated from the terminal resize event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Viewport {
    /// Width in columns
    pub cols: u16,
    /// Height in rows
    pub rows: u16,
}

impl Viewport {
    /// Minimum terminal size the interface supports, used until the real size is known
    pub const MINIMUM: Self = Self::new(80, 24);

    /// A viewport of `cols` by `rows` cells
    #[must_use]
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// The viewport as a layout area anchored at the origin
    #[must_use]
    pub const fn area(self) -> Rect {
        Rect::new(0, 0, self.cols, self.rows)
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::MINIMUM
    }
}

/// Split the main menu screen: header, title, list, quest panel, instructions
pub(crate) fn menu_chunks(area: Rect) -> [Rect; 5] {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(1), // Header with level/XP/streak
            Constraint::Length(3), // Title
            Constraint::Min(4),    // Menu items
            Constraint::Length(6), // Quest panel
            Constraint::Length(3), // Instructions
        ])
        .areas(area)
}

/// Split the achievements screen: title, list, instructions
///
/// `margin(1)`, not the Statistics/Profile screens' `margin(2)`: at the minimum supported
/// terminal size (80x24), `Length(3)+Min(15)+Length(3)=21` rows does not fit the 20 rows
/// `margin(2)` leaves, and the layout solver would silently squeeze the instructions section
/// to 2 rows (no usable content height inside its border), swallowing the achievement count
/// and scroll position indicator.
pub(crate) fn achievement_chunks(area: Rect) -> [Rect; 3] {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(15),   // Content
            Constraint::Length(3), // Instructions
        ])
        .areas(area)
}

/// Rows of the menu list that fit inside its border at `viewport`
#[must_use]
pub(crate) fn menu_list_rows(viewport: Viewport) -> usize {
    usize::from(menu_chunks(viewport.area())[2].height.saturating_sub(2))
}

/// Rows of the achievements list that fit inside its border at `viewport`
#[must_use]
pub(crate) fn achievement_list_rows(viewport: Viewport) -> usize {
    usize::from(
        achievement_chunks(viewport.area())[1]
            .height
            .saturating_sub(2),
    )
}

/// Number of fixed, selectable menu entries after the scenario list: Review, Profile, Statistics, Quit.
///
pub(crate) const FIXED_MENU_ITEMS: usize = 4;

/// Number of rendered rows appended after the scenario list: the `FIXED_MENU_ITEMS`
/// selectable entries plus the non-selectable separator row `build_menu_items` inserts
/// before them. Used for scroll-offset/scrollbar math, which operates on rendered rows —
/// NOT for user-facing item counts (see `FIXED_MENU_ITEMS` for those).
pub(crate) const RENDERED_FIXED_ROWS: usize = FIXED_MENU_ITEMS + 1;

/// Converts a logical selection index into its rendered-row index.
///
/// `selected_item` is a scenario index (`0..scenario_count`) or one of the fixed entries
/// (`scenario_count..scenario_count+FIXED_MENU_ITEMS`). `build_menu_items` inserts a
/// separator row between the two groups, so any fixed-entry selection sits one row further
/// down than its logical index suggests.
pub(crate) fn selection_to_row(selected_item: usize, scenario_count: usize) -> usize {
    selected_item + usize::from(selected_item >= scenario_count)
}

/// Calculate menu scroll offset to keep selected item visible
///
/// Both `selected` and `total_items` must be in the same coordinate space as the rendered
/// row list (see `selection_to_row`) — this function has no way to detect a space mismatch.
///
/// Returns (`scroll_offset`, `visible_count`) tuple
pub(crate) fn calculate_menu_scroll(
    selected: usize,
    current_offset: usize,
    visible_height: usize,
    total_items: usize,
) -> (usize, usize) {
    // No rows are visible (e.g. a very short terminal collapses the menu area to 0 height) -
    // `visible_height - 1` below would underflow (#321). Return the caller's offset
    // unchanged rather than resetting to 0: nothing is visible to scroll, so there is
    // nothing to correct, and resetting would discard the caller's scroll position for
    // this transient 0-height frame.
    if visible_height == 0 {
        return (current_offset, 0);
    }

    let mut scroll_offset = current_offset;

    // Adjust scroll offset to keep selected item visible
    if selected < scroll_offset {
        // Selected item is above visible area - scroll up
        scroll_offset = selected;
    } else if selected >= scroll_offset + visible_height {
        // Selected item is below visible area - scroll down
        scroll_offset = selected.saturating_sub(visible_height - 1);
    }

    // Clamp scroll offset to valid range
    let max_offset = total_items.saturating_sub(visible_height);
    scroll_offset = scroll_offset.min(max_offset);

    (scroll_offset, visible_height)
}

/// Clamp the menu's selection and scroll offset to a list of `scenario_count` scenarios shown
/// in `visible_rows` rows, returning `(selected_item, scroll_offset)`
///
/// A filter change or resize may have shrunk the list or the window since the values were
/// last set; the selection stays on a real item and the offset keeps it visible.
#[must_use]
pub(crate) fn clamp_menu_view(
    selected_item: usize,
    scroll_offset: usize,
    scenario_count: usize,
    visible_rows: usize,
) -> (usize, usize) {
    let display_total = scenario_count.saturating_add(FIXED_MENU_ITEMS);
    let selected = selected_item.min(display_total.saturating_sub(1));
    let row_total = scenario_count.saturating_add(RENDERED_FIXED_ROWS);
    let (offset, _) = calculate_menu_scroll(
        selection_to_row(selected, scenario_count),
        scroll_offset,
        visible_rows,
        row_total,
    );
    (selected, offset)
}

/// Clamp the achievements scroll offset so the last row stays at the bottom of the list
#[must_use]
pub(crate) fn clamp_achievement_scroll(
    scroll_offset: usize,
    total: usize,
    visible_rows: usize,
) -> usize {
    scroll_offset.min(total.saturating_sub(visible_rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for calculate_menu_scroll()
    mod calculate_menu_scroll_tests {
        use super::*;

        #[test]
        fn test_calculate_menu_scroll_selected_at_top() {
            // Selected item is 0, already at top
            let (offset, _) = calculate_menu_scroll(0, 0, 10, 20);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_at_bottom() {
            // Selected item is 19 (last), visible height 10
            // Should scroll to 10 so items 10-19 are visible
            let (offset, _) = calculate_menu_scroll(19, 0, 10, 20);
            assert_eq!(offset, 10);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_in_middle() {
            // Selected item is 5, already in visible range [0, 10)
            let (offset, _) = calculate_menu_scroll(5, 0, 10, 20);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_above_visible() {
            // Selected item is 3, current offset 5
            // Should scroll up to show item 3
            let (offset, _) = calculate_menu_scroll(3, 5, 10, 20);
            assert_eq!(offset, 3);
        }

        #[test]
        fn test_calculate_menu_scroll_selected_below_visible() {
            // Selected item is 15, current offset 0
            // Visible range is [0, 10), should scroll to show 15
            let (offset, _) = calculate_menu_scroll(15, 0, 10, 20);
            assert_eq!(offset, 6); // 15 - 10 + 1 = 6
        }

        #[test]
        fn test_calculate_menu_scroll_all_items_visible() {
            // Only 5 items total, visible height 10
            // Should not scroll
            let (offset, _) = calculate_menu_scroll(4, 0, 10, 5);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_zero_items() {
            // Edge case: 0 items
            let (offset, _) = calculate_menu_scroll(0, 0, 10, 0);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_one_item() {
            // Edge case: 1 item
            let (offset, _) = calculate_menu_scroll(0, 0, 10, 1);
            assert_eq!(offset, 0);
        }

        #[test]
        fn test_calculate_menu_scroll_max_offset_clamped() {
            // Selected item is beyond total items - should clamp offset
            // Max offset is total_items - visible_height = 20 - 10 = 10
            let (offset, _) = calculate_menu_scroll(19, 15, 10, 20);
            assert_eq!(offset, 10);
        }

        #[test]
        fn test_calculate_menu_scroll_maintains_visibility() {
            // Scrolling down gradually
            let (offset, _) = calculate_menu_scroll(9, 0, 10, 20);
            assert_eq!(offset, 0); // Still visible in [0, 10)

            let (offset, _) = calculate_menu_scroll(10, 0, 10, 20);
            assert_eq!(offset, 1); // Need to scroll to keep 10 visible
        }

        #[test]
        fn test_calculate_menu_scroll_returns_visible_height() {
            let (_, visible) = calculate_menu_scroll(5, 0, 10, 20);
            assert_eq!(visible, 10);
        }

        #[test]
        fn test_calculate_menu_scroll_zero_visible_height_does_not_underflow() {
            // Regression test for #321: at very small terminal heights the menu layout can
            // yield a 0-height visible area, and `visible_height - 1` would underflow.
            // Nothing is visible, so the caller's offset must be preserved rather than reset.
            let (offset, visible) = calculate_menu_scroll(5, 3, 0, 20);
            assert_eq!(offset, 3);
            assert_eq!(visible, 0);
        }
    }

    // Regression tests for #311: Quit must be scrollable into view.
    //
    // `calculate_menu_scroll` operates purely on whatever index space it's given, so a test
    // that only checks its output in isolation (as `calculate_menu_scroll_tests` above does)
    // cannot catch a caller passing it the wrong space. These tests instead combine
    // `selection_to_row` with `calculate_menu_scroll` exactly as `render_main_menu` does, and
    // assert on the property that actually matters: the selected item's *rendered row* must
    // fall inside the visible window after scrolling. A caller that skips `selection_to_row`
    // (passing the logical `scenario_count + FIXED_MENU_ITEMS - 1` index straight through, as
    // the original code and the first, inert `RENDERED_FIXED_ROWS`-only fix both did) computes
    // an offset that is exactly one row short of this for every case below.
    mod quit_reachability_tests {
        use super::*;

        fn assert_quit_row_visible(scenario_count: usize, visible_height: usize) {
            let quit_selected_item = scenario_count + FIXED_MENU_ITEMS - 1;
            let row_total = scenario_count + RENDERED_FIXED_ROWS;

            // Ground truth, independent of `selection_to_row` (the function under test):
            // `build_menu_items` always lays out scenario rows, then 1 separator row, then
            // the 4 fixed entries, so Quit's physical row is always `row_total - 1`. Deriving
            // this from `selection_to_row` instead would make the assertion vacuous against a
            // broken (e.g. identity) `selection_to_row`, since `calculate_menu_scroll` always
            // makes whatever row it's given visible.
            let true_quit_row = row_total - 1;

            let scroll_input_row = selection_to_row(quit_selected_item, scenario_count);
            let (offset, _) = calculate_menu_scroll(scroll_input_row, 0, visible_height, row_total);

            assert!(
                offset <= true_quit_row && true_quit_row < offset + visible_height,
                "Quit's true rendered row {true_quit_row} not inside visible window \
                 [{offset}, {}) for scenario_count={scenario_count}, visible_height={visible_height} \
                 (selection_to_row produced {scroll_input_row})",
                offset + visible_height
            );
        }

        #[test]
        fn test_quit_reachable_critic_counterexample_10_5() {
            assert_quit_row_visible(10, 5);
        }

        #[test]
        fn test_quit_reachable_critic_counterexample_20_10() {
            assert_quit_row_visible(20, 10);
        }

        #[test]
        fn test_quit_reachable_critic_counterexample_50_12() {
            assert_quit_row_visible(50, 12);
        }

        #[test]
        fn test_quit_reachable_large_scenario_count() {
            assert_quit_row_visible(200, 20);
        }

        #[test]
        fn test_quit_reachable_visible_height_of_one() {
            // Degenerate but valid: a single visible row must still be able to show Quit.
            assert_quit_row_visible(10, 1);
        }
    }

    #[test]
    fn test_viewport_default_is_the_minimum_supported_size() {
        assert_eq!(Viewport::default(), Viewport::new(80, 24));
    }

    #[test]
    fn test_list_rows_at_minimum_size_match_the_drawn_layout() {
        assert_eq!(menu_list_rows(Viewport::new(80, 24)), 5);
        assert_eq!(achievement_list_rows(Viewport::new(80, 24)), 14);
    }

    #[test]
    fn test_list_rows_saturate_on_a_degenerate_viewport() {
        assert_eq!(menu_list_rows(Viewport::new(0, 0)), 0);
        assert_eq!(achievement_list_rows(Viewport::new(80, 0)), 0);
    }

    #[test]
    fn test_clamp_menu_view_pulls_selection_into_a_shrunken_list() {
        let (selected, offset) = clamp_menu_view(40, 30, 5, 3);
        assert_eq!(selected, 5 + FIXED_MENU_ITEMS - 1);
        assert!(offset <= selection_to_row(selected, 5));
        assert!(selection_to_row(selected, 5) < offset + 3);
    }

    #[test]
    fn test_clamp_menu_view_with_zero_visible_rows_keeps_selection_valid() {
        let (selected, _) = clamp_menu_view(99, 0, 2, 0);
        assert_eq!(selected, 2 + FIXED_MENU_ITEMS - 1);
    }

    #[test]
    fn test_clamp_achievement_scroll_stops_at_the_last_page() {
        assert_eq!(clamp_achievement_scroll(100, 30, 10), 20);
        assert_eq!(clamp_achievement_scroll(3, 8, 10), 0);
    }
}
