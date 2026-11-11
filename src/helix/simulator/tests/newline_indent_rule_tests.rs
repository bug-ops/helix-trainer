//! Newline-indent cases beyond the block openers of #516, closed by #540:
//! comment continuation, Enter between auto-pairs, Python dedent and
//! brackets, argument continuation, blank lines and unclosed blocks.
//!
//! Every expected buffer is a golden recorded from `hx` 25.07.1 started with
//! an empty configuration (`hx -c <file>`) that only turns `auto-format` off
//! (rustfmt on `:w` would rewrite valid Rust), on the same text and keys.
//! Unclosed trailing openers are a recorded divergence (see the test).

use super::newline_indent_tests::run_in;
use super::run_keys;
use crate::grammar::commands::*;
use crate::helix::Language;

fn check(language: Language, text: &str, keys: &[&str], expected: &str) {
    assert_eq!(run_in(language, text, keys), expected, "{text:?} {keys:?}");
}

const O: &str = CMD_OPEN_BELOW;
const UPPER_O: &str = CMD_OPEN_ABOVE;
const ENTER_AT_END: [&str; 2] = [CMD_APPEND_LINE_END, "\n"];

#[test]
fn comment_lines_continue_the_comment() {
    check(
        Language::Rs,
        "// foo (\nx;\n",
        &[O, "y"],
        "// foo (\n// y\nx;\n",
    );
    check(Language::Rs, "//foo\nx\n", &[O, "y"], "//foo\n// y\nx\n");
    check(
        Language::Rs,
        "//   foo\nx\n",
        &[O, "y"],
        "//   foo\n// y\nx\n",
    );
    check(Language::Rs, "/// d\nx\n", &[O, "y"], "/// d\n/// y\nx\n");
    check(
        Language::Rs,
        "/// d\nx\n",
        &[UPPER_O, "y"],
        "/// y\n/// d\nx\n",
    );
    check(Language::Rs, "//! d\nx\n", &[O, "y"], "//! d\n//! y\nx\n");
    check(Language::Rs, "//// d\nx\n", &[O, "y"], "//// d\n/// y\nx\n");
    check(
        Language::Py,
        "# note:\nx = 1\n",
        &[O, "y"],
        "# note:\n# y\nx = 1\n",
    );
    check(
        Language::Py,
        "## d\nx = 1\n",
        &[O, "y"],
        "## d\n# y\nx = 1\n",
    );
    check(
        Language::Py,
        "# c\nx = 1\n",
        &[UPPER_O, "y"],
        "# y\n# c\nx = 1\n",
    );
}

#[test]
fn comment_continuation_keeps_the_comment_indent() {
    let block = "fn f() {\n    // c\n    x;\n}\n";
    check(
        Language::Rs,
        block,
        &["j", UPPER_O, "y"],
        "fn f() {\n    // y\n    // c\n    x;\n}\n",
    );
    check(
        Language::Rs,
        block,
        &["j", O, "y"],
        "fn f() {\n    // c\n    // y\n    x;\n}\n",
    );
    check(
        Language::Rs,
        block,
        &["j", "j", UPPER_O, "y"],
        "fn f() {\n    // c\n    y\n    x;\n}\n",
    );
    check(
        Language::Rs,
        "fn f() {\n// c\n}\n",
        &["j", O, "y"],
        "fn f() {\n// c\n// y\n}\n",
    );
    check(
        Language::Rs,
        "// c\nx;\n",
        &["j", UPPER_O, "y"],
        "// c\ny\nx;\n",
    );
}

#[test]
fn comment_continuation_survives_escape_and_enter() {
    check(Language::Rs, "// foo\nx;\n", &[O], "// foo\n// \nx;\n");
    check(
        Language::Rs,
        "fn f() {\n    // c\n}\n",
        &["j", ENTER_AT_END[0], ENTER_AT_END[1], "y"],
        "fn f() {\n    // c\n    // y\n}\n",
    );
    check(
        Language::Rs,
        "// foo\nx;\n",
        &[ENTER_AT_END[0], "\n", "\n", "y"],
        "// foo\n//\n// y\nx;\n",
    );
    check(
        Language::Rs,
        "// foo\n",
        &["f/", "l", CMD_APPEND, "\n", "y"],
        "//\n// yfoo\n",
    );
}

