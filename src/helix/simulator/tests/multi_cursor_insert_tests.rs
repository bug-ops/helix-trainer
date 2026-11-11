//! Multi-cursor Insert mode, counted `o`/`O` and the `Escape` indent strip (#596).
//!
//! Every buffer is a golden recorded from `hx` 25.07.1 started with an empty
//! configuration (spec 3.8.6, tables C and the two-cursor entry goldens).

use super::{ranges_of, run_keys, run_keys_in};
use crate::grammar::commands::*;

const I1: &str = "fn f() {\n    a\n    b\n}";
const I1_NO_EOL: &str = "fn f() {\n    a";

fn rs(text: &str, keys: &[&str]) -> String {
    run_keys_in("rs", text, keys).display().content()
}

fn expect_i1(keys: &[&str], expected: &str) {
    assert_eq!(rs(I1, keys), expected, "{keys:?}");
}

#[test]
fn counted_open_keeps_the_auto_indent_of_every_opened_line() {
    expect_i1(
        &["j", "2o", CMD_ESCAPE],
        "fn f() {\n    a\n    \n    \n    b\n}",
    );
    expect_i1(
        &["j", "2O", CMD_ESCAPE],
        "fn f() {\n    \n    \n    a\n    b\n}",
    );
    expect_i1(
        &["j", "2o", "x", CMD_BACKSPACE, CMD_ESCAPE],
        "fn f() {\n    a\n    \n    \n    b\n}",
    );
    assert_eq!(
        run_keys("a\nb", &["2o", CMD_ESCAPE]).display().content(),
        "a\n\n\nb"
    );
}

#[test]
fn counted_open_types_into_every_opened_line_with_the_first_primary() {
    expect_i1(
        &["j", "2o", "x", CMD_ESCAPE],
        "fn f() {\n    a\n    x\n    x\n    b\n}",
    );
    let sim = run_keys_in("rs", I1, &["j", "2o", "x"]);
    assert_eq!(ranges_of(&sim).len(), 2);
    assert_eq!(sim.to_snapshot().primary_idx, 0);
}

#[test]
fn open_at_two_cursors_opens_below_and_above_each() {
    expect_i1(
        &["j", "C", "o", CMD_ESCAPE],
        "fn f() {\n    a\n    \n    b\n    \n}",
    );
    expect_i1(
        &["j", "C", "o", "x", CMD_ESCAPE],
        "fn f() {\n    a\n    x\n    b\n    x\n}",
    );
    expect_i1(
        &["j", "C", "O", CMD_ESCAPE],
        "fn f() {\n    \n    a\n    \n    b\n}",
    );
}

#[test]
fn escape_strips_the_indent_of_a_single_unused_new_line() {
    let stripped = "fn f() {\n    a\n\n    b\n}";
    expect_i1(&["j", "o", CMD_ESCAPE], stripped);
    expect_i1(&["j", "o", "x", CMD_BACKSPACE, CMD_ESCAPE], stripped);
    expect_i1(&["j", "A", "\n", CMD_ESCAPE], stripped);
    expect_i1(&["j", "O", CMD_ESCAPE], "fn f() {\n\n    a\n    b\n}");
}

#[test]
fn escape_keeps_the_indent_once_text_was_typed() {
    let typed = "fn f() {\n    a\n    x\n    b\n}";
    expect_i1(&["j", "o", "x", CMD_ESCAPE], typed);
    expect_i1(&["j", "A", "\n", "x", CMD_ESCAPE], typed);
    expect_i1(
        &["j", "o", "\n", CMD_ESCAPE],
        "fn f() {\n    a\n\n    \n    b\n}",
    );
}

#[test]
fn escape_does_not_strip_when_the_new_line_is_last_without_a_line_break() {
    let kept = "fn f() {\n    a\n    ";
    assert_eq!(rs(I1_NO_EOL, &["j", "o", CMD_ESCAPE]), kept);
    assert_eq!(
        rs(I1_NO_EOL, &["j", "o", "x", CMD_BACKSPACE, CMD_ESCAPE]),
        kept
    );
    assert_eq!(
        rs(I1_NO_EOL, &["j", "2o", CMD_ESCAPE]),
        "fn f() {\n    a\n    \n    "
    );
    assert_eq!(rs(I1_NO_EOL, &["j", "O", CMD_ESCAPE]), "fn f() {\n\n    a");
}

#[test]
fn one_undo_restores_the_text_and_every_range_of_a_multi_cursor_session() {
    let mut sim = run_keys_in("rs", I1, &["j", "C", "o", "x", CMD_ESCAPE]);
    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.display().content(), I1);
    assert_eq!(ranges_of(&sim).len(), 2);
}

#[test]
fn dot_replays_a_multi_cursor_session_and_ignores_the_count() {
    expect_i1(
        &["j", "C", "o", "x", CMD_ESCAPE, "gg", CMD_REPEAT],
        "fn f() {\n    x\n    a\n    x\n    b\n    x\n}",
    );
    assert_eq!(
        run_keys("a\nb\nc", &["2o", "x", CMD_ESCAPE, "j", CMD_REPEAT])
            .display()
            .content(),
        "a\nx\nx\nx\nb\nx\nc"
    );
}

const TWO_CURSORS: &str = "ab cd\nef gh\nij kl";

fn at_two_cursors(keys: &[&str]) -> String {
    let mut all = vec!["l", "C"];
    all.extend_from_slice(keys);
    all.push(CMD_ESCAPE);
    run_keys(TWO_CURSORS, &all).display().content()
}

#[test]
fn entry_commands_apply_to_every_cursor() {
    assert_eq!(at_two_cursors(&["i", "x"]), "axb cd\nexf gh\nij kl");
    assert_eq!(at_two_cursors(&["a", "x"]), "abx cd\nefx gh\nij kl");
    assert_eq!(at_two_cursors(&["A", "x"]), "ab cdx\nef ghx\nij kl");
    assert_eq!(at_two_cursors(&["I", "x"]), "xab cd\nxef gh\nij kl");
}

#[test]
fn editing_keys_apply_to_every_cursor() {
    assert_eq!(at_two_cursors(&["i", "\n"]), "a\nb cd\ne\nf gh\nij kl");
    assert_eq!(at_two_cursors(&["i", CMD_BACKSPACE]), "b cd\nf gh\nij kl");
    assert_eq!(at_two_cursors(&["a", CMD_BACKSPACE]), "a cd\ne gh\nij kl");
    assert_eq!(
        at_two_cursors(&["i", CMD_INSERT_DELETE_WORD_BACKWARD]),
        "b cd\nf gh\nij kl"
    );
    assert_eq!(
        at_two_cursors(&["A", CMD_INSERT_DELETE_WORD_BACKWARD]),
        "ab \nef \nij kl"
    );
}
