//! Commands compared with `hx` 25.07.1 on scenario 14 (#576, #578 and friends).
//!
//! Every expected value was recorded from a local `hx` 25.07.1 started with an
//! empty configuration. Where a sequence ends in the probe `c @ Escape` (see
//! [`probed`]), `@` marks the selection the last command left behind; a probe
//! of `d` is written out as the resulting text.

use super::{SCENARIO_14, probed, probed_in, run_keys, run_keys_in};
use crate::grammar::commands::*;
use crate::helix::Language;

fn line(content: &str, index: usize) -> &str {
    content.lines().nth(index).unwrap_or_default()
}

fn after_delete(keys: &[&str]) -> String {
    let mut keys = keys.to_vec();
    keys.push(CMD_DELETE_SELECTION);
    run_keys(SCENARIO_14, &keys).display().content()
}

#[test]
fn replace_with_yanked_on_a_point_cursor_replaces_the_grapheme_under_it() {
    let content = probed(
        SCENARIO_14,
        &[CMD_YANK, CMD_MOVE_DOWN, CMD_REPLACE_WITH_YANKED],
    );
    assert_eq!(line(&content, 1), "@   let sum = a + b;");
}

#[test]
fn replace_with_yanked_after_word_motions_matches_hx() {
    let content = probed(
        SCENARIO_14,
        &[
            CMD_MOVE_WORD_FORWARD,
            CMD_YANK,
            CMD_MOVE_DOWN,
            CMD_MOVE_WORD_FORWARD,
            CMD_REPLACE_WITH_YANKED,
        ],
    );
    assert_eq!(line(&content, 1), "  @let sum = a + b;");

    let content = probed(
        SCENARIO_14,
        &[
            CMD_MOVE_WORD_FORWARD,
            CMD_YANK,
            CMD_MOVE_DOWN,
            CMD_REPLACE_WITH_YANKED,
        ],
    );
    assert_eq!(line(&content, 1), "  @ let sum = a + b;");
}

#[test]
fn n_after_a_backward_search_moves_forward() {
    let content = after_delete(&["? a", CMD_SEARCH_NEXT]);
    assert_eq!(line(&content, 0), "fn dd(a: i32, b: i32) -> i32 {");
}

#[test]
fn big_n_after_a_backward_search_moves_backward() {
    let content = after_delete(&["? a", CMD_SEARCH_PREV]);
    assert_eq!(line(&content, 0), "fn add(: i32, b: i32) -> i32 {");
}

#[test]
fn n_after_a_backward_search_from_the_third_column_wraps_forward() {
    let content = after_delete(&[CMD_MOVE_RIGHT, CMD_MOVE_RIGHT, "? a", CMD_SEARCH_NEXT]);
    assert_eq!(line(&content, 0), "fn dd(a: i32, b: i32) -> i32 {");
}

#[test]
fn n_after_a_forward_search_is_unchanged() {
    let content = after_delete(&["/ a", CMD_SEARCH_NEXT]);
    assert_eq!(line(&content, 0), "fn add(: i32, b: i32) -> i32 {");
}

#[test]
fn star_keeps_the_selection_and_c_changes_it() {
    let content = probed_in(
        SCENARIO_14,
        Language::Rs,
        &[CMD_MOVE_WORD_FORWARD, CMD_SEARCH_WORD],
    );
    assert_eq!(line(&content, 0), "@add(a: i32, b: i32) -> i32 {");
}

#[test]
fn star_then_n_wraps_to_the_only_match() {
    let content = after_delete(&[CMD_MOVE_WORD_FORWARD, CMD_SEARCH_WORD, CMD_SEARCH_NEXT]);
    assert_eq!(line(&content, 0), "add(a: i32, b: i32) -> i32 {");
}

#[test]
fn star_after_inside_word_selection_uses_word_boundaries() {
    let content = after_delete(&["miw", CMD_SEARCH_WORD, CMD_SEARCH_NEXT]);
    assert_eq!(line(&content, 0), " add(a: i32, b: i32) -> i32 {");
}

#[test]
fn merge_consecutive_selections_keeps_separate_ranges() {
    let content = after_delete(&[CMD_SELECT_ALL, "s a", CMD_MERGE_CONSECUTIVE]);
    assert_eq!(line(&content, 0), "fn dd(: i32, b: i32) -> i32 {");
    assert_eq!(line(&content, 1), "    let sum =  + b;");
}