#[test]
fn trailing_comment_is_not_a_continuation_and_does_not_hide_an_opener() {
    check(
        Language::Rs,
        "let a = 1; // c\n",
        &[O, "y"],
        "let a = 1; // c\ny\n",
    );
    check(
        Language::Rs,
        "fn f() { // c\n}\n",
        &[O, "y"],
        "fn f() { // c\n    y\n}\n",
    );
    check(
        Language::Py,
        "if x:  # c\n    y\n",
        &[O, "z"],
        "if x:  # c\n    z\n    y\n",
    );
}

#[test]
fn enter_between_an_auto_pair_splits_the_pair() {
    let cases: [(Language, &str, &str, &str); 8] = [
        (Language::Rs, "fn f() {}\n", "f{", "fn f() {\n    y\n}\n"),
        (Language::Rs, "f()\n", "f(", "f(\n    y\n)\n"),
        (Language::Rs, "x[]\n", "f[", "x[\n    y\n]\n"),
        (Language::Rs, "x = \"\"\n", "f\"", "x = \"\n    y\n\"\n"),
        (Language::Py, "x = {}\n", "f{", "x = {\n    y\n}\n"),
        (Language::Py, "x = ''\n", "f'", "x = '\n    y\n'\n"),
        (Language::Md, "x{}\n", "f{", "x{\n  y\n}\n"),
        (Language::Rs, "x = ''\n", "f'", "x = '\ny'\n"),
    ];
    for (language, text, find, expected) in cases {
        check(language, text, &[find, CMD_APPEND, "\n", "y"], expected);
    }
    check(
        Language::Rs,
        "fn f() {\n    if a {}\n}\n",
        &["j", "f{", CMD_APPEND, "\n", "y"],
        "fn f() {\n    if a {\n        y\n    }\n}\n",
    );
    check(
        Language::Rs,
        "x(]\n",
        &["f(", CMD_APPEND, "\n", "y"],
        "x(\n    y]\n",
    );
}

#[test]
fn enter_between_a_pair_keeps_the_indent_on_escape() {
    check(
        Language::Rs,
        "fn f() {}\n",
        &["f{", CMD_APPEND, "\n"],
        "fn f() {\n    \n}\n",
    );
}

#[test]
fn python_return_like_lines_dedent_one_unit() {
    let cases: [(&str, usize, &str); 7] = [
        (
            "class A:\n    def f(self):\n        return 1\n",
            2,
            "class A:\n    def f(self):\n        return 1\n    y\n",
        ),
        (
            "def f():\n    x = 1\n    return x\n",
            2,
            "def f():\n    x = 1\n    return x\ny\n",
        ),
        (
            "def f():\n    if a:\n        pass\n",
            2,
            "def f():\n    if a:\n        pass\n    y\n",
        ),
        ("def f():\n    raise E\n", 1, "def f():\n    raise E\ny\n"),
        ("while a:\n    break\n", 1, "while a:\n    break\ny\n"),
        (
            "while a:\n    if b:\n        continue\n",
            2,
            "while a:\n    if b:\n        continue\n    y\n",
        ),
        ("return 1\n", 0, "return 1\ny\n"),
    ];
    for (text, down, expected) in cases {
        let keys: Vec<&str> = std::iter::repeat_n("j", down).chain([O, "y"]).collect();
        check(Language::Py, text, &keys, expected);
    }
}

#[test]
fn python_dedent_needs_a_leading_keyword_and_no_trailing_comment() {
    let keep = |text: &str, expected: &str| check(Language::Py, text, &["j", O, "y"], expected);
    keep(
        "def f():\n    returned = 1\n",
        "def f():\n    returned = 1\n    y\n",
    );
    keep(
        "def f():\n    pass  # c\n",
        "def f():\n    pass  # c\n    y\n",
    );
    keep(
        "def f():\n    if a: return 1\n",
        "def f():\n    if a: return 1\n    y\n",
    );
    check(
        Language::Py,
        "def f():\n    return 1\n\nx = 2\n",
        &["j", "j", "j", UPPER_O, "y"],
        "def f():\n    return 1\n\ny\nx = 2\n",
    );
}

