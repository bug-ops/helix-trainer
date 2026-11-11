//! Movement command tests for `HelixSimulator`
//!
//! Tests for cursor movement commands including:
//! - Basic movement: h, l, j, k
//! - Word movement: w, b, e
//! - Line movement: gh, gl, gs
//! - Document navigation: gg, ge

use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Basic Movement Tests (h, l, j, k)
// ============================================================================

#[test]
fn test_move_right() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().1, 1);

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().1, 2);
}

#[test]
fn test_move_left() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Move right twice
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();

    // Move left once
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().1, 1);
}

#[test]
fn test_move_down_up() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 1);

    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 2);

    sim.execute_command(CMD_MOVE_UP).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 1);
}

// ============================================================================
// Word Movement Tests (w, b, e)
// ============================================================================

#[test]
fn test_word_movement() {
    let mut sim = AnyModeSimulator::new("hello world foo".to_string());

    // Move to next word
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.state().unwrap();
    // 'w' extends selection from current position to start of next word
    // helix-core: anchor=0, head=6 (start of "world")
    // state() returns head position for cursor
    assert_eq!(state.cursor_position().1, 6); // Start of "world"

    // Move to next word again
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().1, 12); // Start of "foo"
}

#[test]
fn test_move_word_boundary() {
    let mut sim = AnyModeSimulator::new("  spaced  words  ".to_string());

    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    let state = sim.state().unwrap();
    // Should move to first non-space character of next word
    assert!(state.cursor_position().1 > 0);
}

#[test]
fn test_move_word_end() {
    let mut sim = AnyModeSimulator::new("hello world".to_string());

    sim.execute_command(CMD_MOVE_WORD_END).unwrap();
    let state = sim.state().unwrap();
    // Should be at end of "hello"
    assert!(state.cursor_position().1 >= 4 && state.cursor_position().1 <= 5);
}

#[test]
fn test_move_prev_word() {
    let mut sim = AnyModeSimulator::new("hello world foo".to_string());

    // Move to end of line first (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();
    // Then move to previous word
    sim.execute_command(CMD_MOVE_WORD_BACKWARD).unwrap();

    let state = sim.state().unwrap();
    // Should have moved to start of "foo" (col 12)
    assert_eq!(state.cursor_position().1, 12);
}

// ============================================================================
// Line Movement Tests (gh, gl, gs)
// ============================================================================

#[test]
fn test_move_line_start() {
    let mut sim = AnyModeSimulator::new("hello\nworld\n".to_string());

    // Move to next line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    // Move to end of line (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();
    let state = sim.state().unwrap();
    // Cursor at end of "world" - which is position 4 or 5
    assert!(state.cursor_position().1 >= 4);

    // Move to start of line (use 'gh' in Helix, not '0')
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_goto_first_nonwhitespace() {
    // Test gs command - go to first non-whitespace character
    let mut sim = AnyModeSimulator::new("    fn main() {".to_string());

    // Move cursor to middle of line first
    for _ in 0..10 {
        sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    }
    assert_eq!(sim.state().unwrap().cursor_position().1, 10);

    // Execute gs command
    sim.execute_command(CMD_GOTO_FIRST_NONWHITESPACE).unwrap();
    let state = sim.state().unwrap();
    // Should be at position 4 (first 'f' in "fn")
    assert_eq!(state.cursor_position().1, 4);
}

// ============================================================================
// Document Navigation Tests (gg, ge)
// ============================================================================

#[test]
fn test_document_start() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    // Move somewhere else
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 1);

    // Go back to start
    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_goto_last_line_ge_command() {
    // Test 'ge' command (goto last line)
    let mut sim = AnyModeSimulator::new("Line 1\nLine 2\nLine 3\nLast line".to_string());

    // Cursor starts at (0, 0)
    let state = sim.state().unwrap();
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);

    // Execute 'ge' to go to last line
    let result = sim.execute_command(CMD_GOTO_LAST_LINE);
    assert!(result.is_ok(), "ge command should succeed: {result:?}");

    let state = sim.state().unwrap();
    assert_eq!(
        state.cursor_position().0,
        3,
        "Should be on last line (row 3)"
    );
    assert_eq!(
        state.cursor_position().1,
        0,
        "Cursor should be at start of line"
    );
}