#[test]
fn merge_selections_spans_first_to_last_range() {
    let content = after_delete(&[CMD_SELECT_ALL, "s a", CMD_MERGE_SELECTIONS]);
    assert_eq!(line(&content, 0), "fn  + b;");
}

#[test]
fn merge_consecutive_selections_joins_touching_ranges() {
    let mut sim = run_keys("aaa b", &[CMD_SELECT_ALL, "s a", CMD_MERGE_CONSECUTIVE]);
    assert_eq!(super::ranges_of(&sim), vec![(0, 3)]);
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.display().content(), " b");
}

#[test]
fn match_brackets_from_a_selection_ending_on_an_opener_jumps_to_its_closer() {
    let content = after_delete(&["f(", CMD_MATCH_BRACKETS]);
    assert_eq!(line(&content, 0), "fn add(a: i32, b: i32 -> i32 {");
}

#[test]
fn match_brackets_from_a_selection_ending_on_a_closer_jumps_to_its_opener() {
    let content = after_delete(&["f)", CMD_MATCH_BRACKETS]);
    assert_eq!(line(&content, 0), "fn adda: i32, b: i32) -> i32 {");
}

#[test]
fn match_brackets_from_a_selection_ending_on_a_brace_jumps_to_the_closing_brace() {
    let content = after_delete(&["f{", CMD_MATCH_BRACKETS]);
    assert_eq!(
        content,
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n\n"
    );
}

#[test]
fn match_brackets_on_a_single_character_cursor_is_unchanged() {
    let content = after_delete(&[CMD_MOVE_WORD_FORWARD, CMD_MATCH_BRACKETS]);
    assert_eq!(line(&content, 0), "add(a: i32, b: i32) -> i32 {");
}

#[test]
fn insert_at_line_start_goes_after_the_indentation() {
    let content = run_keys(
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_INSERT_LINE_START, "a", "b", CMD_ESCAPE],
    )
    .display()
    .content();
    assert_eq!(line(&content, 1), "    ablet sum = a + b;");
}

#[test]
fn insert_at_line_start_then_escape_leaves_the_cursor_after_the_typed_text() {
    let content = after_delete(&[CMD_MOVE_DOWN, CMD_INSERT_LINE_START, "a", "b", CMD_ESCAPE]);
    assert_eq!(line(&content, 1), "    abet sum = a + b;");
}

#[test]
fn insert_at_line_start_uses_the_line_of_the_block_cursor_after_a_line_selection() {
    let content = run_keys(
        SCENARIO_14,
        &[CMD_SELECT_LINE, CMD_INSERT_LINE_START, "a", "b", CMD_ESCAPE],
    )
    .display()
    .content();
    assert_eq!(line(&content, 0), "abfn add(a: i32, b: i32) -> i32 {");
}

#[test]
fn insert_at_line_start_on_an_empty_line_auto_indents() {
    let rust = |text: &str| {
        run_keys_in(
            Language::Rs,
            text,
            &[CMD_MOVE_DOWN, CMD_INSERT_LINE_START, "a", CMD_ESCAPE],
        )
        .display()
        .content()
    };
    assert_eq!(rust("fn f() {\n\n}\n"), "fn f() {\n    a\n}\n");

    let plain = run_keys(
        "a\n\nb\n",
        &[CMD_MOVE_DOWN, CMD_INSERT_LINE_START, "a", CMD_ESCAPE],
    )
    .display()
    .content();
    assert_eq!(plain, "a\na\nb\n");
}

#[test]
fn insert_at_line_start_on_an_empty_line_copies_the_indent_above() {
    let content = run_keys_in(
        Language::Rs,
        "fn add() {\n    x\n\n}\n",
        &[
            CMD_MOVE_DOWN,
            CMD_MOVE_DOWN,
            CMD_INSERT_LINE_START,
            "a",
            CMD_ESCAPE,
        ],
    )
    .display()
    .content();
    assert_eq!(content, "fn add() {\n    x\n    a\n}\n");
}

#[test]
fn insert_at_line_start_with_indent_is_one_undo_step() {
    let content = run_keys_in(
        Language::Rs,
        "fn f() {\n\n}\n",
        &[
            CMD_MOVE_DOWN,
            CMD_INSERT_LINE_START,
            "a",
            CMD_ESCAPE,
            CMD_UNDO,
        ],
    )
    .display()
    .content();
    assert_eq!(content, "fn f() {\n\n}\n");
}

