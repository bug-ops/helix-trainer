//! View commands (z, zt, zb, zm, zj, zk)
//!
//! Provides viewport control for the Helix simulator.

use crate::helix::simulator::HelixSimulator;
use crate::security::UserError;

/// Center viewport on cursor line (z or zz command)
pub fn view_center<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    sim.view_state.center_on_line(cursor_line, total_lines);
    Ok(())
}

/// Align cursor line to top of viewport (zt command)
pub fn view_align_top<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    sim.view_state.align_top(cursor_line, total_lines);
    Ok(())
}

/// Align cursor line to bottom of viewport (zb command)
pub fn view_align_bottom<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let total_lines = sim.doc.len_lines();

    sim.view_state.align_bottom(cursor_line, total_lines);
    Ok(())
}

/// Center viewport horizontally on cursor column (zm command)
pub fn view_center_horizontal<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let head = sim.selection.primary().head;
    let cursor_line = sim.doc.char_to_line(head);
    let line_start = sim.doc.line_to_char(cursor_line);
    let cursor_col = head - line_start;

    // Get line width (approximate)
    let line_end = if cursor_line + 1 < sim.doc.len_lines() {
        sim.doc.line_to_char(cursor_line + 1)
    } else {
        sim.doc.len_chars()
    };
    let line_width = line_end - line_start;

    sim.view_state.center_on_col(cursor_col, line_width);
    Ok(())
}

/// Scroll viewport down by one line (zj command)
pub fn scroll_down<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    let total_lines = sim.doc.len_lines();
    sim.view_state.scroll_down(1, total_lines);
    Ok(())
}

/// Scroll viewport up by one line (zk command)
pub fn scroll_up<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
) -> Result<(), UserError> {
    sim.view_state.scroll_up(1);
    Ok(())
}

/// Scroll viewport down by count lines
pub fn scroll_down_count<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    let total_lines = sim.doc.len_lines();
    sim.view_state.scroll_down(count, total_lines);
    Ok(())
}

/// Scroll viewport up by count lines
pub fn scroll_up_count<M: crate::helix::simulator::EditorMode>(
    sim: &mut HelixSimulator<M>,
    count: usize,
) -> Result<(), UserError> {
    sim.view_state.scroll_up(count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;
    use helix_core::Selection;

    fn create_multiline_sim(lines: usize) -> HelixSimulator<NormalMode> {
        let content: String = (0..lines).map(|i| format!("line {}\n", i)).collect();
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
    fn test_scroll_down_count() {
        let mut sim = create_multiline_sim(100);
        sim.view_state.set_size(20, 80);

        scroll_down_count(&mut sim, 5).unwrap();
        assert_eq!(sim.view_state.top_line(), 5);
    }

    #[test]
    fn test_scroll_up_count() {
        let mut sim = create_multiline_sim(100);
        sim.view_state.set_size(20, 80);

        // Start at line 10
        sim.view_state.scroll_down(10, 100);

        scroll_up_count(&mut sim, 5).unwrap();
        assert_eq!(sim.view_state.top_line(), 5);
    }

    #[test]
    fn test_scroll_down_at_end() {
        let mut sim = create_multiline_sim(50);
        sim.view_state.set_size(20, 80);

        // Try to scroll past end
        for _ in 0..100 {
            scroll_down(&mut sim).unwrap();
        }

        // Should stop at max (total_lines - visible_lines)
        // Note: create_multiline_sim creates lines with trailing \n, so rope may see 51 lines
        let total_lines = sim.doc.len_lines();
        let expected_max = total_lines.saturating_sub(20);
        assert_eq!(sim.view_state.top_line(), expected_max);
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
