//! Indentation on `o`/`O`/Enter (#490) and multi-range `r` (#497).
//!
//! Expected buffers match `hx` 25.07.1 on the same text and keys.

use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

const SCENARIO_14: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n";

fn run(text: &str, cmds: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new(text.to_string());
    for cmd in cmds {
        sim.execute_command(cmd).unwrap();
    }
    sim.state().unwrap().content().to_string()
}

#[test]
fn open_below_and_enter_keep_indentation() {
    let out = run(
        SCENARIO_14,
        &[
            CMD_MOVE_DOWN,
            CMD_OPEN_BELOW,
            "x",
            "y",
            "z",
            "\n",
            "a",
            "b",
            "c",
            CMD_ESCAPE,
        ],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    xyz\n    abc\n    sum\n}\n"
    );
}

#[test]
fn open_above_keeps_indentation() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_OPEN_ABOVE, "x", CMD_ESCAPE],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {\n    x\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn open_below_on_unindented_line_adds_no_indent() {
    let out = run("a\nb\n", &[CMD_OPEN_BELOW, "x", CMD_ESCAPE]);
    assert_eq!(out, "a\nx\nb\n");
}

#[test]
fn open_below_on_last_unterminated_line() {
    let out = run("  a", &[CMD_OPEN_BELOW, "x", CMD_ESCAPE]);
    assert_eq!(out, "  a\n  x");
}

#[test]
fn enter_after_text_copies_indent() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_APPEND_LINE_END, "\n", "x", CMD_ESCAPE],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    x\n    sum\n}\n"
    );
}

#[test]
fn enter_before_first_char_moves_line_down_intact() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_INSERT, "\n", "x", CMD_ESCAPE],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {\n\nx    let sum = a + b;\n    sum\n}\n"
    );
}

const MAIN_FN: &str = "fn main() {\n    println!(\"Hello\");\n}";

#[test]
fn escape_right_after_open_strips_the_auto_indent() {
    assert_eq!(
        run(MAIN_FN, &[CMD_MOVE_DOWN, CMD_OPEN_ABOVE, CMD_ESCAPE]),
        "fn main() {\n\n    println!(\"Hello\");\n}"
    );
    assert_eq!(
        run(MAIN_FN, &[CMD_MOVE_DOWN, CMD_OPEN_BELOW, CMD_ESCAPE]),
        "fn main() {\n    println!(\"Hello\");\n\n}"
    );
}

#[test]
fn escape_right_after_enter_strips_the_auto_indent() {
    let out = run(
        MAIN_FN,
        &[CMD_MOVE_DOWN, CMD_APPEND_LINE_END, "\n", CMD_ESCAPE],
    );
    assert_eq!(out, "fn main() {\n    println!(\"Hello\");\n\n}");
}

#[test]
fn escape_keeps_indent_once_text_was_typed() {
    let out = run(MAIN_FN, &[CMD_MOVE_DOWN, CMD_OPEN_BELOW, "x", CMD_ESCAPE]);
    assert_eq!(out, "fn main() {\n    println!(\"Hello\");\n    x\n}");
}

#[test]
fn stripped_auto_indent_is_part_of_the_open_undo_step() {
    use crate::helix::commands::CMD_UNDO;
    let out = run(
        MAIN_FN,
        &[CMD_MOVE_DOWN, CMD_OPEN_BELOW, CMD_ESCAPE, CMD_UNDO],
    );
    assert_eq!(out, MAIN_FN);
}

#[test]
fn replace_char_replaces_every_selected_char() {
    let out = run(SCENARIO_14, &[CMD_MOVE_WORD_FORWARD, "rz"]);
    assert_eq!(
        out,
        "zzzadd(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn replace_char_keeps_selection_so_delete_removes_replaced_text() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_WORD_FORWARD, "rz", CMD_DELETE_SELECTION],
    );
    assert_eq!(
        out,
        "add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn replace_char_touches_every_range() {
    let out = run(SCENARIO_14, &[CMD_SELECT_ALL, "s a", "rz"]);
    assert_eq!(
        out,
        "fn zdd(z: i32, b: i32) -> i32 {\n    let sum = z + b;\n    sum\n}\n"
    );
}

#[test]
fn replace_char_on_plain_cursor_replaces_one_char() {
    assert_eq!(run("abc", &["rz"]), "zbc");
}

#[test]
fn replace_char_at_document_end_is_a_noop() {
    assert_eq!(run("", &["rz"]), "");
}
