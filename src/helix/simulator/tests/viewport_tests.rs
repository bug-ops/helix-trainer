//! The paging keys and `zj`/`zk` scroll one fixed 24-line view like `hx`
//! (#541).
//!
//! Expected values were recorded from `hx` 25.07.1 started with an empty
//! configuration in an 80x26 terminal (24 inner rows) on a 70-line document,
//! each sequence followed by the probe `c @ Escape`. A result lists where the
//! probe's `@` landed as `line:column` and how many lines the text has left,
//! which exposes every range the sequence left behind.

use super::probed;
use crate::grammar::commands::*;

/// Seventy lines `L00 abcdefghij` .. `L69 abcdefghij`, no final line break.
fn document() -> String {
    (0..70)
        .map(|line| format!("L{line:02} abcdefghij"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `line:column` of every `@` and the number of lines of `content`.
fn marks(content: &str) -> String {
    let at: Vec<String> = content
        .split('\n')
        .enumerate()
        .flat_map(|(line, text)| {
            text.char_indices()
                .filter(|&(_, ch)| ch == '@')
                .map(move |(column, _)| format!("{line}:{column}"))
        })
        .collect();
    format!("{} | lines {}", at.join(" "), content.split('\n').count())
}

fn after(keys: &[&str]) -> String {
    marks(&probed(&document(), keys))
}

fn assert_cases(cases: &[(&[&str], &str)]) {
    for (keys, expected) in cases {
        assert_eq!(&after(keys), expected, "{keys:?}");
    }
}

#[test]
fn half_page_keys_move_the_view_and_every_range_by_twelve_lines() {
    assert_cases(&[
        (&[CMD_HALF_PAGE_DOWN], "12:0 | lines 70"),
        (&[CMD_HALF_PAGE_DOWN, CMD_HALF_PAGE_DOWN], "24:0 | lines 70"),
        (&[CMD_HALF_PAGE_DOWN, CMD_HALF_PAGE_UP], "0:0 | lines 70"),
        (
            &[CMD_HALF_PAGE_DOWN, CMD_HALF_PAGE_DOWN, CMD_HALF_PAGE_UP],
            "12:0 | lines 70",
        ),
        (&["30G", CMD_HALF_PAGE_DOWN], "41:0 | lines 70"),
        (&["30G", CMD_HALF_PAGE_UP], "17:0 | lines 70"),
        (&["66G", CMD_HALF_PAGE_DOWN], "69:0 | lines 70"),
        (&["70G", CMD_HALF_PAGE_DOWN], "69:0 | lines 70"),
        (&["5G", CMD_HALF_PAGE_UP], "0:0 | lines 70"),
        (
            &[CMD_COPY_SELECTION_NEXT, CMD_HALF_PAGE_DOWN],
            "12:0 13:0 | lines 70",
        ),
    ]);
}

#[test]
fn page_keys_move_the_cursor_only_when_the_view_passes_it() {
    assert_cases(&[
        (&[CMD_PAGE_DOWN], "29:0 | lines 70"),
        (&[CMD_PAGE_DOWN, CMD_PAGE_DOWN], "53:0 | lines 70"),
        (&["30G", CMD_PAGE_DOWN], "40:0 | lines 70"),
        (&["30G", CMD_PAGE_UP], "18:0 | lines 70"),
        (&["62G", CMD_PAGE_DOWN], "69:14 | lines 70"),
        (&["70G", CMD_PAGE_DOWN], "69:14 | lines 70"),
        (&[CMD_PAGE_UP], "0:0 | lines 70"),
        (&["70G", CMD_PAGE_UP], "45:0 | lines 70"),
        (
            &[CMD_COPY_SELECTION_NEXT, CMD_PAGE_DOWN],
            "0:0 29:0 | lines 70",
        ),
    ]);
}

#[test]
fn select_mode_extends_from_the_anchor() {
    assert_cases(&[
        (&[CMD_SELECT_MODE, CMD_HALF_PAGE_DOWN], "0:0 | lines 58"),
        (&[CMD_SELECT_MODE, CMD_PAGE_DOWN], "0:0 | lines 42"),
        (&["30G", CMD_SELECT_MODE, CMD_PAGE_UP], "18:0 | lines 60"),
        (&["30G", CMD_SELECT_MODE, CMD_PAGE_DOWN], "29:0 | lines 60"),
        (
            &["30G", CMD_SELECT_MODE, CMD_HALF_PAGE_DOWN],
            "29:0 | lines 58",
        ),
        (
            &["30G", CMD_SELECT_MODE, CMD_HALF_PAGE_UP],
            "17:0 | lines 58",
        ),
    ]);
}

#[test]
fn zj_and_zk_scroll_the_view_and_keep_the_cursor_in_the_band() {
    assert_cases(&[
        (&[CMD_SCROLL_DOWN], "6:0 | lines 70"),
        (&[CMD_SCROLL_DOWN, CMD_SCROLL_DOWN], "7:0 | lines 70"),
        (&[CMD_SCROLL_DOWN, CMD_PAGE_DOWN], "30:0 | lines 70"),
        (&["30G", CMD_SCROLL_DOWN], "29:0 | lines 70"),
        (&["30G", CMD_SCROLL_UP], "28:0 | lines 70"),
        (&["30G", CMD_SCROLL_UP, CMD_SCROLL_UP], "27:0 | lines 70"),
        (&["30G", CMD_SCROLL_UP, CMD_PAGE_DOWN], "39:0 | lines 70"),
        (&["30G", CMD_SCROLL_DOWN, CMD_PAGE_UP], "18:0 | lines 70"),
        (&[CMD_SCROLL_UP], "0:0 | lines 70"),
        (
            &["70G", CMD_SCROLL_DOWN, CMD_SCROLL_DOWN, CMD_PAGE_DOWN],
            "69:14 | lines 70",
        ),
        (&[CMD_SELECT_MODE, CMD_SCROLL_DOWN], "0:0 | lines 65"),
    ]);
}

#[test]
fn a_search_outside_the_view_centers_it() {
    assert_cases(&[
        (&["/ L50", CMD_HALF_PAGE_UP], "38:2 | lines 70"),
        (&["/ L50", CMD_PAGE_DOWN], "68:0 | lines 70"),
        (&["/ L50", CMD_PAGE_UP], "33:0 | lines 70"),
        (
            &["/ L50", CMD_SEARCH_NEXT, CMD_HALF_PAGE_DOWN],
            "62:2 | lines 70",
        ),
        (&["/ L20", CMD_PAGE_DOWN], "31:0 | lines 70"),
        (&["/ L40", "/ L10", CMD_PAGE_DOWN], "29:0 | lines 70"),
    ]);
}
