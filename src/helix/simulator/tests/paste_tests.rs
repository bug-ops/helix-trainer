//! Linewise, multi-range and selecting paste tests (#449, #455, #491).
//!
//! Expected buffers match `hx` 25.07.1 (`paste_impl`) on the same keys.

use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, Mode};

const SCENARIO_14: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n";

fn run(text: &str, cmds: &[&str]) -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(text.to_string());
    for cmd in cmds {
        sim.execute_command(cmd).unwrap();
    }
    sim
}

fn content(sim: &AnyModeSimulator) -> String {
    sim.state().unwrap().content().to_string()
}

#[test]
fn linewise_paste_after_goes_below_current_line() {
    let sim = run(
        SCENARIO_14,
        &[CMD_SELECT_LINE, CMD_YANK, CMD_MOVE_DOWN, CMD_PASTE_AFTER],
    );
    assert_eq!(
        content(&sim),
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\nfn add(a: i32, b: i32) -> i32 {\n    sum\n}\n"
    );
}

#[test]
fn linewise_paste_before_goes_above_current_line() {
    let sim = run(
        SCENARIO_14,
        &[CMD_SELECT_LINE, CMD_YANK, CMD_MOVE_DOWN, CMD_PASTE_BEFORE],
    );
    assert_eq!(
        content(&sim),
        "fn add(a: i32, b: i32) -> i32 {\nfn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn linewise_paste_after_last_unterminated_line_adds_separator() {
    let sim = run(
        "ab\ncd",
        &[CMD_SELECT_LINE, CMD_YANK, CMD_MOVE_DOWN, CMD_PASTE_AFTER],
    );
    assert_eq!(content(&sim), "ab\ncd\nab\n");
}

#[test]
fn linewise_paste_selects_the_pasted_line() {
    let mut sim = run(
        SCENARIO_14,
        &[CMD_SELECT_LINE, CMD_YANK, CMD_MOVE_DOWN, CMD_PASTE_AFTER],
    );
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(
        content(&sim),
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn linewise_paste_on_last_terminated_line_keeps_state_readable() {
    let sim = run(
        "abc\nXYZ\n",
        &[CMD_SELECT_LINE, CMD_YANK, CMD_MOVE_DOWN, CMD_PASTE_AFTER],
    );
    assert_eq!(content(&sim), "abc\nXYZ\nabc\n");
    assert_eq!(sim.mode(), Mode::Normal);
}

#[test]
fn inline_paste_is_unchanged_for_text_without_line_ending() {
    let sim = run("abc def", &[CMD_YANK, CMD_PASTE_AFTER]);
    assert_eq!(content(&sim), "aabc def");
}

#[test]
fn paste_selects_pasted_text_so_delete_restores_original() {
    for paste in [CMD_PASTE_BEFORE, CMD_PASTE_AFTER] {
        let sim = run(
            SCENARIO_14,
            &[CMD_MOVE_WORD_FORWARD, CMD_YANK, paste, CMD_DELETE_SELECTION],
        );
        assert_eq!(content(&sim), SCENARIO_14, "{paste}");
    }
}

#[test]
fn each_range_pastes_its_own_yanked_value() {
    let sim = run(
        "ab cd ef\n",
        &[CMD_SELECT_ALL, "s [a-z]+", CMD_YANK, CMD_PASTE_AFTER],
    );
    assert_eq!(content(&sim), "abab cdcd efef\n");
}

#[test]
fn multi_range_paste_selects_text_in_every_range() {
    let sim = run(
        "ab cd ef\n",
        &[
            CMD_SELECT_ALL,
            "s [a-z]+",
            CMD_YANK,
            CMD_PASTE_AFTER,
            CMD_DELETE_SELECTION,
        ],
    );
    assert_eq!(content(&sim), "ab cd ef\n");
}

#[test]
fn single_char_matches_are_duplicated_in_place() {
    let sim = run(
        SCENARIO_14,
        &[CMD_SELECT_ALL, "s um", CMD_YANK, CMD_PASTE_AFTER],
    );
    assert_eq!(
        content(&sim),
        "fn add(a: i32, b: i32) -> i32 {\n    let sumum = a + b;\n    sumum\n}\n"
    );
}
