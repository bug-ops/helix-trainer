//! Line endings opened by Enter, `o` and `O` follow the document's (FR-051).
//!
//! `ab\r\ncd` cases are goldens from `hx` 25.07.1; the indented CRLF cases
//! apply the same rules to a CRLF block.

use super::{run_keys, run_keys_in};
use crate::grammar::commands::*;

fn typed(text: &str, keys: &[&str]) -> String {
    let keys: Vec<&str> = keys.iter().copied().chain([CMD_ESCAPE]).collect();
    run_keys(text, &keys).display().content()
}

#[test]
fn enter_at_the_end_of_a_crlf_line_inserts_a_crlf() {
    assert_eq!(typed("ab\r\ncd", &["A", "\n"]), "ab\r\n\r\ncd");
    assert_eq!(typed("ab\r\ncd", &["A", "\n", "x"]), "ab\r\nx\r\ncd");
}

#[test]
fn open_below_and_above_use_the_crlf_of_the_document() {
    assert_eq!(typed("ab\r\ncd", &["o"]), "ab\r\n\r\ncd");
    assert_eq!(typed("ab\r\ncd", &["o", "x"]), "ab\r\nx\r\ncd");
    assert_eq!(typed("ab\r\ncd", &["j", "O"]), "ab\r\n\r\ncd");
    assert_eq!(typed("ab\r\ncd", &["O", "x"]), "x\r\nab\r\ncd");
    assert_eq!(typed("ab\r\ncd", &["j", "O", "x"]), "ab\r\nx\r\ncd");
}

#[test]
fn counted_open_uses_the_crlf_of_the_document() {
    assert_eq!(typed("ab\r\ncd", &["2o", "x"]), "ab\r\nx\r\nx\r\ncd");
}

#[test]
fn an_unused_indent_after_a_crlf_is_stripped_like_after_an_lf() {
    let block = "fn f() {\r\n    a\r\n}";
    assert_eq!(
        run_keys_in("rs", block, &["j", "o", CMD_ESCAPE])
            .display()
            .content(),
        "fn f() {\r\n    a\r\n\r\n}"
    );
    assert_eq!(
        run_keys_in("rs", block, &["j", "A", "\n", CMD_ESCAPE])
            .display()
            .content(),
        "fn f() {\r\n    a\r\n\r\n}"
    );
    assert_eq!(
        run_keys_in("rs", block, &["j", "o", "x", CMD_ESCAPE])
            .display()
            .content(),
        "fn f() {\r\n    a\r\n    x\r\n}"
    );
}

#[test]
fn a_document_without_a_line_break_or_with_lf_first_keeps_lf() {
    assert_eq!(typed("ab", &["o", "x"]), "ab\nx");
    assert_eq!(typed("a\nb\r\nc", &["o", "x"]), "a\nx\nb\r\nc");
}
