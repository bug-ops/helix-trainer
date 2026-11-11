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
fn one_undo_restores_a_counted_open_and_an_open_whose_indent_was_stripped() {
    for keys in [
        &["j", "2o", "x", CMD_ESCAPE][..],
        &["j", "2o", CMD_ESCAPE],
        &["j", "o", CMD_ESCAPE],
        &["j", "O", CMD_ESCAPE],
    ] {
        let mut sim = run_keys_in("rs", I1, keys);
        assert_ne!(sim.display().content(), I1, "{keys:?}");
        sim.execute_command(CMD_UNDO).unwrap();
        assert_eq!(sim.display().content(), I1, "{keys:?}");
    }
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

#[test]
fn escape_keeps_every_range_after_cursors_merged_in_the_session() {
    let sim = run_keys(
        "abcd\nef",
        &[
            CMD_SELECT_ALL,
            "s [ace]",
            "a",
            CMD_INSERT_DELETE_TO_LINE_START,
            CMD_ESCAPE,
        ],
    );
    assert_eq!(sim.display().content(), "d\nf");
    assert_eq!(ranges_of(&sim), [(0, 0), (2, 2)]);
}

#[test]
fn append_on_separated_ranges_keeps_one_cursor_per_range() {
    let sim = run_keys(
        "abcdef",
        &[CMD_SELECT_ALL, "s [abd]", "a", CMD_BACKSPACE, CMD_ESCAPE],
    );
    assert_eq!(sim.display().content(), "acef");
    assert_eq!(ranges_of(&sim).len(), 2);
}

#[test]
fn append_on_a_flipped_selection_inserts_after_its_last_character() {
    let sim = run_keys(
        "abcd",
        &["v", "l", CMD_FLIP_SELECTIONS, "a", "x", CMD_ESCAPE],
    );
    assert_eq!(sim.display().content(), "abxcd");
}

#[test]
fn append_groups_touching_and_separated_ranges_like_hx() {
    let cases = [
        ("abcdef", "ab|cd", "abcdxef", 1),
        ("abcdef", "ab|cd|ef", "abcdefx", 1),
        ("abcd", "ab|d", "abxcdx", 2),
        ("abcd", "[abc]", "abcxd", 1),
    ];
    for (text, pattern, expected, ranges) in cases {
        let sim = run_keys(
            text,
            &[
                CMD_SELECT_ALL,
                &format!("s {pattern}"),
                "a",
                "x",
                CMD_ESCAPE,
            ],
        );
        assert_eq!(sim.display().content(), expected, "{pattern}");
        assert_eq!(ranges_of(&sim).len(), ranges, "{pattern}");
    }
}

#[test]
fn append_on_adjacent_ranges_merges_them_like_hx() {
    let sim = run_keys("abc", &[CMD_SELECT_ALL, "s [ab]", "a", "x", CMD_ESCAPE]);
    assert_eq!(sim.display().content(), "abxc");
    assert_eq!(ranges_of(&sim).len(), 1);
}

const MANY_CURSORS: usize = crate::limits::MAX_REGEX_SELECTION_MATCHES;

#[test]
fn a_session_with_the_maximum_cursor_count_applies_to_every_cursor() {
    let text = "a\n".repeat(MANY_CURSORS);
    let sim = run_keys(&text, &[CMD_SELECT_ALL, "s a", "o", "x", CMD_ESCAPE]);
    assert_eq!(sim.display().content(), "a\nx\n".repeat(MANY_CURSORS));
}

#[test]
fn counted_open_beyond_the_cursor_cap_is_rejected_and_changes_nothing() {
    let text = "a\n".repeat(MANY_CURSORS);
    let mut sim = run_keys(&text, &[CMD_SELECT_ALL, "s a"]);
    let error = sim.execute_command("2o").unwrap_err();
    assert!(matches!(
        error,
        crate::helix::error::SimulatorError::TooManyCursors { max: MANY_CURSORS }
    ));
    assert!(!sim.is_insert_mode());
    assert_eq!(sim.display().content(), text);
}