#[test]
fn python_brackets_open_a_block() {
    let cases: [(&str, &[&str], &str); 8] = [
        ("x = {\n}\n", &[O, "y"], "x = {\n    y\n}\n"),
        ("x = [\n]\n", &[O, "y"], "x = [\n    y\n]\n"),
        ("x = foo(\n)\n", &[O, "y"], "x = foo(\n    y\n)\n"),
        ("x = {1:\n2}\n", &[O, "y"], "x = {1:\n    y\n2}\n"),
        (
            "def f():\n    x = foo(\n    )\n",
            &["j", O, "y"],
            "def f():\n    x = foo(\n        y\n    )\n",
        ),
        (
            "x = {\n    1: 2,\n}\n",
            &["j", O, "y"],
            "x = {\n    1: 2,\n    y\n}\n",
        ),
        (
            "x = {\n    1: 2,\n}\n",
            &["j", "j", O, "y"],
            "x = {\n    1: 2,\n}\ny\n",
        ),
        (
            "x = {\n    1: 2,\n}\n",
            &["j", UPPER_O, "y"],
            "x = {\n    y\n    1: 2,\n}\n",
        ),
    ];
    for (text, keys, expected) in cases {
        check(Language::Py, text, keys, expected);
    }
    check(
        Language::Py,
        "x = [\n]\n",
        &[ENTER_AT_END[0], ENTER_AT_END[1], "y"],
        "x = [\n    y\n]\n",
    );
}

#[test]
fn python_open_bracket_beats_dedent_and_closer_line_dedents() {
    check(
        Language::Py,
        "def f():\n    x = 1\n    return {\n    }\n",
        &["j", "j", O, "y"],
        "def f():\n    x = 1\n    return {\n        y\n    }\n",
    );
    check(
        Language::Py,
        "def f():\n    return foo(\n        1,\n    )\n",
        &["j", "j", "j", O, "y"],
        "def f():\n    return foo(\n        1,\n    )\ny\n",
    );
}

#[test]
fn an_unclosed_opener_with_no_closer_gives_indent_zero() {
    // hx auto-pairs a typed opener, so a loaded text with an unmatched one is a
    // parse error and hx gives 0 (#597).
    let cases: [(Language, &str, &[&str], &str); 5] = [
        (
            Language::Rs,
            "fn f() -> u8 {\n",
            &[O, "y"],
            "fn f() -> u8 {\ny\n",
        ),
        (
            Language::Rs,
            "fn f() -> u8 {\n",
            &[ENTER_AT_END[0], ENTER_AT_END[1], "y"],
            "fn f() -> u8 {\ny\n",
        ),
        (
            Language::Rs,
            "let v = vec![\n    1,\n",
            &[O, "y"],
            "let v = vec![\ny\n    1,\n",
        ),
        (
            Language::Rs,
            "fn f() {\n    a;\n",
            &["j", UPPER_O, "y"],
            "fn f() {\ny\n    a;\n",
        ),
        (Language::Py, "x = {\n", &[O, "y"], "x = {\ny\n"),
    ];
    for (language, text, keys, expected) in cases {
        check(language, text, keys, expected);
    }
}

#[test]
fn unclosed_brackets_not_at_line_end_do_not_indent() {
    check(
        Language::Rs,
        "let a = foo(1,\n",
        &[O, "y"],
        "let a = foo(1,\ny\n",
    );
    check(Language::Py, "def f():\n", &[O, "y"], "def f():\n    y\n");
    check(
        Language::Rs,
        "fn f() {\n    if a {\n}\n",
        &["j", O, "y"],
        "fn f() {\n    if a {\n        y\n}\n",
    );
}

