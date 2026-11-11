//! Selection stays inside the document after every command (#569).
//!
//! The rows are the crash sequences of the issue: a transaction that shrinks
//! the document must leave the selection mapped or replaced, never stale.

use super::{ranges_of, selection_in_bounds};
use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;
use proptest::prelude::*;

/// Runs `keys`, asserting the selection is in bounds after each one.
fn run_checked(text: &str, keys: &[&str]) -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(text.to_string());
    for key in keys {
        sim.execute_command(key)
            .unwrap_or_else(|err| panic!("`{key}` failed: {err}"));
        assert!(
            selection_in_bounds(&sim),
            "selection out of bounds after `{key}`: {:?}",
            sim.to_snapshot()
        );
    }
    sim
}

/// Zero-width matches put several point cursors inside one grapheme; `R` must not build
/// overlapping changes from them.
#[test]
fn replace_with_yanked_with_cursors_inside_one_grapheme_keeps_the_document_intact() {
    for text in ["a\u{1f1fa}\u{1f1f8}b", "e\u{301}", "x\u{1f1fa}\u{1f1f8}"] {
        let sim = run_checked(
            text,
            &[CMD_SELECT_ALL, CMD_YANK, "s x*", CMD_REPLACE_WITH_YANKED],
        );
        let content = sim.display().content().clone();
        assert!(
            content.chars().count() >= text.chars().count(),
            "{text:?} -> {content:?}"
        );
        for original in text.chars() {
            assert!(content.contains(original), "{text:?} -> {content:?}");
        }
    }
}

#[test]
fn replace_with_yanked_shrinking_the_document_keeps_selection_valid() {
    let sim = run_checked(
        "abc def\n",
        &[
            CMD_YANK,
            CMD_SELECT_ALL,
            CMD_REPLACE_WITH_YANKED,
            CMD_MOVE_WORD_FORWARD,
        ],
    );
    assert_eq!(sim.display().content(), "a");
}

#[test]
fn replace_with_yanked_after_word_yank_and_line_select() {
    let sim = run_checked(
        "foo bar baz\n",
        &[
            CMD_MOVE_WORD_END,
            CMD_YANK,
            CMD_SELECT_LINE,
            CMD_REPLACE_WITH_YANKED,
            CMD_MOVE_DOWN,
        ],
    );
    assert_eq!(sim.display().content(), "foo");
}

#[test]
fn replace_with_yanked_scenario_row_then_any_motion() {
    let sim = run_checked(
        "AAAA\nBBBB\nCCCC",
        &[
            CMD_YANK,
            CMD_MOVE_DOWN,
            CMD_MOVE_DOWN,
            CMD_SELECT_LINE,
            CMD_REPLACE_WITH_YANKED,
            CMD_MOVE_RIGHT,
        ],
    );
    assert_eq!(sim.display().content(), "AAAA\nBBBB\nA");
}

#[test]
fn replace_with_yanked_selects_the_replaced_text() {
    let sim = run_checked(
        "one two",
        &[
            CMD_MOVE_WORD_FORWARD,
            CMD_YANK,
            CMD_MOVE_WORD_FORWARD,
            CMD_REPLACE_WITH_YANKED,
        ],
    );
    assert_eq!(sim.display().content(), "one one ");
    assert_eq!(ranges_of(&sim), vec![(4, 8)]);
}

#[test]
fn join_selections_space_without_separator_maps_the_selection() {
    let sim = run_checked(
        "fn a() {}\n",
        &[CMD_SELECT_LINE, CMD_JOIN_SELECTIONS_SPACE, CMD_SELECT_LINE],
    );
    assert_eq!(sim.display().content(), "fn a() {}");
}

#[test]
fn join_selections_space_on_second_line_keeps_selection_in_bounds() {
    let sim = run_checked(
        "foo\nbar\n",
        &[
            CMD_MOVE_DOWN,
            CMD_SELECT_LINE,
            CMD_JOIN_SELECTIONS_SPACE,
            CMD_MOVE_WORD_FORWARD,
        ],
    );
    assert_eq!(sim.display().content(), "foo\nbar");
}

#[test]
fn toggle_comments_maps_the_selection() {
    let sim = run_checked(
        "// hello\n",
        &[CMD_GOTO_LINE_END, CMD_TOGGLE_COMMENTS, CMD_MOVE_RIGHT],
    );
    assert_eq!(sim.display().content(), "hello\n");
}

