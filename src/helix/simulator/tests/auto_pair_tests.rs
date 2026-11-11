//! Typed auto-pairs, pair-deleting Backspace and the unclosed-opener indent (#597).
//!
//! Every buffer is a golden recorded from `hx` 25.07.1 started with an empty
//! configuration (spec 3.8.6, tables D and E).

use super::run_keys_in;
use crate::helix::commands::*;

fn typed(language: &str, text: &str, keys: &[&str]) -> String {
    let keys: Vec<&str> = keys.iter().copied().chain([CMD_ESCAPE]).collect();
    run_keys_in(language, text, &keys).display().content()
}

#[test]
fn an_unclosed_opener_with_no_closer_gives_indent_zero() {
    assert_eq!(
        typed("rs", "fn f() -> u8 {", &["o", "x"]),
        "fn f() -> u8 {\nx"
    );
    assert_eq!(
        typed("rs", "fn f() -> u8 {", &["A", "\n", "x"]),
        "fn f() -> u8 {\nx"
    );
    assert_eq!(
        typed("rs", "fn f() -> u8 {\n}", &["o", "x"]),
        "fn f() -> u8 {\n    x\n}"
    );
    assert_eq!(
        typed("rs", "fn f() -> u8 {\n    a", &["o", "x"]),
        "fn f() -> u8 {\nx\n    a"
    );
    assert_eq!(typed("json", "{", &["o", "x"]), "{\nx");
}

#[test]
fn a_python_block_opener_still_indents() {
    assert_eq!(
        typed("py", "def f():", &["A", "\n", "x"]),
        "def f():\n    x"
    );
}

#[test]
fn backspace_between_a_pair_deletes_both_halves() {
    for opener in ["(", "[", "\""] {
        assert_eq!(
            typed("rs", "", &["i", opener, CMD_BACKSPACE]),
            "",
            "{opener}"
        );
    }
}

#[test]
fn typing_the_closer_skips_the_auto_inserted_one() {
    assert_eq!(typed("rs", "", &["i", "(", "x", ")", "y"]), "(x)y");
    assert_eq!(typed("rs", "", &["i", "(", ")", "y"]), "()y");
    assert_eq!(typed("rs", "", &["i", "(", "x"]), "(x)");
    assert_eq!(typed("rs", "", &["i", "\"", "x", "\"", "y"]), "\"x\"y");
}

#[test]
fn enter_between_a_typed_pair_splits_it() {
    assert_eq!(typed("rs", "", &["i", "{", "\n", "x"]), "{\n    x\n}");
}

#[test]
fn apostrophes_follow_the_language_pair_table() {
    for language in ["rs", "md"] {
        assert_eq!(
            typed(language, "", &["i", "d", "o", "n", "'"]),
            "don'",
            "{language}"
        );
    }
    for language in ["txt", "md"] {
        assert_eq!(typed(language, "", &["i", "'"]), "''", "{language}");
    }
    assert_eq!(typed("rs", "", &["i", "'"]), "'");
}

#[test]
fn an_opener_is_not_paired_before_an_alphanumeric() {
    assert_eq!(typed("rs", "ab", &["i", "("]), "(ab");
    assert_eq!(typed("rs", " ab", &["i", "("]), "() ab");
}

#[test]
fn every_cursor_pairs_on_its_own() {
    assert_eq!(typed("rs", "ab\ncd", &["C", "i", "("]), "(ab\n(cd");
    assert_eq!(typed("rs", "ab\ncd", &["A", "("]), "ab()\ncd");
    assert_eq!(typed("rs", "ab\ncd", &["C", "A", "("]), "ab()\ncd()");
    assert_eq!(typed("rs", "\n\n", &["C", "i", "("]), "()\n()\n");
}

#[test]
fn two_ranges_pair_at_each_range_start() {
    for language in ["rs", "md"] {
        assert_eq!(
            typed(language, "  a\n  b", &["v", "l", "C", "i", "("]),
            "()  a\n()  b",
            "{language}"
        );
    }
    assert_eq!(
        typed("rs", "ab cd\nef gh", &["v", "l", "C", "i", "("]),
        "(ab cd\n(ef gh"
    );
}

#[test]
fn three_typed_backticks_give_four_in_markdown() {
    let text = "print(\"hello\")";
    assert_eq!(typed("md", text, &["O", "`", "`"]), "``\nprint(\"hello\")");
    assert_eq!(
        typed("md", text, &["O", "`", "`", "`"]),
        "````\nprint(\"hello\")"
    );
    assert_eq!(
        typed(
            "md",
            text,
            &["O", "`", "`", "`", CMD_ESCAPE, "j", "o", "`", "`", "`"]
        ),
        "````\nprint(\"hello\")\n````"
    );
}