#[test]
fn argument_continuation_adds_a_unit() {
    check(
        Language::Rs,
        "let a = foo(1,\n    2);\n",
        &[O, "y"],
        "let a = foo(1,\n    y\n    2);\n",
    );
    check(
        Language::Rs,
        "let a = foo(1,\n",
        &[O, "y"],
        "let a = foo(1,\ny\n",
    );
    check(
        Language::Rs,
        "foo(bar(\n    1));\n",
        &[O, "y"],
        "foo(bar(\n    y\n    1));\n",
    );
    let call = "fn f() {\n    let a = foo(1,\n        2);\n}\n";
    check(
        Language::Rs,
        call,
        &["j", O, "y"],
        "fn f() {\n    let a = foo(1,\n        y\n        2);\n}\n",
    );
    check(
        Language::Rs,
        call,
        &["j", "j", O, "y"],
        "fn f() {\n    let a = foo(1,\n        2);\n    y\n}\n",
    );
    check(
        Language::Rs,
        "fn f() {\n    let a = foo(\n        1,\n    );\n}\n",
        &["j", "j", "j", O, "y"],
        "fn f() {\n    let a = foo(\n        1,\n    );\n    y\n}\n",
    );
    check(
        Language::Rs,
        "fn f(a: u8,\n    b: u8) {\n    c;\n}\n",
        &["j", O, "y"],
        "fn f(a: u8,\n    b: u8) {\n    y\n    c;\n}\n",
    );
    check(
        Language::Rs,
        "fn f() {\n    if a {\n        b;\n    } else {\n        c;\n    }\n}\n",
        &["j", "j", "j", O, "y"],
        "fn f() {\n    if a {\n        b;\n    } else {\n        y\n        c;\n    }\n}\n",
    );
}

#[test]
fn blank_lines_take_the_indent_of_the_nearest_line_above() {
    let body = "fn f() {\n    a;\n\n    b;\n}\n";
    check(
        Language::Rs,
        body,
        &["j", "j", O, "y"],
        "fn f() {\n    a;\n\n    y\n    b;\n}\n",
    );
    check(
        Language::Rs,
        body,
        &["j", "j", UPPER_O, "y"],
        "fn f() {\n    a;\n    y\n\n    b;\n}\n",
    );
    check(
        Language::Rs,
        body,
        &["j", "j", CMD_INSERT, "\n", "y"],
        "fn f() {\n    a;\n\ny\n    b;\n}\n",
    );
    check(
        Language::Rs,
        "fn f() {\n    a;\n\n\n    b;\n}\n",
        &["j", "j", "j", O, "y"],
        "fn f() {\n    a;\n\n\n    y\n    b;\n}\n",
    );
    check(
        Language::Rs,
        "fn f() {\n\n    b;\n}\n",
        &["j", O, "y"],
        "fn f() {\n\n    y\n    b;\n}\n",
    );
    check(Language::Rs, "a;\n\nb;\n", &["j", O, "y"], "a;\n\ny\nb;\n");
    check(
        Language::Rs,
        "fn f() {\n    a;\n}\n\nfn g() {}\n",
        &["j", "j", "j", O, "y"],
        "fn f() {\n    a;\n}\n\ny\nfn g() {}\n",
    );
    check(Language::Rs, "\nfn g() {}\n", &[O, "y"], "\ny\nfn g() {}\n");
    check(
        Language::Py,
        "def f():\n    a = 1\n\n    b = 2\n",
        &["j", "j", O, "y"],
        "def f():\n    a = 1\n\n    y\n    b = 2\n",
    );
}

#[test]
fn enter_on_a_comment_line_continues_it_even_between_a_pair() {
    check(
        Language::Rs,
        "// foo()\n",
        &["f(", CMD_APPEND, "\n", "y"],
        "// foo(\n// y)\n",
    );
    check(
        Language::Py,
        "# foo()\n",
        &["f(", CMD_APPEND, "\n", "y"],
        "# foo(\n# y)\n",
    );
}

#[test]
fn tabs_are_copied_and_extended_by_the_detected_unit() {
    let text = "fn f() {\n\tif a {\n\t\tb;\n\t}\n}\n";
    check(
        Language::Rs,
        text,
        &["j", O, "y"],
        "fn f() {\n\tif a {\n\t\ty\n\t\tb;\n\t}\n}\n",
    );
    check(
        Language::Rs,
        text,
        &["j", "j", "j", O, "y"],
        "fn f() {\n\tif a {\n\t\tb;\n\t}\n\ty\n}\n",
    );
    check(
        Language::Py,
        "def f():\n\treturn 1\n",
        &["j", O, "y"],
        "def f():\n\treturn 1\ny\n",
    );
}