#[test]
fn toggle_comments_over_two_lines_keeps_selection_in_bounds() {
    let sim = run_checked(
        "// a\n// b\n",
        &[CMD_SELECT_ALL, CMD_TOGGLE_COMMENTS, CMD_YANK],
    );
    assert_eq!(sim.display().content(), "a\nb\n");
}

#[test]
fn delete_and_replace_surround_work_on_a_backward_range() {
    let deleted = run_checked(
        "x(ab)y",
        &[
            CMD_MOVE_RIGHT,
            CMD_MOVE_RIGHT,
            "mi(",
            CMD_FLIP_SELECTIONS,
            "md(",
        ],
    );
    assert_eq!(deleted.display().content(), "xaby");

    let replaced = run_checked(
        "x(ab)y",
        &[
            CMD_MOVE_RIGHT,
            CMD_MOVE_RIGHT,
            "mi(",
            CMD_FLIP_SELECTIONS,
            "mr([",
        ],
    );
    assert_eq!(replaced.display().content(), "x[ab]y");
}

#[test]
fn delete_surround_on_the_issue_sequence_keeps_the_selection_valid() {
    let sim = run_checked(
        "x\na",
        &[
            "4O",
            "a",
            CMD_ESCAPE,
            "100G",
            CMD_MOVE_LONG_WORD_BACKWARD,
            "mda",
        ],
    );
    assert!(selection_in_bounds(&sim));
}

fn command_alphabet() -> Vec<String> {
    let mut keys: Vec<String> = crate::helix::registry::normal_registry()
        .all_commands()
        .map(|meta| meta.key().to_string())
        .collect();
    keys.extend(
        [
            "i",
            "a",
            "I",
            "A",
            "o",
            "O",
            "c",
            CMD_ESCAPE,
            CMD_BACKSPACE,
            CMD_INSERT_TAB,
            CMD_INSERT_DELETE_WORD_BACKWARD,
            CMD_INSERT_DELETE_TO_LINE_START,
            CMD_INSERT_DELETE_TO_LINE_END,
            CMD_INSERT_DELETE_CHAR_FORWARD,
            CMD_INSERT_DELETE_WORD_FORWARD,
            CMD_ARROW_LEFT,
            CMD_ARROW_RIGHT,
            CMD_ARROW_UP,
            CMD_ARROW_DOWN,
            "x",
            "y",
            "p",
            "d",
            "u",
            "U",
            "\n",
            "q",
            "\u{e9}",
            "\u{1f600}",
            "f\u{e9}",
            "t\u{1f600}",
            "Fx",
            "Ta",
            "r\u{e9}",
            "r\u{1f600}",
            "ms\u{e9}",
            "md\u{e9}",
            "mr\u{e9}\u{1f600}",
            "mi\u{e9}",
            "ma\u{1f600}",
            "mi(",
            "ma\"",
            "md\"",
            "mr()",
            "s a",
            "s x*",
            "S ,",
            "K a",
            "Alt-K a",
            "/ a",
            "? a",
            "\"ay",
            "\"_d",
            "\"ac",
            ":goto 2",
            "3x",
            "2d",
            "4O",
            "100G",
            "2gg",
        ]
        .map(String::from),
    );
    keys
}

fn arbitrary_text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop::sample::select(vec![
            "a",
            "b",
            " ",
            "\n",
            "(",
            ")",
            "\"",
            "{",
            "}",
            "\u{e9}",
            "\u{1f600}",
            "e\u{301}",
            "\u{1f1fa}\u{1f1f8}",
            "x",
            "// ",
        ]),
        0..40,
    )
    .prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(if cfg!(miri) { 4 } else { 4096 }))]

    #[test]
    fn no_command_sequence_panics_or_leaves_the_selection_out_of_bounds(
        text in arbitrary_text(),
        steps in prop::collection::vec(prop::sample::select(command_alphabet()), 0..40),
    ) {
        let mut sim = AnyModeSimulator::new(text);
        for step in &steps {
            let _ = sim.execute_command(step);
            prop_assert!(
                selection_in_bounds(&sim),
                "after `{}`: {:?}",
                step,
                sim.to_snapshot()
            );
        }
    }
}
