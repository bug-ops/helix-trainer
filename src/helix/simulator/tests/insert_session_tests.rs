//! An Insert session is one undo step (#571) and `a` selects through the
//! inserted text (#589).
//!
//! Expected values were recorded from `hx` 25.07.1 started with an empty
//! configuration on scenario 14, each sequence followed by the probe
//! `c @ Escape` that exposes the selection a command leaves behind.

use crate::helix::commands::*;
use crate::helix::registry::{ModeTransition, normal_registry};
use crate::helix::simulator::{
    AnyModeSimulator, HelixSimulator, InsertEntryCommand, NormalMode,
    tests::{SCENARIO_14, ranges_of, run_keys},
};

/// Content after `keys` and the probe `c @ Escape`.
fn probed(keys: &[&str]) -> String {
    super::probed(SCENARIO_14, "txt", keys)
}

fn first_line(content: &str) -> &str {
    content.lines().next().unwrap_or_default()
}

#[test]
fn u_after_insert_session_restores_text_and_selection() {
    let content = probed(&[CMD_INSERT, "a", "b", CMD_ESCAPE, CMD_UNDO]);
    assert_eq!(first_line(&content), "@n add(a: i32, b: i32) -> i32 {");
}

#[test]
fn u_after_three_typed_characters_is_one_step() {
    let content = probed(&[CMD_INSERT, "a", "b", "c", CMD_ESCAPE, CMD_UNDO]);
    assert_eq!(first_line(&content), "@n add(a: i32, b: i32) -> i32 {");
}

#[test]
fn u_after_append_session_restores_the_extended_selection() {
    let content = probed(&[CMD_APPEND, "x", "y", CMD_ESCAPE, CMD_UNDO]);
    assert_eq!(first_line(&content), "@ add(a: i32, b: i32) -> i32 {");
}

#[test]
fn u_after_open_below_removes_the_opened_line() {
    let content = probed(&[CMD_OPEN_BELOW, "x", "y", CMD_ESCAPE, CMD_UNDO]);
    assert_eq!(first_line(&content), "@n add(a: i32, b: i32) -> i32 {");
    assert_eq!(content.lines().count(), 4);
}

#[test]
fn u_after_change_restores_deleted_text_in_one_step() {
    let content = probed(&[CMD_CHANGE, "@", CMD_ESCAPE, CMD_UNDO]);
    assert_eq!(first_line(&content), "@n add(a: i32, b: i32) -> i32 {");
}

#[test]
fn redo_reapplies_the_whole_session() {
    let content = probed(&[CMD_INSERT, "a", "b", CMD_ESCAPE, CMD_UNDO, CMD_REDO]);
    assert_eq!(first_line(&content), "ab@n add(a: i32, b: i32) -> i32 {");
}

/// Line 2 of the document after `j w <entry> x y Escape u c @ Escape`.
fn line_two_after_entry(entry: &str) -> String {
    let content = probed(&[
        CMD_MOVE_DOWN,
        CMD_MOVE_WORD_FORWARD,
        entry,
        "x",
        "y",
        CMD_ESCAPE,
        CMD_UNDO,
    ]);
    content.lines().nth(1).unwrap_or_default().to_string()
}

#[test]
fn restored_selection_per_entering_command_matches_hx() {
    let table = [
        (CMD_INSERT, "@let sum = a + b;"),
        (CMD_APPEND, "@et sum = a + b;"),
        (CMD_INSERT_LINE_START, "    @et sum = a + b;"),
        (CMD_APPEND_LINE_END, "    let sum = a + b;@    sum"),
        (CMD_OPEN_BELOW, "@let sum = a + b;"),
        (CMD_OPEN_ABOVE, "@let sum = a + b;"),
        (CMD_CHANGE, "@let sum = a + b;"),
    ];
    for (entry, expected) in table {
        assert_eq!(line_two_after_entry(entry), expected, "entry `{entry}`");
    }
}