// ============================================================================
// Counted commands, find/till extension, gl, search prompt (#494 #492 #493 #504)
// ============================================================================

const SCENARIO_14: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}";

fn run(sim: &mut AnyModeSimulator, cmds: &[&str]) {
    for cmd in cmds {
        sim.execute_command(cmd).unwrap();
    }
}

fn text_and_ranges(sim: &AnyModeSimulator) -> (String, Vec<(usize, usize)>) {
    let snap = sim.to_snapshot();
    (
        snap.content,
        snap.selections.iter().map(|r| (r.anchor, r.head)).collect(),
    )
}

#[test]
fn test_find_char_then_delete_removes_through_target() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["fa"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 4)]);
    run(&mut sim, &[CMD_DELETE_SELECTION]);
    assert!(text_and_ranges(&sim).0.starts_with("dd(a: i32"));
}

#[test]
fn test_till_char_then_delete_removes_before_target() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["t(", CMD_DELETE_SELECTION]);
    assert!(text_and_ranges(&sim).0.starts_with("(a: i32"));
}

#[test]
fn test_counted_find_accepts_three_char_string() {
    let mut sim = AnyModeSimulator::new("banana".to_string());
    run(&mut sim, &["3fa"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 6)]);
    run(&mut sim, &[CMD_DELETE_SELECTION]);
    assert_eq!(text_and_ranges(&sim).0, "");
}

#[test]
fn test_counted_find_beyond_matches_is_unchanged() {
    let mut sim = AnyModeSimulator::new("banana".to_string());
    run(&mut sim, &["9fa"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 0)]);
}

#[test]
fn test_counted_find_backward_and_till() {
    let mut sim = AnyModeSimulator::new("banana".to_string());
    run(&mut sim, &["2fn"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 5)]);

    // The adjacent `a` is skipped by `t`, so the 2nd match is the 3rd `a`.
    let mut sim = AnyModeSimulator::new("banana".to_string());
    run(&mut sim, &["2ta"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 5)]);
}

#[test]
fn test_counted_goto_line_g_and_gg() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["2G"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(32, 32)]);
    run(&mut sim, &["gg"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 0)]);
    run(&mut sim, &["3gg"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(53, 53)]);
}

#[test]
fn test_counted_goto_line_then_delete_char() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["2G", "x", CMD_DELETE_SELECTION]);
    assert_eq!(
        text_and_ranges(&sim).0,
        "fn add(a: i32, b: i32) -> i32 {\n    sum\n}"
    );
}

#[test]
fn test_goto_line_clamps_to_last_line() {
    let mut sim = AnyModeSimulator::new("a\nb\n".to_string());
    run(&mut sim, &["99G"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(2, 2)]);
}

#[test]
fn test_bare_goto_line_is_noop_without_count() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["2G", "G"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(32, 32)]);
}

#[test]
fn test_repeat_counted_command_applies_n_times() {
    let mut sim = AnyModeSimulator::new("abcdef".to_string());
    run(&mut sim, &["3l"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(3, 3)]);
}

#[test]
fn test_execute_counted_matches_prefixed_string() {
    use std::num::NonZeroUsize;
    let mut a = AnyModeSimulator::new("banana".to_string());
    let mut b = AnyModeSimulator::new("banana".to_string());
    a.execute_counted("fa", NonZeroUsize::new(2).unwrap())
        .unwrap();
    b.execute_command("2fa").unwrap();
    assert_eq!(text_and_ranges(&a), text_and_ranges(&b));
    a.execute_counted("l", NonZeroUsize::new(2).unwrap())
        .unwrap();
    assert_eq!(text_and_ranges(&a).1.len(), 1);
}

#[test]
fn test_count_behavior_metadata() {
    use crate::helix::registry::{CountBehavior, normal_registry};
    let reg = normal_registry();
    assert_eq!(reg.count_behavior("x"), CountBehavior::Repeat);
    assert_eq!(reg.count_behavior(CMD_JOIN_LINES), CountBehavior::Ignore);
    assert_eq!(
        reg.count_behavior(CMD_JOIN_SELECTIONS_SPACE),
        CountBehavior::Ignore
    );
    assert_eq!(
        reg.count_behavior(CMD_GOTO_FILE_START),
        CountBehavior::Native
    );
    assert_eq!(reg.count_behavior(CMD_GOTO_LINE), CountBehavior::Native);
    assert_eq!(reg.count_behavior("fa"), CountBehavior::Native);
    assert_eq!(reg.count_behavior("Tz"), CountBehavior::Native);
}

