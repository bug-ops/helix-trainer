//! View commands (z, zt, zb, zm, zj, zk) and the paging keys (Ctrl-b, Ctrl-f,
//! Ctrl-u, Ctrl-d), which share one scroll of a fixed logical view

use super::movement::move_vertically_all;
use crate::helix::error::SimulatorError;
use crate::helix::simulator::{EditorMode, HelixSimulator};
use helix_core::Range;
use helix_core::movement::{Direction, Movement};

/// Whether a scroll takes the cursor along (`hx` `sync_cursor`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorSync {
    /// Every range moves as many lines as the view
    Follow,
    /// The cursor stays unless the view scrolled past it
    Keep,
}

/// Scroll the view `offset` lines in `direction`, as `hx` `scroll`
///
/// With [`CursorSync::Follow`] every range moves by the same number of lines,
/// extending in Select mode. With [`CursorSync::Keep`] the cursor stays unless
/// the view left it behind the scrolloff band: then the primary range becomes
/// a point at the start of the band's edge line (in Select mode only its head
/// moves).
fn scroll<M: EditorMode>(
    sim: &mut HelixSimulator<M>,
    offset: usize,
    direction: Direction,
    sync: CursorSync,
) {
    let total_lines = sim.doc.len_lines();
    match direction {
        Direction::Forward => sim.view_state.scroll_down(offset, total_lines),
        Direction::Backward => sim.view_state.scroll_up(offset),
    }

    if sync == CursorSync::Follow {
        let movement = if sim.select_mode {
            Movement::Extend
        } else {
            Movement::Move
        };
        move_vertically_all(sim, direction, offset, movement);
        return;
    }

    let text = sim.doc.slice(..);
    let primary = sim.selection.primary();
    let band_line = match direction {
        Direction::Forward => sim.view_state.top_band_line(),
        Direction::Backward => sim.view_state.bottom_band_line(),
    };
    let head = text.line_to_char(band_line.min(total_lines));
    let cursor = primary.cursor(text);
    let cursor_in_band = match direction {
        Direction::Forward => head <= cursor,
        Direction::Backward => head >= cursor,
    };
    if cursor_in_band {
        return;
    }
    let anchor = if sim.select_mode {
        primary.anchor
    } else {
        head
    };
    let primary_index = sim.selection.primary_index();
    sim.selection = sim
        .selection
        .clone()
        .replace(primary_index, Range::new(anchor, head));
}

/// Move the view, never the cursor, until the cursor is inside the `scrolloff`
/// margins: what `hx` does after every key
pub(super) fn ensure_cursor_in_view<M: EditorMode>(sim: &mut HelixSimulator<M>) {
    let cursor = sim.selection.primary().cursor(sim.doc.slice(..));
    sim.view_state
        .ensure_cursor_in_view(sim.doc.char_to_line(cursor));
}

/// Center the view on the cursor when a search put it outside the view
pub(super) fn center_if_outside_view<M: EditorMode>(sim: &mut HelixSimulator<M>) {
    let cursor = sim.selection.primary().cursor(sim.doc.slice(..));
    sim.view_state
        .center_if_outside(sim.doc.char_to_line(cursor));
}

/// Page up (Ctrl-b command): scroll one view height, the cursor stays unless
/// the view passed it
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn page_up<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let height = sim.view_state.visible_lines();
    scroll(sim, height, Direction::Backward, CursorSync::Keep);
    Ok(())
}

/// Page down (Ctrl-f command): scroll one view height, the cursor stays unless
/// the view passed it
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn page_down<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let height = sim.view_state.visible_lines();
    scroll(sim, height, Direction::Forward, CursorSync::Keep);
    Ok(())
}

/// Half page up (Ctrl-u command): scroll half a view height and move every
/// range with it
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn half_page_up<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let half = sim.view_state.visible_lines() / 2;
    scroll(sim, half, Direction::Backward, CursorSync::Follow);
    Ok(())
}

/// Half page down (Ctrl-d command): scroll half a view height and move every
/// range with it
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn half_page_down<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    let half = sim.view_state.visible_lines() / 2;
    scroll(sim, half, Direction::Forward, CursorSync::Follow);
    Ok(())
}

/// Center viewport on cursor line (z or zz command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn view_center<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    sim.view_state.center_on_line(cursor_line, total_lines);
    Ok(())
}

/// Align cursor line to top of viewport (zt command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn view_align_top<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    sim.view_state.align_top(cursor_line, total_lines);
    Ok(())
}

/// Align cursor line to bottom of viewport (zb command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn view_align_bottom<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    sim.view_state.align_bottom(cursor_line, total_lines);
    Ok(())
}