#[test]
fn insert_session_records_exactly_one_history_entry() {
    let mut sim = HelixSimulator::<NormalMode>::new(SCENARIO_14.to_string())
        .begin_insert(InsertEntryCommand::Insert)
        .map_err(|failed| failed.1)
        .unwrap();
    sim.insert_text("a").unwrap();
    sim.insert_text("b").unwrap();
    assert_eq!(sim.history.len(), 0, "an open session is not exposed");
    let sim = sim.exit_insert_mode();
    assert_eq!(sim.history.len(), 1);
}

#[test]
fn change_with_typed_text_is_one_history_entry() {
    let mut sim = HelixSimulator::<NormalMode>::new(SCENARIO_14.to_string())
        .begin_insert(InsertEntryCommand::Change { register: None })
        .map_err(|failed| failed.1)
        .unwrap();
    sim.insert_text("@").unwrap();
    assert_eq!(sim.exit_insert_mode().history.len(), 1);
}

#[test]
fn session_without_net_change_keeps_redo_history() {
    // x d u leaves one redo entry; `i Escape` must not discard it.
    let sim = run_keys(
        SCENARIO_14,
        &[
            CMD_SELECT_LINE,
            CMD_DELETE_SELECTION,
            CMD_UNDO,
            CMD_INSERT,
            CMD_ESCAPE,
            CMD_REDO,
        ],
    );
    assert_eq!(
        sim.display().content(),
        "    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn typed_then_deleted_characters_record_no_entry() {
    let sim = run_keys(
        SCENARIO_14,
        &[
            CMD_SELECT_LINE,
            CMD_DELETE_SELECTION,
            CMD_UNDO,
            CMD_INSERT,
            "a",
            CMD_BACKSPACE,
            CMD_ESCAPE,
            CMD_REDO,
        ],
    );
    assert_eq!(
        sim.display().content(),
        "    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn change_and_retype_of_the_same_text_keeps_redo_history() {
    let sim = run_keys(
        SCENARIO_14,
        &[
            CMD_INSERT, "a", CMD_ESCAPE, CMD_UNDO, CMD_CHANGE, "f", CMD_ESCAPE, CMD_REDO,
        ],
    );
    assert_eq!(sim.display().content(), format!("a{SCENARIO_14}"));
}

#[test]
fn session_with_a_change_clears_pending_redo() {
    let sim = run_keys(
        SCENARIO_14,
        &[
            CMD_INSERT, "a", CMD_ESCAPE, CMD_UNDO, CMD_INSERT, "c", CMD_ESCAPE, CMD_REDO,
        ],
    );
    assert_eq!(sim.display().content(), format!("c{SCENARIO_14}"));
}

#[test]
fn undo_after_a_session_leaves_earlier_steps_alone() {
    let sim = run_keys(
        SCENARIO_14,
        &[
            CMD_SELECT_LINE,
            CMD_DELETE_SELECTION,
            CMD_INSERT,
            "a",
            CMD_ESCAPE,
            CMD_UNDO,
        ],
    );
    assert_eq!(
        sim.display().content(),
        "    let sum = a + b;\n    sum\n}\n"
    );
}

#[test]
fn dot_repeat_of_a_session_is_one_undo_step() {
    let once = run_keys(SCENARIO_14, &[CMD_INSERT, "a", "b", CMD_ESCAPE]);
    let twice = run_keys(
        SCENARIO_14,
        &[CMD_INSERT, "a", "b", CMD_ESCAPE, CMD_REPEAT, CMD_UNDO],
    );
    assert_eq!(twice.display().content(), once.display().content());
}

#[test]
fn auto_indent_is_kept_once_text_is_typed_after_it() {
    let mut sim = AnyModeSimulator::new("foo {".to_string()).with_language("rs");
    for key in [CMD_OPEN_BELOW, "x", CMD_ESCAPE] {
        sim.execute_command(key).unwrap();
    }
    assert_eq!(sim.display().content(), "foo {\n    x");
}

#[test]
fn unused_auto_indent_is_stripped_inside_the_same_undo_step() {
    let mut sim = AnyModeSimulator::new("foo {".to_string()).with_language("rs");
    for key in [CMD_OPEN_BELOW, CMD_ESCAPE] {
        sim.execute_command(key).unwrap();
    }
    assert_eq!(sim.display().content(), "foo {\n");
    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.display().content(), "foo {");
}

#[test]
fn append_then_typing_selects_from_the_original_start_through_the_text() {
    let mut sim = run_keys(SCENARIO_14, &[CMD_APPEND, "a", "b", CMD_ESCAPE]);
    assert_eq!(ranges_of(&sim), vec![(0, 3)]);
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert!(sim.display().content().starts_with("n add(a: i32"));
}

#[test]
fn append_after_a_word_selection_keeps_the_selection_start() {
    let mut sim = run_keys(
        SCENARIO_14,
        &[CMD_MOVE_WORD_FORWARD, CMD_APPEND, "x", "y", CMD_ESCAPE],
    );
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert!(sim.display().content().starts_with("add(a: i32"));
}

#[test]
fn append_without_typing_leaves_a_plain_cursor() {
    let sim = run_keys(SCENARIO_14, &[CMD_APPEND, CMD_ESCAPE]);
    assert_eq!(ranges_of(&sim), vec![(0, 0)]);
}

#[test]
fn insert_entry_commands_round_trip_through_parse() {
    let all = [
        InsertEntryCommand::Insert,
        InsertEntryCommand::Append,
        InsertEntryCommand::InsertLineStart,
        InsertEntryCommand::AppendLineEnd,
        InsertEntryCommand::OpenBelow,
        InsertEntryCommand::OpenAbove,
        InsertEntryCommand::Change { register: None },
        InsertEntryCommand::Change {
            register: Some('a'),
        },
        InsertEntryCommand::Change {
            register: Some('_'),
        },
        InsertEntryCommand::ChangeNoYank,
    ];
    for entry in all {
        assert_eq!(InsertEntryCommand::parse(&entry.command()), Some(entry));
    }
}

#[test]
fn non_entering_commands_are_not_insert_entries() {
    for cmd in [
        CMD_MOVE_LEFT,
        CMD_DELETE_SELECTION,
        CMD_YANK,
        CMD_ESCAPE,
        "\"ad",
        "\"ay",
        "\"aaa",
        "\"c",
        "",
        "xyz",
    ] {
        assert_eq!(InsertEntryCommand::parse(cmd), None, "`{cmd}`");
    }
}

#[test]
fn registry_insert_entries_equal_what_parse_accepts() {
    let mut registry_keys: Vec<&str> = normal_registry()
        .all_commands()
        .filter(|meta| meta.mode_change() == Some(ModeTransition::ToInsert))
        .map(crate::helix::registry::metadata::CommandMetadata::key)
        .collect();
    registry_keys.sort_unstable();

    let mut parsed_keys: Vec<&str> = registry_keys
        .iter()
        .copied()
        .filter(|key| InsertEntryCommand::parse(key).is_some())
        .collect();
    parsed_keys.sort_unstable();
    assert_eq!(registry_keys, parsed_keys, "every ToInsert key must parse");

    let mut plain: Vec<String> = [
        InsertEntryCommand::Insert,
        InsertEntryCommand::Append,
        InsertEntryCommand::InsertLineStart,
        InsertEntryCommand::AppendLineEnd,
        InsertEntryCommand::OpenBelow,
        InsertEntryCommand::OpenAbove,
        InsertEntryCommand::Change { register: None },
        InsertEntryCommand::ChangeNoYank,
    ]
    .map(InsertEntryCommand::command)
    .to_vec();
    plain.sort_unstable();
    assert_eq!(
        plain, registry_keys,
        "every plain entry must be a ToInsert key"
    );
}
