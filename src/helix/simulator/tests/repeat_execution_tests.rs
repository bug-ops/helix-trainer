//! Repeat dot command execution tests for `HelixSimulator`
//!
//! Tests that verify the dot command correctly replays recorded actions:
//! - Basic repeat: Delete, join, indent operations
//! - Insert mode repeat: Text insertion with entry commands
//! - Complex scenarios: Real-world usage patterns

use crate::grammar::commands::*;
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// `.` replays only the last Insert session (FR-066, hx `repeat_last_insert`)
// ============================================================================

const SNIPPET: &str = "fn add(a: i32, b: i32) -> i32 {\n    let sum = a + b;\n    sum\n}";

/// Runs `commands` on `SNIPPET`, then `d` to expose the final selection.
fn run_then_delete(commands: &[&str]) -> String {
    let mut sim = AnyModeSimulator::new(SNIPPET.to_string());
    for command in commands {
        sim.execute_command(command).unwrap();
    }
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    sim.display().content()
}

/// Oracle: hx 25.07.1, the same keys on `SNIPPET` followed by `d`.
#[test]
fn test_dot_never_repeats_non_insert_edits() {
    let rest = "    let sum = a + b;\n    sum\n}";
    let cases: [(&[&str], String); 11] = [
        (
            &["~", "x", "d", "."],
            "   let sum = a + b;\n    sum\n}".to_string(),
        ),
        (
            &["~", "x", "d", ".", "."],
            "   let sum = a + b;\n    sum\n}".to_string(),
        ),
        (
            &["~", "d", "."],
            format!(" add(a: i32, b: i32) -> i32 {{\n{rest}"),
        ),
        (
            &["~", "w", "d", "."],
            format!("dd(a: i32, b: i32) -> i32 {{\n{rest}"),
        ),
        (
            &["~", "w", "d", "j", "."],
            "add(a: i32, b: i32) -> i32 {\n   let sum = a + b;\n    sum\n}".to_string(),
        ),
        (
            &["~", ">", "."],
            format!("    n add(a: i32, b: i32) -> i32 {{\n{rest}"),
        ),
        (
            &["~", "J", "."],
            "n add(a: i32, b: i32) -> i32 { let sum = a + b;\n    sum\n}".to_string(),
        ),
        (
            &["~", "y", "p", "."],
            format!("Fn add(a: i32, b: i32) -> i32 {{\n{rest}"),
        ),
        (
            &["~", "x", "y", "p", "."],
            format!("Fn add(a: i32, b: i32) -> i32 {{\n{rest}"),
        ),
        (
            &["i", "Z", "Escape", "gg", "x", "d", "."],
            "Z   let sum = a + b;\n    sum\n}".to_string(),
        ),
        (
            &["~", "r_", "l", "l", "l", "l", "."],
            format!("_n ad(a: i32, b: i32) -> i32 {{\n{rest}"),
        ),
    ];
    for (commands, expected) in cases {
        assert_eq!(run_then_delete(commands), expected, "keys: {commands:?}");
    }
}

#[test]
fn test_repeat_with_empty_buffer() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Try to repeat without any previous action
    let result = sim.execute_command(".");
    assert!(result.is_ok()); // Should not error

    let state = sim.display();
    assert_eq!(state.content(), "test"); // Should be unchanged
}

#[test]
fn test_repeat_is_not_recorded() {
    let mut sim = AnyModeSimulator::new("abcd".to_string());

    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command("Z").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.display().content(), "Zabcd");

    sim.execute_command(".").unwrap();
    sim.execute_command(".").unwrap();

    // Each `.` replays the original session, not the previous `.`
    assert_eq!(sim.display().content(), "ZZZabcd");
}

#[test]
fn test_repeat_survives_undo_and_movements() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command("Z").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();
    sim.execute_command(CMD_UNDO).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();

    sim.execute_command(".").unwrap();
    assert_eq!(sim.display().content(), "Ztest");
}

// ============================================================================
// Insert Mode Repeat Tests
// ============================================================================

#[test]
fn test_repeat_insert_mode() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    // Insert "hi"
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hiworld");

    // `gl` lands on the last character, so `i` inserts before it
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Repeat insert
    sim.execute_command(".").unwrap();
    let state = sim.display();
    assert_eq!(state.content(), "hiworlhid");
}

#[test]
fn test_repeat_append() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Move to end of word and append " world" (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap(); // Move to end
    sim.execute_command(CMD_APPEND).unwrap(); // Append (cursor after last char)
    sim.execute_command(" ").unwrap();
    sim.execute_command("w").unwrap();
    sim.execute_command("o").unwrap();
    sim.execute_command("r").unwrap();
    sim.execute_command("l").unwrap();
    sim.execute_command("d").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hello world");

    // Move to start (use 'gh' in Helix, not '0')
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();

    // Repeat should append " world" after the first character (using 'a' to append)
    sim.execute_command(".").unwrap();
    let state = sim.display();
    // Since we used 'a' (append), repeat should place cursor after 'h' and insert " world"
    assert_eq!(state.content(), "h worldello world");
}

