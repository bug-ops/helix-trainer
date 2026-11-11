//! Insert-mode editing key tests (#489): `Ctrl-w`, `Ctrl-u`, `Ctrl-k`,
//! `Ctrl-d`, `Delete`, `Alt-d` and `Alt-Backspace`.
//!
//! Expected buffers were captured from `hx` 25.07.1 on the same text and keys.

use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, InsertCommand};

const SCENARIO_14: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n";

fn run(text: &str, cmds: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new(text.to_string());
    for cmd in cmds {
        sim.execute_command(cmd).unwrap();
    }
    sim.display().content()
}

#[test]
fn ctrl_w_deletes_word_instead_of_inserting_letter() {
    let out = run(SCENARIO_14, &[CMD_APPEND_LINE_END, "Ctrl-w", CMD_ESCAPE]);
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 \n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn repeated_ctrl_w_removes_two_words() {
    let out = run(
        SCENARIO_14,
        &[
            CMD_MOVE_DOWN,
            CMD_APPEND_LINE_END,
            "Ctrl-w",
            "Ctrl-w",
            CMD_ESCAPE,
        ],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + \n    sum\n}\n"
    );
}

#[test]
fn alt_backspace_deletes_word_backward_across_newline() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_INSERT, "Alt-Backspace", CMD_ESCAPE],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32     let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn ctrl_u_deletes_to_line_start() {
    let out = run(SCENARIO_14, &[CMD_APPEND_LINE_END, "Ctrl-u", CMD_ESCAPE]);
    assert_eq!(out, "\n    let sum = a + b;\n    sum\n}\n");
}

#[test]
fn ctrl_u_keeps_indentation() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_APPEND_LINE_END, "Ctrl-u", CMD_ESCAPE],
    );
    assert_eq!(out, "fn add(a: i32, b: i32) -> i32 {\n    \n    sum\n}\n");
}

#[test]
fn ctrl_u_at_line_start_joins_previous_line() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_INSERT, "Ctrl-u", CMD_ESCAPE],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn ctrl_k_at_line_end_deletes_newline() {
    let out = run(SCENARIO_14, &[CMD_APPEND_LINE_END, "Ctrl-k", CMD_ESCAPE]);
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn ctrl_k_mid_line_deletes_to_line_end() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, "gh", CMD_INSERT, "Ctrl-k", CMD_ESCAPE],
    );
    assert_eq!(out, "fn add(a: i32, b: i32) -> i32 {\n\n    sum\n}\n");
}

#[test]
fn ctrl_d_and_delete_remove_char_forward() {
    let expected = "fn add(a: i32, b: i32) -> i32 {\n  let sum = a + b;\n    sum\n}\n";
    for key in ["Ctrl-d", "Delete"] {
        let out = run(
            SCENARIO_14,
            &[CMD_MOVE_DOWN, CMD_INSERT, key, key, CMD_ESCAPE],
        );
        assert_eq!(out, expected, "{key}");
    }
}

#[test]
fn alt_d_deletes_word_forward() {
    let out = run(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_APPEND_LINE_END, "Alt-d", CMD_ESCAPE],
    );
    assert_eq!(
        out,
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n}\n"
    );
}

#[test]
fn deleting_keys_at_document_edges_do_not_panic() {
    for key in [
        "Ctrl-w",
        "Ctrl-u",
        "Ctrl-k",
        "Ctrl-d",
        "Delete",
        "Alt-d",
        "Alt-Backspace",
    ] {
        assert_eq!(run("", &[CMD_INSERT, key, CMD_ESCAPE]), "", "{key}");
    }
    assert_eq!(run("ab", &[CMD_APPEND_LINE_END, "Alt-d", "Ctrl-d"]), "ab");
}

#[test]
fn dot_repeats_insert_session_with_deleting_keys() {
    let out = run(
        "foo bar\nfoo bar\n",
        &[
            CMD_APPEND_LINE_END,
            "x",
            "y",
            "Ctrl-w",
            "z",
            CMD_ESCAPE,
            CMD_MOVE_DOWN,
            CMD_REPEAT,
        ],
    );
    assert_eq!(out, "foo z\nfoo z\n");
}

#[test]
fn insert_command_parse_roundtrips_canonical_strings() {
    for cmd in [
        InsertCommand::DeleteWordBackward,
        InsertCommand::DeleteToLineStart,
        InsertCommand::DeleteToLineEnd,
        InsertCommand::DeleteCharForward,
        InsertCommand::DeleteWordForward,
        InsertCommand::Indent,
    ] {
        assert_eq!(InsertCommand::parse(cmd.as_str()), Some(cmd));
    }
    assert_eq!(InsertCommand::parse("w"), None);
}

#[test]
fn tab_inserts_one_indent_unit_not_a_tab_character() {
    assert_eq!(
        run("x\ny\n", &[CMD_INSERT, CMD_INSERT_TAB, "z", CMD_ESCAPE]),
        "    zx\ny\n"
    );
    assert_eq!(
        run(
            "x\ny\n",
            &[CMD_INSERT, CMD_INSERT_TAB, CMD_INSERT_TAB, "z", CMD_ESCAPE]
        ),
        "        zx\ny\n"
    );
}

#[test]
fn tab_at_the_start_of_an_indented_line_keeps_the_existing_indent() {
    assert_eq!(
        run(
            SCENARIO_14,
            &[CMD_MOVE_DOWN, CMD_INSERT, CMD_INSERT_TAB, "z", CMD_ESCAPE]
        ),
        "fn add(a: i32, b: i32) -> i32 {\n    z    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn tab_unit_follows_the_language() {
    for (language, text, expected) in [("md", "- a\n", "  z- a\n"), ("py", "x\n", "    zx\n")] {
        let mut sim = AnyModeSimulator::new(text.to_string()).with_language(language);
        for cmd in [CMD_INSERT, CMD_INSERT_TAB, "z", CMD_ESCAPE] {
            sim.execute_command(cmd).unwrap();
        }
        assert_eq!(sim.display().content(), expected, "{language}");
    }
}

#[test]
fn dot_replays_tab_in_its_original_position() {
    assert_eq!(
        run(
            "x\ny\n",
            &[
                CMD_INSERT,
                CMD_INSERT_TAB,
                "z",
                CMD_ESCAPE,
                CMD_MOVE_DOWN,
                CMD_GOTO_LINE_START,
                CMD_REPEAT,
            ]
        ),
        "    zx\n    zy\n"
    );
}