#[test]
fn test_goto_line_end_lands_on_last_character() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &[CMD_GOTO_LINE_END]);
    assert_eq!(text_and_ranges(&sim).1, vec![(30, 30)]);
    run(&mut sim, &[CMD_DELETE_SELECTION]);
    assert!(
        text_and_ranges(&sim)
            .0
            .starts_with("fn add(a: i32, b: i32) -> i32 \n")
    );
}

#[test]
fn test_goto_line_end_on_last_line_without_trailing_newline() {
    let mut sim = AnyModeSimulator::new("ab\ncd".to_string());
    run(&mut sim, &["j", CMD_GOTO_LINE_END]);
    assert_eq!(text_and_ranges(&sim).1, vec![(4, 4)]);
}

#[test]
fn test_search_prompt_selects_next_match_and_deletes() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["/ sum", CMD_DELETE_SELECTION]);
    assert_eq!(
        text_and_ranges(&sim).0,
        "fn add(a: i32, b: i32) -> i32 {\n    let  = a + b;\n    sum\n}"
    );
}

#[test]
fn test_search_prompt_stores_pattern_for_n() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["/ sum"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(40, 43)]);
    run(&mut sim, &[CMD_SEARCH_NEXT]);
    assert_eq!(text_and_ranges(&sim).1, vec![(57, 60)]);
}

#[test]
fn test_search_prompt_pattern_with_spaces() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["/ a \\+ b"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(46, 51)]);
}

#[test]
fn test_search_prompt_invalid_regex_errs_and_keeps_selection() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    assert!(sim.execute_command("/ (").is_err());
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 0)]);
}

#[test]
fn test_search_prompt_empty_pattern_is_noop() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["/ "]);
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 0)]);
}

// Expected buffers below were recorded from `hx` 25.07.1 on the same keys.
const LET_SUM_DELETED: &str = "fn add(a: i32, b: i32) -> i32 {\n    let  = a + b;\n    sum\n}";
const LAST_SUM_DELETED: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    \n}";

#[test]
fn test_backward_search_wraps_to_last_match_from_buffer_start() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["? sum", CMD_DELETE_SELECTION]);
    assert_eq!(text_and_ranges(&sim).0, LAST_SUM_DELETED);
}

#[test]
fn test_backward_search_selects_previous_match_before_cursor() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["j", "j", "? sum", CMD_DELETE_SELECTION]);
    assert_eq!(text_and_ranges(&sim).0, LET_SUM_DELETED);
}

#[test]
fn test_backward_search_repeated_walks_backward() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["? sum", "? sum", CMD_DELETE_SELECTION]);
    assert_eq!(text_and_ranges(&sim).0, LET_SUM_DELETED);
}

#[test]
fn test_n_after_backward_search_continues_backward() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["? sum", CMD_SEARCH_NEXT, CMD_DELETE_SELECTION]);
    assert_eq!(text_and_ranges(&sim).0, LET_SUM_DELETED);
}

#[test]
fn test_backward_search_finds_match_ending_at_cursor() {
    for steps in [2, 3] {
        let mut sim = AnyModeSimulator::new("abab x".to_string());
        run(&mut sim, &vec![CMD_MOVE_RIGHT; steps]);
        run(&mut sim, &["? ab", CMD_DELETE_SELECTION]);
        assert_eq!(text_and_ranges(&sim).0, "ab x", "cursor at {steps}");
    }
}

#[test]
fn test_backward_search_invalid_regex_errs_and_keeps_selection() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    assert!(sim.execute_command("? (").is_err());
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 0)]);
}

#[test]
fn test_counted_find_spans_lines_on_scenario_14() {
    let mut sim = AnyModeSimulator::new(SCENARIO_14.to_string());
    run(&mut sim, &["3fa"]);
    // 'a' at 3 and 7 on line 0, third 'a' at line 1 col 14 (offset 46)
    assert_eq!(text_and_ranges(&sim).1, vec![(0, 47)]);
}

