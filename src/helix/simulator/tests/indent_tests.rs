//! Indent and dedent (`>`, `<`) tests (#500).
//!
//! Expected buffers match `hx` 25.07.1 on the same keys (probe: `d`).

use crate::grammar::commands::*;
use crate::helix::simulator::AnyModeSimulator;

const SCENARIO_14: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n";

fn run_in(language: &str, text: &str, keys: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new(text.to_string()).with_language(language);
    for key in keys {
        sim.execute_command(key).unwrap();
    }
    sim.display().content()
}

fn run(keys: &[&str]) -> String {
    run_in("rs", SCENARIO_14, keys)
}

#[test]
fn indent_adds_one_unit_to_the_cursor_line() {
    assert_eq!(
        run(&[CMD_INDENT, CMD_DELETE_SELECTION]),
        "    n add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn indent_touches_every_selected_line() {
    assert_eq!(
        run(&[CMD_SELECT_LINE, CMD_INDENT, CMD_DELETE_SELECTION]),
        "        let sum = a + b;\n    sum\n}\n"
    );
    assert_eq!(
        run(&[
            CMD_SELECT_LINE,
            CMD_SELECT_LINE,
            CMD_INDENT,
            CMD_DELETE_SELECTION
        ]),
        "        sum\n}\n"
    );
}

#[test]
fn indent_touches_the_lines_of_every_range() {
    assert_eq!(
        run(&[CMD_SELECT_ALL, "s a", CMD_INDENT, CMD_DELETE_SELECTION]),
        "    fn dd(: i32, b: i32) -> i32 {\n        let sum =  + b;\n    sum\n}\n"
    );
}

#[test]
fn indent_skips_blank_lines() {
    assert_eq!(
        run_in("rs", "a\n\nb\n", &[CMD_SELECT_ALL, CMD_INDENT]),
        "    a\n\n    b\n"
    );
}

#[test]
fn counted_indent_indents_that_many_times() {
    assert_eq!(
        run(&["2>", CMD_DELETE_SELECTION]),
        "        n add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}\n"
    );
    assert_eq!(
        run(&[CMD_SELECT_LINE, CMD_SELECT_LINE, "2>", CMD_DELETE_SELECTION]),
        "            sum\n}\n"
    );
}

#[test]
fn dedent_removes_one_unit_from_the_cursor_line() {
    assert_eq!(
        run(&[CMD_MOVE_DOWN, CMD_DEDENT, CMD_DELETE_SELECTION]),
        "fn add(a: i32, b: i32) -> i32 {\net sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn dedent_touches_every_selected_line() {
    assert_eq!(
        run(&[
            CMD_MOVE_DOWN,
            CMD_SELECT_LINE,
            CMD_DEDENT,
            CMD_DELETE_SELECTION
        ]),
        "fn add(a: i32, b: i32) -> i32 {\n    sum\n}\n"
    );
}

#[test]
fn counted_dedent_removes_that_many_units() {
    assert_eq!(
        run(&[CMD_MOVE_DOWN, CMD_MOVE_DOWN, "2<", CMD_DELETE_SELECTION]),
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\num\n}\n"
    );
}

#[test]
fn indent_then_dedent_roundtrips_one_unit() {
    assert_eq!(
        run(&[
            CMD_MOVE_DOWN,
            CMD_MOVE_DOWN,
            CMD_INDENT,
            CMD_INDENT,
            CMD_DEDENT,
            CMD_DELETE_SELECTION
        ]),
        "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n       sum\n}\n"
    );
}

#[test]
fn indent_unit_follows_the_language() {
    assert_eq!(
        run_in("md", "- a\n- b\n", &[CMD_SELECT_ALL, CMD_INDENT]),
        "  - a\n  - b\n"
    );
    assert_eq!(
        run_in("py", "x\ny\n", &[CMD_INDENT, CMD_INDENT]),
        "        x\ny\n"
    );
    assert_eq!(
        run_in(
            "md",
            "- a\n- b\n",
            &[CMD_MOVE_DOWN, CMD_INDENT, CMD_INDENT, CMD_DEDENT]
        ),
        "- a\n  - b\n"
    );
}

#[test]
fn dedent_removes_the_whole_unit_in_python() {
    assert_eq!(
        run_in("py", "def f():\n    pass\n", &[CMD_MOVE_DOWN, CMD_DEDENT]),
        "def f():\npass\n"
    );
}

#[test]
fn unlisted_language_uses_the_default_unit() {
    assert_eq!(run_in("js", "x\n", &[CMD_INDENT]), "    x\n");
}

#[test]
fn every_scenario_language_has_an_indent_unit_entry() {
    use crate::config::{DEFAULT_LANGUAGE, ScenarioLoader};
    use crate::helix::IndentUnit;

    let loader = ScenarioLoader::new();
    for locale in loader.available_locales() {
        for scenario in loader.load_from_embedded(&locale).unwrap() {
            let language = scenario
                .setup
                .language
                .as_deref()
                .unwrap_or(DEFAULT_LANGUAGE);
            assert!(
                IndentUnit::for_language(language).is_some(),
                "scenario {} uses language {language:?} without an indent unit entry",
                scenario.id
            );
        }
    }
}
