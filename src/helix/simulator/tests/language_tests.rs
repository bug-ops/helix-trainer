//! Language-table behavior: indent auto-detection (#535), `&` display widths
//! (#535) and comment tokens for `J` (#521).
//!
//! Expected buffers are recorded from `hx` 25.07.1 started with an empty
//! configuration on the same text and keys.

use super::run_keys;
use crate::grammar::commands::*;
use crate::helix::Language;
use crate::helix::simulator::AnyModeSimulator;

fn run_in(language: Language, text: &str, keys: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new(text.to_string()).with_language(language);
    for key in keys {
        sim.execute_command(key).unwrap();
    }
    sim.display().content()
}

const EIGHT_SPACES: &str = "fn a() {\n        let x = 1;\n        let y = 2;\n}\n";

#[test]
fn indent_unit_is_detected_from_the_text() {
    assert_eq!(
        run_in(Language::Rs, EIGHT_SPACES, &[CMD_MOVE_DOWN, CMD_INDENT]),
        "fn a() {\n                let x = 1;\n        let y = 2;\n}\n"
    );
}

#[test]
fn tab_inserts_the_detected_unit() {
    assert_eq!(
        run_in(
            Language::Rs,
            EIGHT_SPACES,
            &[CMD_MOVE_DOWN, CMD_INSERT, CMD_INSERT_TAB, CMD_ESCAPE]
        ),
        "fn a() {\n                let x = 1;\n        let y = 2;\n}\n"
    );
}

#[test]
fn two_space_text_overrides_the_four_space_language_default() {
    assert_eq!(
        run_in(
            Language::Rs,
            "fn a() {\n  let x = 1;\n  let y = 2;\n}\n",
            &[CMD_MOVE_DOWN, CMD_INSERT, CMD_INSERT_TAB, CMD_ESCAPE]
        ),
        "fn a() {\n    let x = 1;\n  let y = 2;\n}\n"
    );
}

#[test]
fn tab_indented_text_indents_with_a_tab() {
    assert_eq!(
        run_in(Language::Rs, "a\n\tb\n\tc\n", &[CMD_MOVE_DOWN, CMD_INDENT]),
        "a\n\t\tb\n\tc\n"
    );
}

#[test]
fn text_without_indentation_falls_back_to_the_language_table() {
    assert_eq!(
        run_in(
            Language::Rs,
            "x\n",
            &[CMD_INSERT, CMD_INSERT_TAB, CMD_ESCAPE]
        ),
        "    x\n"
    );
    assert_eq!(
        run_in(
            Language::Md,
            "x\n",
            &[CMD_INSERT, CMD_INSERT_TAB, CMD_ESCAPE]
        ),
        "  x\n"
    );
}

#[test]
fn align_pads_by_display_width() {
    assert_eq!(
        run_keys(
            "日本 x\nab x\ne\u{301} x\n",
            &[CMD_SELECT_ALL, "s x", CMD_ALIGN_SELECTIONS]
        )
        .display()
        .content(),
        "日本 x\nab   x\ne\u{301}    x\n"
    );
}

#[test]
fn align_counts_a_tab_at_the_language_tab_width() {
    let keys = [CMD_SELECT_ALL, "s x", CMD_ALIGN_SELECTIONS];
    let text = "\tx\nabcd x\n";
    assert_eq!(run_in(Language::Rs, text, &keys), "\t x\nabcd x\n");
    assert_eq!(run_in(Language::Md, text, &keys), "\t   x\nabcd x\n");
}

#[test]
fn dedent_counts_a_tab_at_the_language_tab_width() {
    let text = "    a\n    b\n\t\tx\n";
    let keys = [CMD_MOVE_DOWN, CMD_MOVE_DOWN, CMD_DEDENT];
    assert_eq!(run_in(Language::Rs, text, &keys), "    a\n    b\n\tx\n");
    assert_eq!(run_in(Language::Md, text, &keys), "    a\n    b\nx\n");
}

#[test]
fn join_strips_the_comment_token_of_the_next_line() {
    assert_eq!(
        run_in(Language::Rs, "// a\n// b\n", &[CMD_JOIN_LINES]),
        "// a b\n"
    );
}

#[test]
fn join_keeps_a_different_comment_token() {
    assert_eq!(
        run_in(Language::Rs, "// a\n/// b\n", &[CMD_JOIN_LINES]),
        "// a /// b\n"
    );
}

#[test]
fn join_over_a_selection_strips_only_the_first_lines_token() {
    assert_eq!(
        run_in(
            Language::Rs,
            "/// a\n/// b\n//! c\n",
            &[CMD_SELECT_LINE, CMD_SELECT_LINE, CMD_JOIN_LINES]
        ),
        "/// a b\n//! c\n"
    );
    assert_eq!(
        run_in(
            Language::Py,
            "# a\n   # b\nc\n",
            &[
                CMD_SELECT_LINE,
                CMD_SELECT_LINE,
                CMD_SELECT_LINE,
                CMD_JOIN_LINES
            ]
        ),
        "# a b c\n"
    );
}

#[test]
fn join_without_comment_tokens_joins_whitespace_only() {
    assert_eq!(
        run_in(Language::Md, "// a\n// b\n", &[CMD_JOIN_LINES]),
        "// a // b\n"
    );
}