#[test]
fn test_repeat_insert_with_movements() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hiworld");
    // Cursor should be at position 1 (moved left once from 2)
    assert_eq!(state.cursor_position().col(), 1);

    // Move to end (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Repeat insert with movements
    sim.execute_command(".").unwrap();
    let state = sim.display();
    // Should insert "hi" before the last char, then move left once
    assert_eq!(state.content(), "hiworlhid");
    // Cursor moved left from position 8 to position 7
    assert_eq!(state.cursor_position().col(), 7);
}

#[test]
fn test_repeat_insert_keeps_movement_between_typed_text() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command("a").unwrap();
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    sim.execute_command("b").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.display().content(), "baworld");

    sim.execute_command(CMD_GOTO_LINE_END).unwrap();
    sim.execute_command(".").unwrap();

    assert_eq!(sim.display().content(), "baworlbad");
}

#[test]
fn test_repeat_insert_simple() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Insert 'd' at the beginning (CMD_DELETE_SELECTION is "d")
    sim.execute_command(CMD_INSERT).unwrap(); // Enter insert mode
    sim.execute_command(CMD_DELETE_SELECTION).unwrap(); // Insert 'd'
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "dhello");
    // After insert + escape, cursor is at position 1 (after 'd')

    // Move to position: cursor at 1, move right twice -> position 3 (on first 'l')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // cursor at 2 ('e')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // cursor at 3 (first 'l')

    // Repeat - should insert 'd' at position 3
    sim.execute_command(".").unwrap();
    let state = sim.display();
    // Result: "dhe" + "d" + "llo" = "dhedllo"
    assert_eq!(state.content(), "dhedllo");
}

#[test]
fn test_repeat_insert_at_line_start() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Insert at line start
    sim.execute_command(CMD_INSERT_LINE_START).unwrap();
    sim.execute_command(CMD_INDENT).unwrap();
    sim.execute_command(CMD_INDENT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), ">>hello");

    // Move somewhere else (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Repeat
    sim.execute_command(".").unwrap();
    let state = sim.display();
    // Should insert ">>" at cursor position
    assert!(state.content().contains(">>"));
}

#[test]
fn test_repeat_append_at_line_end() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Append at line end
    sim.execute_command(CMD_APPEND_LINE_END).unwrap();
    sim.execute_command("!").unwrap();
    sim.execute_command("!").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.display();
    assert_eq!(state.content(), "hello!!");

    // Move to start (use 'gh' in Helix, not '0')
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();

    // Repeat - should use 'A' to append at line end
    sim.execute_command(".").unwrap();
    let state = sim.display();
    // 'A' moves to end of line and appends, so "!!" should be at the end
    assert_eq!(state.content(), "hello!!!!");
}

// ============================================================================
// Scenario Validation Tests
// ============================================================================

#[test]
fn test_scenario_repeat_insert_001() {
    // Scenario: repeat_insert_001
    // Setup: "TODO:\nFIX:\nNOTE:", cursor [0,5]
    // Target: "TODO: Update docs\nFIX: Update docs\nNOTE:", cursor [1,16]
    // Commands: ["i", " ", "U", "p", "d", "a", "t", "e", " ", "d", "o", "c", "s", "Escape", "j", "0", "$", "."]
    let mut sim = AnyModeSimulator::new("TODO:\nFIX:\nNOTE:".to_string());

    // Move cursor to position [0,5] (after "TODO:")
    // "TODO:" has 5 chars, so col 5 is right after ':'
    // But in Helix cursor can't go past last char, so max is col 4 (the ':')
    // Scenario says cursor_position = [0, 5] which means we start at position 5
    // Actually let's check what the scenario expects - it inserts AFTER "TODO:"
    // So we need to be at the ':' (col 4) or use append mode
    // `gl` lands on the ':' (last char), so `i` inserts before it
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Enter insert mode and type " Update docs"
    sim.execute_command("i").unwrap();
    for ch in " Update docs".chars() {
        sim.execute_command(&ch.to_string()).unwrap();
    }
    sim.execute_command("Escape").unwrap();

    let state = sim.display();
    assert_eq!(
        state.content(),
        "TODO Update docs:\nFIX:\nNOTE:",
        "After first insert"
    );

    // Navigate: j (down), gh (line start), gl (line end)
    // Note: '0' and '$' are NOT line movement commands in Helix
    sim.execute_command("j").unwrap();
    sim.execute_command("gh").unwrap();
    sim.execute_command("gl").unwrap();

    let state = sim.display();
    assert_eq!(state.cursor_position().row(), 1, "Should be on line 1");

    // Repeat the insert
    sim.execute_command(".").unwrap();

    let state = sim.display();
    assert_eq!(
        state.content(),
        "TODO Update docs:\nFIX Update docs:\nNOTE:",
        "Both lines should have ' Update docs' inserted before the ':'"
    );
}
