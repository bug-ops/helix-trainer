//! Select mode: search, paragraph, bracket and half-page motions (#534).
//!
//! Expected buffers match `hx` 25.07.1 on the same keys (probe: `d`), except
//! the half-page cases, which depend on the terminal height in `hx` and
//! assert only that the anchor is kept.

use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, ModeKind};

const PARAGRAPHS: &str = "alpha beta\n\nfoo bar\nbaz\n\nqux\n";

fn sim_after(text: &str, keys: &[&str]) -> AnyModeSimulator {
    let mut sim = AnyModeSimulator::new(text.to_string());
    for key in keys {
        sim.execute_command(key).unwrap();
    }
    sim
}

fn deleted(text: &str, keys: &[&str]) -> String {
    let mut sim = sim_after(text, keys);
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    sim.display().content()
}

#[test]
fn search_pushes_a_range_and_keeps_the_previous_one() {
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, "/ baz"]),
        "lpha beta\n\nfoo bar\n\n\nqux\n"
    );
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, "/ bar"]),
        "lpha beta\n\nfoo \nbaz\n\nqux\n"
    );
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, "/ qux"]),
        "lpha beta\n\nfoo bar\nbaz\n\n\n"
    );
    assert_eq!(
        deleted(PARAGRAPHS, &["l", "l", CMD_SELECT_MODE, "/ be"]),
        "alha ta\n\nfoo bar\nbaz\n\nqux\n"
    );
}

#[test]
fn backward_search_pushes_a_range() {
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, "? ba"]),
        "lpha beta\n\nfoo bar\nz\n\nqux\n"
    );
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, "? foo"]),
        "lpha beta\n\n bar\nbaz\n\nqux\n"
    );
}

#[test]
fn n_and_capital_n_push_further_ranges() {
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, "/ ba", CMD_SEARCH_NEXT]),
        "lpha beta\n\nfoo r\nz\n\nqux\n"
    );
    assert_eq!(
        deleted(
            PARAGRAPHS,
            &[CMD_SELECT_MODE, "/ ba", CMD_SEARCH_NEXT, CMD_SEARCH_PREV]
        ),
        "lpha beta\n\nfoo r\nz\n\nqux\n"
    );
}

#[test]
fn search_keeps_select_mode() {
    let sim = sim_after(PARAGRAPHS, &[CMD_SELECT_MODE, "/ baz"]);
    assert_eq!(sim.mode_kind(), ModeKind::Select);
}

#[test]
fn paragraph_motions_extend() {
    assert_eq!(
        deleted(
            PARAGRAPHS,
            &["l", "l", CMD_SELECT_MODE, CMD_GOTO_NEXT_PARAGRAPH]
        ),
        "alfoo bar\nbaz\n\nqux\n"
    );
    assert_eq!(
        deleted(
            PARAGRAPHS,
            &[
                "l",
                "l",
                "l",
                "j",
                "j",
                CMD_SELECT_MODE,
                CMD_GOTO_PREV_PARAGRAPH
            ]
        ),
        "alpha beta\n\nbar\nbaz\n\nqux\n"
    );
    assert_eq!(
        deleted(PARAGRAPHS, &[CMD_SELECT_MODE, CMD_GOTO_PREV_PARAGRAPH]),
        "lpha beta\n\nfoo bar\nbaz\n\nqux\n"
    );
    assert_eq!(
        deleted(
            PARAGRAPHS,
            &[
                "l",
                CMD_SELECT_MODE,
                CMD_GOTO_NEXT_PARAGRAPH,
                CMD_GOTO_NEXT_PARAGRAPH
            ]
        ),
        "aqux\n"
    );
}

#[test]
fn match_brackets_extends() {
    let text = "f(a, [b])\nx\n";
    assert_eq!(
        deleted(text, &["l", CMD_SELECT_MODE, CMD_MATCH_BRACKETS]),
        "f\nx\n"
    );
    assert_eq!(
        deleted(
            text,
            &["l", "l", "l", "l", "l", CMD_SELECT_MODE, CMD_MATCH_BRACKETS]
        ),
        "f(a, )\nx\n"
    );
    assert_eq!(
        deleted(text, &[CMD_SELECT_MODE, CMD_MATCH_BRACKETS]),
        "(a, [b])\nx\n"
    );
}

#[test]
fn half_page_motions_extend_from_the_anchor() {
    let text = "ab\nb\nc\nd\ne\nfg";
    assert_eq!(
        deleted(text, &["l", CMD_SELECT_MODE, CMD_HALF_PAGE_DOWN]),
        "a"
    );
    assert_eq!(
        deleted(
            text,
            &[
                "j",
                "j",
                "j",
                "j",
                "j",
                "l",
                CMD_SELECT_MODE,
                CMD_HALF_PAGE_UP
            ]
        ),
        "a"
    );
}
