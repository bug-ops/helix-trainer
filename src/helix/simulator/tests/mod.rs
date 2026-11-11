//! Tests for `HelixSimulator` organized by functionality
//!
//! This module contains comprehensive tests for the Helix editor simulator:
//! - `basic_tests`: Core simulator creation and initialization
//! - `movement_tests`: Cursor movement commands (h, l, j, k, w, b, e, etc.)
//! - `editing_tests`: Text modification commands (delete, undo, indent, etc.)
//! - `selection_tests`: Selection operations (x, %, etc.)
//! - `mode_tests`: Mode transitions and insert mode operations
//! - `clipboard_tests`: Yank and paste operations
//! - `repeat_recording_tests`: Repeat buffer recording verification
//! - `repeat_execution_tests`: Dot command execution tests
//! - `multi_cursor_tests`: Multi-cursor snapshot round-trips (Issue #141)

mod basic_tests;
mod clipboard_tests;
mod editing_tests;
mod hx_differential_tests;
mod indent_tests;
mod insert_indent_replace_tests;
mod insert_keys_tests;
mod insert_session_tests;
mod language_tests;
mod macro_tests;
mod mode_tests;
mod movement_tests;
mod multi_cursor_insert_tests;
mod multi_cursor_tests;
mod newline_indent_rule_tests;
mod newline_indent_tests;
mod paste_tests;
mod repeat_execution_tests;
mod repeat_recording_tests;
mod select_mode_extend_tests;
mod select_mode_tests;
mod selection_bounds_tests;
mod selection_tests;

use crate::helix::simulator::{AnyModeSimulator, EditorSnapshot};

/// Builds a Normal-mode simulator with the cursor at `(row, col)`.
pub(crate) fn sim_at_cursor(content: &str, row: usize, col: usize) -> AnyModeSimulator {
    AnyModeSimulator::from_snapshot(&EditorSnapshot::from_row_col(content.to_string(), row, col))
}

/// Builds a Normal-mode simulator whose primary selection runs from
/// `[start_row, start_col]` (anchor) to `[end_row, end_col]` (head).
pub(crate) fn sim_with_selection(content: &str, sel: [usize; 4]) -> AnyModeSimulator {
    AnyModeSimulator::from_snapshot(&EditorSnapshot::from_row_col_selection(
        content.to_string(),
        sel,
    ))
}

/// `(anchor, head)` of every selection range, for exact selection assertions.
pub(crate) fn ranges_of(sim: &AnyModeSimulator) -> Vec<(usize, usize)> {
    sim.to_snapshot()
        .selections
        .iter()
        .map(|range| (range.anchor, range.head))
        .collect()
}

/// Whether every selection range lies inside the document.
pub(crate) fn selection_in_bounds(sim: &AnyModeSimulator) -> bool {
    let snapshot = sim.to_snapshot();
    let len = snapshot.content.chars().count();
    snapshot
        .selections
        .iter()
        .all(|range| range.anchor <= len && range.head <= len)
}

/// Scenario 14 of the bundled set, the text the `hx` differential tests run on.
pub(crate) const SCENARIO_14: &str =
    "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n";

/// Runs `keys` on a fresh Normal-mode simulator over `text`, unwrapping every command.
pub(crate) fn run_keys(text: &str, keys: &[&str]) -> AnyModeSimulator {
    run_keys_in("txt", text, keys)
}

/// Like [`run_keys`] for a document of the given language.
pub(crate) fn run_keys_in(language: &str, text: &str, keys: &[&str]) -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(text.to_string()).with_language(language);
    for key in keys {
        sim.execute_command(key)
            .unwrap_or_else(|err| panic!("`{key}` failed: {err}"));
    }
    sim
}

/// Content after `keys` and the probe `c @ Escape`, which shows where the
/// selection a command left behind sits (it is replaced by `@`).
pub(crate) fn probed(text: &str, language: &str, keys: &[&str]) -> String {
    let keys: Vec<&str> = keys.iter().copied().chain(["c", "@", "Escape"]).collect();
    run_keys_in(language, text, &keys).display().content()
}