fn change_line_probe(language: Language, text: &str, keys: &[&str]) -> String {
    let mut keys = keys.to_vec();
    keys.extend([CMD_SELECT_LINE, CMD_CHANGE, "@", CMD_ESCAPE]);
    run_keys_in(language, text, &keys).display().content()
}

#[test]
fn changing_a_whole_line_keeps_the_indent_of_the_language_rule() {
    let third = change_line_probe(
        Language::Rs,
        SCENARIO_14,
        &[CMD_MOVE_RIGHT, CMD_MOVE_DOWN, CMD_MOVE_RIGHT, CMD_MOVE_DOWN],
    );
    assert_eq!(line(&third, 2), "    @");
    assert_eq!(third.lines().count(), 4);

    let first = change_line_probe(Language::Rs, SCENARIO_14, &[CMD_MOVE_RIGHT]);
    assert_eq!(line(&first, 0), "    @");
    assert_eq!(line(&first, 1), "    let sum = a + b;");

    let word = change_line_probe(Language::Rs, SCENARIO_14, &[CMD_MOVE_WORD_FORWARD]);
    assert_eq!(line(&word, 0), "    @");
}

#[test]
fn changing_the_last_line_indents_like_the_line_above() {
    let content = change_line_probe(
        Language::Rs,
        SCENARIO_14,
        &[CMD_MOVE_DOWN, CMD_MOVE_DOWN, CMD_MOVE_DOWN],
    );
    assert_eq!(line(&content, 3), "    @");
}

#[test]
fn changing_a_line_below_a_comment_does_not_continue_the_comment() {
    let text = "// note\nfn f() {\n    x\n}\n";
    assert_eq!(line(&change_line_probe(Language::Rs, text, &[]), 0), "@");
    assert_eq!(
        line(&change_line_probe(Language::Rs, text, &[CMD_MOVE_DOWN]), 1),
        "@"
    );
    assert_eq!(
        line(
            &change_line_probe(Language::Rs, text, &[CMD_MOVE_DOWN, CMD_MOVE_DOWN]),
            2
        ),
        "    @"
    );
}

#[test]
fn changing_a_python_block_line_keeps_the_block_indent() {
    let text = "def f():\n    x\n    y\nz = 1\n";
    assert_eq!(
        line(&change_line_probe(Language::Py, text, &[CMD_MOVE_DOWN]), 1),
        "    @"
    );
    assert_eq!(
        line(
            &change_line_probe(Language::Py, text, &[CMD_MOVE_DOWN, CMD_MOVE_DOWN]),
            2
        ),
        "    @"
    );
}

#[test]
fn changing_a_whole_line_is_one_undo_step() {
    let content = run_keys_in(
        Language::Rs,
        SCENARIO_14,
        &[
            CMD_MOVE_DOWN,
            CMD_SELECT_LINE,
            CMD_CHANGE,
            "@",
            CMD_ESCAPE,
            CMD_UNDO,
        ],
    )
    .display()
    .content();
    assert_eq!(content, SCENARIO_14);
}

fn typed_a_after(entry: &[&str], language: Language) -> String {
    let keys: Vec<&str> = entry.iter().copied().chain(["a", CMD_ESCAPE]).collect();
    run_keys_in(language, SCENARIO_14, &keys)
        .display()
        .content()
}

/// #593: `i` inserts at the selection start whatever the range direction.
#[test]
fn insert_on_a_forward_selection_inserts_at_its_start() {
    let content = typed_a_after(&[CMD_MOVE_WORD_FORWARD, CMD_INSERT], Language::Rs);
    assert_eq!(line(&content, 0), "afn add(a: i32, b: i32) -> i32 {");
    assert_eq!(content.lines().count(), SCENARIO_14.lines().count());
}

#[test]
fn insert_on_a_flipped_selection_matches_the_forward_one() {
    let forward = typed_a_after(&[CMD_MOVE_WORD_FORWARD, CMD_INSERT], Language::Rs);
    let flipped = typed_a_after(
        &[CMD_MOVE_WORD_FORWARD, CMD_FLIP_SELECTIONS, CMD_INSERT],
        Language::Rs,
    );
    assert_eq!(flipped, forward);
}

#[test]
fn append_on_a_selection_still_inserts_at_its_end() {
    let content = typed_a_after(&[CMD_MOVE_WORD_FORWARD, CMD_APPEND], Language::Rs);
    assert_eq!(line(&content, 0), "fn aadd(a: i32, b: i32) -> i32 {");
}
