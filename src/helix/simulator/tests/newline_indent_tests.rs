//! Indent of the line `o`, `O` and Enter create, by the per-language rule of
//! #516.
//!
//! Every expected buffer is a golden recorded from `hx` 25.07.1 started with
//! an empty configuration (`hx -c <file>`, which only turns `auto-format` off
//! because rustfmt on `:w` would rewrite valid Rust) on the same text and keys.
//! Cases where the rule differs from `hx` are not asserted here; they are
//! listed as divergences in the helix-core readiness spec (3.8.4).

use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

pub(super) fn run_in(language: &str, text: &str, keys: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new(text.to_string()).with_language(language);
    for key in keys {
        sim.execute_command(key).unwrap();
    }
    sim.execute_command(CMD_ESCAPE).unwrap();
    sim.state().unwrap().content().to_string()
}

const MAIN: &str = "fn main() {\n    let x = 1;\n}\n";

#[test]
fn rust_block_body() {
    let cases: [(&[&str], &str); 8] = [
        (
            &[CMD_OPEN_BELOW, "x"],
            "fn main() {\n    x\n    let x = 1;\n}\n",
        ),
        (
            &[CMD_OPEN_ABOVE, "x"],
            "x\nfn main() {\n    let x = 1;\n}\n",
        ),
        (
            &["j", CMD_OPEN_BELOW, "x"],
            "fn main() {\n    let x = 1;\n    x\n}\n",
        ),
        (
            &["j", CMD_OPEN_ABOVE, "x"],
            "fn main() {\n    x\n    let x = 1;\n}\n",
        ),
        (
            &["j", "j", CMD_OPEN_BELOW, "x"],
            "fn main() {\n    let x = 1;\n}\nx\n",
        ),
        (
            &["j", "j", CMD_OPEN_ABOVE, "x"],
            "fn main() {\n    let x = 1;\n    x\n}\n",
        ),
        (
            &[CMD_APPEND_LINE_END, "\n", "x"],
            "fn main() {\n    x\n    let x = 1;\n}\n",
        ),
        (
            &["j", CMD_APPEND_LINE_END, "\n", "x"],
            "fn main() {\n    let x = 1;\n    x\n}\n",
        ),
    ];
    for (keys, expected) in cases {
        assert_eq!(run_in("rs", MAIN, keys), expected, "{keys:?}");
    }
}

#[test]
fn rust_every_opening_bracket_adds_a_unit() {
    let vec = "let v = vec![\n    1,\n];\n";
    assert_eq!(
        run_in("rs", vec, &[CMD_OPEN_BELOW, "x"]),
        "let v = vec![\n    x\n    1,\n];\n"
    );
    assert_eq!(
        run_in("rs", vec, &["j", CMD_OPEN_BELOW, "x"]),
        "let v = vec![\n    1,\n    x\n];\n"
    );
    assert_eq!(
        run_in("rs", vec, &["j", "j", CMD_OPEN_BELOW, "x"]),
        "let v = vec![\n    1,\n];\nx\n"
    );
    assert_eq!(
        run_in("rs", vec, &["j", "j", CMD_OPEN_ABOVE, "x"]),
        "let v = vec![\n    1,\n    x\n];\n"
    );
    let call = "foo(\n    a,\n);\n";
    assert_eq!(
        run_in("rs", call, &[CMD_OPEN_BELOW, "x"]),
        "foo(\n    x\n    a,\n);\n"
    );
    assert_eq!(
        run_in("rs", call, &[CMD_APPEND_LINE_END, "\n", "x"]),
        "foo(\n    x\n    a,\n);\n"
    );
}

#[test]
fn rust_nested_blocks() {
    let nested = "if a {\n    if b {\n        c;\n    }\n}\n";
    let cases: [(&[&str], &str); 4] = [
        (
            &["j", CMD_OPEN_BELOW, "x"],
            "if a {\n    if b {\n        x\n        c;\n    }\n}\n",
        ),
        (
            &["j", "j", "j", CMD_OPEN_BELOW, "x"],
            "if a {\n    if b {\n        c;\n    }\n    x\n}\n",
        ),
        (
            &["j", "j", "j", CMD_OPEN_ABOVE, "x"],
            "if a {\n    if b {\n        c;\n        x\n    }\n}\n",
        ),
        (
            &["j", "j", CMD_OPEN_ABOVE, "x"],
            "if a {\n    if b {\n        x\n        c;\n    }\n}\n",
        ),
    ];
    for (keys, expected) in cases {
        assert_eq!(run_in("rs", nested, keys), expected, "{keys:?}");
    }
}

#[test]
fn rust_lines_that_do_not_open_a_block_copy_the_indent() {
    assert_eq!(
        run_in("rs", "x;\n\ny;\n", &["j", CMD_OPEN_BELOW, "z"]),
        "x;\n\nz\ny;\n"
    );
    assert_eq!(
        run_in("rs", "x;\n\ny;\n", &["j", "j", CMD_OPEN_ABOVE, "z"]),
        "x;\n\nz\ny;\n"
    );
    assert_eq!(
        run_in("rs", "if a { b }\n", &[CMD_OPEN_BELOW, "x"]),
        "if a { b }\nx\n"
    );
    assert_eq!(
        run_in("rs", "if a { b }\n", &[CMD_APPEND_LINE_END, "\n", "x"]),
        "if a { b }\nx\n"
    );
    assert_eq!(
        run_in("rs", "fn f() {}\n", &[CMD_OPEN_BELOW, "x"]),
        "fn f() {}\nx\n"
    );
    assert_eq!(
        run_in("rs", "    let x = 1;\n", &[CMD_OPEN_BELOW, "y"]),
        "    let x = 1;\n    y\n"
    );
    assert_eq!(
        run_in("rs", "\tlet x = 1;\n", &[CMD_OPEN_BELOW, "y"]),
        "\tlet x = 1;\n\ty\n"
    );
}