#[test]
fn counted_open_repeats_the_rule() {
    check(
        Language::Rs,
        "fn f() {\n}\n",
        &["2o", "y"],
        "fn f() {\n    y\n    y\n}\n",
    );
    check(
        Language::Rs,
        "fn f() {\n    a;\n}\n",
        &["j", "2O", "y"],
        "fn f() {\n    y\n    y\n    a;\n}\n",
    );
}

#[test]
fn languages_without_a_rule_copy_the_indent() {
    let plain = |text: &str, keys: &[&str]| run_keys(text, keys).display().content();
    check(Language::Md, "  a {\n", &[O, "y"], "  a {\n  y\n");
    check(Language::Md, "  a {\n", &[UPPER_O, "y"], "  y\n  a {\n");
    check(
        Language::Md,
        "x\n  a\n",
        &["j", UPPER_O, "y"],
        "x\n  y\n  a\n",
    );
    check(Language::Md, "// c\n", &[O, "y"], "// c\ny\n");
    assert_eq!(plain("  a {\n", &[O, "y"]), "  a {\n  y\n");
    assert_eq!(plain("  a {\n", &[UPPER_O, "y"]), "  y\n  a {\n");
    assert_eq!(plain("x\n  a\n", &["j", UPPER_O, "y"]), "x\n  y\n  a\n");
    assert_eq!(plain("// c\n", &[O, "y"]), "// c\ny\n");
}

#[test]
fn enter_in_the_middle_of_a_python_line() {
    check(
        Language::Py,
        "def f():\n    x = 1\n",
        &["j", "f=", CMD_APPEND, "\n", "y"],
        "def f():\n    x =\n    y 1\n",
    );
    check(
        Language::Py,
        "if a:  # c\n    b\n",
        &["f:", CMD_APPEND, "\n", "y"],
        "if a:\n    y  # c\n    b\n",
    );
}

#[test]
fn python_colon_opens_only_on_compound_statement_lines() {
    check(Language::Py, "f = lambda:\n", &[O, "y"], "f = lambda:\ny\n");
    check(
        Language::Py,
        "if x:\n    y\n",
        &[O, "z"],
        "if x:\n    z\n    y\n",
    );
    check(Language::Py, "else:\n", &[O, "z"], "else:\n    z\n");
    check(Language::Py, "def f(x):\n", &[O, "z"], "def f(x):\n    z\n");
}

#[test]
fn colon_inside_a_string_at_the_cursor_does_not_open() {
    check(
        Language::Py,
        "x = \"a:b\"\n",
        &["f:", CMD_APPEND, "\n", "y"],
        "x = \"a:\nyb\"\n",
    );
}

#[test]
fn brackets_in_multi_line_literals_and_block_comments_are_not_counted() {
    check(
        Language::Py,
        "s = '''\n(\nfoo\n)\n'''\nx = 1\n",
        &["j", O, "y"],
        "s = '''\n(\ny\nfoo\n)\n'''\nx = 1\n",
    );
    check(
        Language::Py,
        "def f():\n    s = '''\n    (\n    '''\n    g()\n)\n",
        &["j", "j", O, "y"],
        "def f():\n    s = '''\n    (\n    y\n    '''\n    g()\n)\n",
    );
    check(
        Language::Rs,
        "/* (\n x\n) */\nfn f() {}\n",
        &[O, "y"],
        "/* (\ny\n x\n) */\nfn f() {}\n",
    );
    check(
        Language::Rs,
        "let s = r#\"(\n\"#;\nfoo();\n)\n",
        &[O, "y"],
        "let s = r#\"(\ny\n\"#;\nfoo();\n)\n",
    );
    check(
        Language::Rs,
        "fn f() {\n    let s = \"a\n(\nb\";\n    g();\n}\n",
        &["j", "j", O, "y"],
        "fn f() {\n    let s = \"a\n(\ny\nb\";\n    g();\n}\n",
    );
}

#[test]
fn comment_token_comes_from_the_whole_line() {
    check(
        Language::Rs,
        "/// d\n",
        &["l", CMD_APPEND, "\n", "y"],
        "//\n/// y/ d\n",
    );
}