#[test]
fn test_word_forward_from_last_char_of_word_selects_next_word() {
    // helix-core `word_move`: a cursor on the last char of a word is already at a
    // boundary, so the first `w` selects the following word (`: `), not `x`.
    let mut sim = AnyModeSimulator::new("struct Point {\n    x: i32,\n    y: i32,\n}".to_string());
    run(&mut sim, &["j", "l", "l", "l", "l"]);
    run(&mut sim, &["w"]);
    assert_eq!(text_and_ranges(&sim).1, vec![(20, 22)]);
}

fn content_after(cmds: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new("abcdef".to_string());
    run(&mut sim, cmds);
    text_and_ranges(&sim).0
}

#[test]
fn test_count_is_ignored_by_delete_and_change() {
    assert_eq!(content_after(&["3d"]), "bcdef");
    assert_eq!(content_after(&["3Alt-d"]), "bcdef");
    assert_eq!(content_after(&["3\"_d"]), "bcdef");
    assert_eq!(content_after(&["3c", "Z", "Escape"]), "Zbcdef");
    assert_eq!(content_after(&["3Alt-c", "Z", "Escape"]), "Zbcdef");
    assert_eq!(content_after(&["3\"_c", "Z", "Escape"]), "Zbcdef");
}

#[test]
fn test_count_is_ignored_by_insert_entry_commands() {
    assert_eq!(content_after(&["3i", "Z", "Escape"]), "Zabcdef");
    assert_eq!(content_after(&["3a", "Z", "Escape"]), "aZbcdef");
    assert_eq!(content_after(&["3I", "Z", "Escape"]), "Zabcdef");
    assert_eq!(content_after(&["3A", "Z", "Escape"]), "abcdefZ");
}

#[test]
fn test_repeat_loop_stops_when_command_enters_insert_mode() {
    use crate::helix::registry::{CountBehavior, normal_registry};
    use std::num::NonZeroUsize;
    // `o` is not count-Ignore; the loop must still stop once Insert mode is entered.
    assert_ne!(
        normal_registry().count_behavior(CMD_OPEN_BELOW),
        CountBehavior::Ignore
    );
    let mut sim = AnyModeSimulator::new("abc".to_string());
    sim.execute_counted(CMD_OPEN_BELOW, NonZeroUsize::new(3).unwrap())
        .unwrap();
    assert_eq!(text_and_ranges(&sim).0, "abc\n");
    assert_eq!(sim.mode_name(), "INSERT");
}

#[test]
fn test_count_behavior_ignore_set() {
    use crate::helix::registry::{CountBehavior, normal_registry};
    let reg = normal_registry();
    for cmd in [
        CMD_DELETE_SELECTION,
        CMD_DELETE_SELECTION_NOYANK,
        CMD_CHANGE,
        CMD_CHANGE_SELECTION_NOYANK,
        CMD_INSERT,
        CMD_APPEND,
        CMD_INSERT_LINE_START,
        CMD_APPEND_LINE_END,
        CMD_REMOVE_PRIMARY_SELECTION,
        CMD_SWITCH_CASE,
        CMD_SWITCH_CASE_ALT,
        CMD_SWITCH_TO_UPPERCASE,
        CMD_FLIP_SELECTIONS,
        CMD_SELECT_MODE,
        CMD_REPEAT,
        CMD_TOGGLE_MACRO_RECORDING,
        CMD_JOIN_LINES,
        CMD_JOIN_SELECTIONS_SPACE,
    ] {
        assert_eq!(reg.count_behavior(cmd), CountBehavior::Ignore, "{cmd}");
    }
}

#[test]
fn test_count_is_ignored_by_switch_case() {
    let mut sim = AnyModeSimulator::new("abcdef".to_string());
    run(&mut sim, &["fc", "2~"]);
    assert_eq!(text_and_ranges(&sim).0, "ABCdef");
}

#[test]
fn test_count_is_ignored_by_flip_selections() {
    let mut once = AnyModeSimulator::new("abcdef".to_string());
    let mut counted = AnyModeSimulator::new("abcdef".to_string());
    run(&mut once, &["fc", "Alt-;"]);
    run(&mut counted, &["fc", "2Alt-;"]);
    assert_eq!(text_and_ranges(&once), text_and_ranges(&counted));
}