/// Center viewport horizontally on cursor column (zm command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn view_center_horizontal<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), SimulatorError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(cursor_line);
    let cursor_col = head - line_start;
    let line_end = if cursor_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(cursor_line + 1)
    } else {
        sim.doc.len_chars()
    };
    let line_width = line_end - line_start;

    sim.view_state.center_on_col(cursor_col, line_width);
    Ok(())
}

/// Scroll viewport down by one line (zj command); the cursor stays in the
/// scrolloff band
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn scroll_down<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    scroll(sim, 1, Direction::Forward, CursorSync::Keep);
    Ok(())
}

/// Scroll viewport up by one line (zk command); the cursor stays in the
/// scrolloff band
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn scroll_up<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), SimulatorError> {
    scroll(sim, 1, Direction::Backward, CursorSync::Keep);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;
    use helix_core::Selection;

    fn create_multiline_sim(lines: usize) -> HelixSimulator<NormalMode> {
        let content = (0..lines)
            .map(|i| format!("line {i}\n"))
            .collect::<Vec<_>>()
            .concat();
        HelixSimulator::new(content)
    }

    #[test]
    fn test_view_center() {
        let mut sim = create_multiline_sim(100);

        // Move cursor to line 50
        let line_start = sim.doc.line_to_char(50);
        sim.selection = Selection::point(line_start);

        // Set viewport size for predictable results
        sim.view_state.set_size(20, 80);

        view_center(&mut sim).unwrap();

        // Cursor should be centered (line 50 - 10 = line 40 at top)
        assert_eq!(sim.view_state.top_line(), 40);
    }

    #[test]
    fn test_view_align_top() {
        let mut sim = create_multiline_sim(100);

        // Move cursor to line 30
        let line_start = sim.doc.line_to_char(30);
        sim.selection = Selection::point(line_start);

        view_align_top(&mut sim).unwrap();

        // Cursor line should be at top
        assert_eq!(sim.view_state.top_line(), 30);
    }

    #[test]
    fn test_view_align_bottom() {
        let mut sim = create_multiline_sim(100);

        // Move cursor to line 30
        let line_start = sim.doc.line_to_char(30);
        sim.selection = Selection::point(line_start);

        // Set viewport size
        sim.view_state.set_size(20, 80);

        view_align_bottom(&mut sim).unwrap();

        // Cursor line should be at bottom (30 - 19 = 11)
        assert_eq!(sim.view_state.top_line(), 11);
    }

    #[test]
    fn test_scroll_down() {
        let mut sim = create_multiline_sim(100);
        sim.view_state.set_size(20, 80);

        assert_eq!(sim.view_state.top_line(), 0);

        scroll_down(&mut sim).unwrap();
        assert_eq!(sim.view_state.top_line(), 1);

        scroll_down(&mut sim).unwrap();
        assert_eq!(sim.view_state.top_line(), 2);
    }

    #[test]
    fn test_scroll_up() {
        let mut sim = create_multiline_sim(100);
        sim.view_state.set_size(20, 80);

        // Start at line 10
        sim.view_state.scroll_down(10, 100);
        assert_eq!(sim.view_state.top_line(), 10);

        scroll_up(&mut sim).unwrap();
        assert_eq!(sim.view_state.top_line(), 9);

        scroll_up(&mut sim).unwrap();
        assert_eq!(sim.view_state.top_line(), 8);
    }

    #[test]
    fn test_scroll_down_at_end() {
        let mut sim = create_multiline_sim(50);
        sim.view_state.set_size(20, 80);

        // Try to scroll past end
        for _ in 0..100 {
            scroll_down(&mut sim).unwrap();
        }

        // The last line may reach the top of the view, as in hx
        // Note: create_multiline_sim creates lines with trailing \n, so rope may see 51 lines
        let last_line = sim.doc.len_lines() - 1;
        assert_eq!(sim.view_state.top_line(), last_line);
    }

    #[test]
    fn test_scroll_up_at_start() {
        let mut sim = create_multiline_sim(50);
        sim.view_state.set_size(20, 80);

        // Try to scroll past start
        for _ in 0..10 {
            scroll_up(&mut sim).unwrap();
        }

        // Should stay at 0
        assert_eq!(sim.view_state.top_line(), 0);
    }

    #[test]
    fn test_view_center_horizontal() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new(
            "short\nvery long line with lots of content here\nshort".to_string(),
        );

        // Move cursor to column 30 of long line
        let line_start = sim.doc.line_to_char(1);
        sim.selection = Selection::point(line_start + 30);

        sim.view_state.set_size(20, 40);

        view_center_horizontal(&mut sim).unwrap();

        // Column 30 should be centered (30 - 20 = 10)
        assert_eq!(sim.view_state.left_col(), 10);
    }
}
