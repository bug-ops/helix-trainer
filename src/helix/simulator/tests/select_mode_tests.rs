//! Select (extend) mode tests (#524).
//!
//! Expected buffers match `hx` 25.07.1 on the same keys (probe: `d` or `c`).

use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, ModeKind};

const TEXT: &str = "hello world foo\nline two\nthree\n";

fn run(keys: &[&str]) -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(TEXT.to_string());
    for key in keys {
        sim.execute_command(key).unwrap();
    }
    sim
}

fn content(sim: &AnyModeSimulator) -> String {
    sim.display().content()
}

fn deleted(keys: &[&str]) -> String {
    let mut sim = run(keys);
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    content(&sim)
}

#[test]
fn v_toggles_select_mode() {
    let mut sim = run(&[CMD_SELECT_MODE]);
    assert_eq!(sim.mode_kind(), ModeKind::Select);
    sim.execute_command(CMD_SELECT_MODE).unwrap();
    assert_eq!(sim.mode_kind(), ModeKind::Normal);
}

#[test]
fn escape_leaves_select_mode_and_keeps_selection() {
    let sim = run(&[CMD_SELECT_MODE, CMD_MOVE_RIGHT, CMD_MOVE_RIGHT, CMD_ESCAPE]);
    assert_eq!(sim.mode_kind(), ModeKind::Normal);
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_ESCAPE]),
        "lo world foo\nline two\nthree\n"
    );
}

#[test]
fn motions_move_normally_after_leaving_select_mode() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_ESCAPE, "l"]),
        "helo world foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_SELECT_MODE, "l"]),
        "helo world foo\nline two\nthree\n"
    );
}

#[test]
fn char_motions_extend_over_moved_over_characters() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l"]),
        "lo world foo\nline two\nthree\n"
    );
    assert_eq!(deleted(&[CMD_SELECT_MODE, "j"]), "ine two\nthree\n");
    assert_eq!(deleted(&[CMD_SELECT_MODE, "j", "j"]), "hree\n");
    assert_eq!(
        deleted(&["l", "l", CMD_SELECT_MODE, "h", "h", "h"]),
        "lo world foo\nline two\nthree\n"
    );
}

#[test]
fn counted_motion_extends() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "3l"]),
        "o world foo\nline two\nthree\n"
    );
}

#[test]
fn word_motions_extend() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "w", "w"]),
        "foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "e"]),
        " world foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "w", "b"]),
        "ello world foo\nline two\nthree\n"
    );
}

#[test]
fn find_and_till_extend() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "fo"]),
        " world foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "to"]),
        "o world foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "2fo"]),
        "rld foo\nline two\nthree\n"
    );
}

#[test]
fn goto_motions_extend() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, CMD_GOTO_LINE_END]),
        "\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&["l", "l", "l", CMD_SELECT_MODE, CMD_GOTO_LINE_START]),
        "o world foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&["j", "j", CMD_SELECT_MODE, CMD_GOTO_FILE_START]),
        "hree\n"
    );
}

#[test]
fn collapse_selection_keeps_select_mode() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_COLLAPSE_SELECTION, "l"]),
        "heo world foo\nline two\nthree\n"
    );
}

#[test]
fn undo_keeps_select_mode() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_UNDO, "l"]),
        "o world foo\nline two\nthree\n"
    );
}

#[test]
fn delete_leaves_select_mode() {
    let mut sim = run(&[CMD_SELECT_MODE, "l", "l", CMD_DELETE_SELECTION]);
    assert_eq!(sim.mode_kind(), ModeKind::Normal);
    sim.execute_command("l").unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(content(&sim), "l world foo\nline two\nthree\n");
}

#[test]
fn yank_and_switch_case_leave_select_mode() {
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_YANK, "l"]),
        "helo world foo\nline two\nthree\n"
    );
    assert_eq!(
        deleted(&[CMD_SELECT_MODE, "l", "l", CMD_SWITCH_CASE, "l"]),
        "HELo world foo\nline two\nthree\n"
    );
}

#[test]
fn replace_applies_to_every_selected_character() {
    let sim = run(&[CMD_SELECT_MODE, "l", "l", "rx"]);
    assert_eq!(sim.mode_kind(), ModeKind::Normal);
    assert_eq!(content(&sim), "xxxlo world foo\nline two\nthree\n");
}

#[test]
fn change_enters_insert_and_escape_returns_to_normal() {
    let mut sim = run(&[CMD_SELECT_MODE, "l", "l", CMD_CHANGE, "Z"]);
    assert_eq!(sim.mode_kind(), ModeKind::Insert);
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.mode_kind(), ModeKind::Normal);
    assert_eq!(content(&sim), "Zlo world foo\nline two\nthree\n");
}

#[test]
fn select_mode_is_reported_by_name() {
    let sim = run(&[CMD_SELECT_MODE]);
    assert_eq!(sim.mode_name(), "SELECT");
}