#[test]
fn rust_match_and_struct_bodies() {
    let arms = "match x {\n    A => 1,\n    B => 2,\n}\n";
    assert_eq!(
        run_in("rs", arms, &[CMD_OPEN_BELOW, "x"]),
        "match x {\n    x\n    A => 1,\n    B => 2,\n}\n"
    );
    assert_eq!(
        run_in("rs", arms, &["j", CMD_OPEN_BELOW, "x"]),
        "match x {\n    A => 1,\n    x\n    B => 2,\n}\n"
    );
    assert_eq!(
        run_in(
            "rs",
            "struct S {\n    a: u8,\n}\n",
            &["j", CMD_OPEN_BELOW, "b"]
        ),
        "struct S {\n    a: u8,\n    b\n}\n"
    );
    assert_eq!(
        run_in("rs", "use a::{\n    b,\n};\n", &[CMD_OPEN_BELOW, "x"]),
        "use a::{\n    x\n    b,\n};\n"
    );
}

#[test]
fn rust_enter_uses_the_text_before_the_cursor() {
    assert_eq!(
        run_in("rs", "fn f() { x }\n", &["f{", CMD_APPEND, "\n", "y"]),
        "fn f() {\n    y x }\n"
    );
    let body = "fn f() {\n    let a = 1;\n}\n";
    assert_eq!(
        run_in("rs", body, &["j", "f=", CMD_APPEND, "\n", "y"]),
        "fn f() {\n    let a =\n    y 1;\n}\n"
    );
    assert_eq!(
        run_in("rs", body, &["j", CMD_APPEND_LINE_END, "\n", "\n", "y"]),
        "fn f() {\n    let a = 1;\n\n    y\n}\n"
    );
    assert_eq!(
        run_in("rs", body, &["j", CMD_OPEN_BELOW, "\n", "y"]),
        "fn f() {\n    let a = 1;\n\n    y\n}\n"
    );
}

#[test]
fn python_colon_adds_a_unit() {
    let def = "def f():\n    x = 1\n";
    assert_eq!(
        run_in("py", def, &[CMD_OPEN_BELOW, "y"]),
        "def f():\n    y\n    x = 1\n"
    );
    assert_eq!(
        run_in("py", def, &["j", CMD_OPEN_BELOW, "y"]),
        "def f():\n    x = 1\n    y\n"
    );
    assert_eq!(
        run_in("py", def, &["j", CMD_OPEN_ABOVE, "y"]),
        "def f():\n    y\n    x = 1\n"
    );
    assert_eq!(
        run_in("py", def, &["j", CMD_APPEND_LINE_END, "\n", "y"]),
        "def f():\n    x = 1\n    y\n"
    );
    assert_eq!(
        run_in("py", "for i in x:  \n    y\n", &[CMD_OPEN_BELOW, "z"]),
        "for i in x:  \n    z\n    y\n"
    );
}

#[test]
fn python_else_and_nested_blocks() {
    let branches = "if a:\n    b\nelse:\n    c\n";
    assert_eq!(
        run_in("py", branches, &["j", "j", CMD_OPEN_BELOW, "y"]),
        "if a:\n    b\nelse:\n    y\n    c\n"
    );
    assert_eq!(
        run_in("py", branches, &["j", CMD_OPEN_ABOVE, "y"]),
        "if a:\n    y\n    b\nelse:\n    c\n"
    );
    assert_eq!(
        run_in("py", branches, &["j", "j", CMD_OPEN_ABOVE, "y"]),
        "if a:\n    b\n    y\nelse:\n    c\n"
    );
    let class = "class A:\n    def f(self):\n        x = 1\n";
    assert_eq!(
        run_in("py", class, &["j", CMD_OPEN_BELOW, "y"]),
        "class A:\n    def f(self):\n        y\n        x = 1\n"
    );
    assert_eq!(
        run_in("py", class, &["j", "j", CMD_OPEN_BELOW, "y"]),
        "class A:\n    def f(self):\n        x = 1\n        y\n"
    );
    assert_eq!(
        run_in("py", class, &[CMD_APPEND_LINE_END, "\n", "y"]),
        "class A:\n    y\n    def f(self):\n        x = 1\n"
    );
}

#[test]
fn markdown_copies_the_indent() {
    assert_eq!(run_in("md", "# h\n", &[CMD_OPEN_BELOW, "x"]), "# h\nx\n");
    assert_eq!(
        run_in("md", "text:\n", &[CMD_OPEN_BELOW, "x"]),
        "text:\nx\n"
    );
    assert_eq!(
        run_in("md", "  indented\n", &[CMD_OPEN_BELOW, "x"]),
        "  indented\n  x\n"
    );
    assert_eq!(
        run_in("md", "- a\n  - b\n", &["j", CMD_OPEN_BELOW, "x"]),
        "- a\n  - b\n  x\n"
    );
}

#[test]
fn detected_unit_is_used_for_the_extra_level() {
    assert_eq!(
        run_in(
            "rs",
            "fn f() {\n  let a = 1;\n  let b = 2;\n}\n",
            &[CMD_OPEN_BELOW, "x"]
        ),
        "fn f() {\n  x\n  let a = 1;\n  let b = 2;\n}\n"
    );
}
