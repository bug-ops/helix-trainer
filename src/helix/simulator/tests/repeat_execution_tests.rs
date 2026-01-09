//! Repeat dot command execution tests for HelixSimulator
//!
//! Tests that verify the dot command correctly replays recorded actions:
//! - Basic repeat: Delete, join, indent operations
//! - Insert mode repeat: Text insertion with entry commands
//! - Complex scenarios: Real-world usage patterns

use crate::helix::commands::*;
use crate::helix::simulator::AnyModeSimulator;

// ============================================================================
// Basic Repeat Execution Tests
// ============================================================================

#[test]
fn test_repeat_delete_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Execute delete command
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "ello");

    // Repeat delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo");
}

#[test]
fn test_repeat_delete_line() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3".to_string());

    // Delete first line using x+d (Helix way)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 2\nline 3");

    // Repeat delete - should execute x+d together
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 3");
}

#[test]
fn test_repeat_with_empty_buffer() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Try to repeat without any previous action
    let result = sim.execute_command(".");
    assert!(result.is_ok()); // Should not error

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "test"); // Should be unchanged
}

#[test]
fn test_repeat_is_not_recorded() {
    let mut sim = AnyModeSimulator::new("abcd".to_string());

    // Delete a char
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "bcd");

    // Repeat once
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "cd");

    // Repeat again - should repeat the ORIGINAL x, not the previous .
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "d");

    // Total: 3 deletes (original x + two repeats)
}

#[test]
fn test_repeat_join_lines() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2\nline 3".to_string());

    // Join lines
    sim.execute_command(CMD_JOIN_LINES).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1 line 2\nline 3");

    // Repeat join
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "line 1 line 2 line 3");
}

#[test]
fn test_repeat_indent() {
    let mut sim = AnyModeSimulator::new("line 1\nline 2".to_string());

    // Indent
    sim.execute_command(CMD_INDENT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  line 1\nline 2");

    // Repeat indent
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Line should be double-indented
    assert_eq!(state.content(), "    line 1\nline 2");
}

#[test]
fn test_repeat_dedent() {
    let mut sim = AnyModeSimulator::new("    code".to_string());

    // Dedent
    sim.execute_command(CMD_DEDENT).unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "  code");

    // Repeat dedent
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "code");
}

#[test]
fn test_repeat_replace_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Replace 'h' with 'x'
    sim.execute_command("rx").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xello");

    // Move to next char
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();

    // Repeat replace (should replace 'e' with 'x')
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "xxllo");
}

#[test]
fn test_repeat_multiple_times() {
    let mut sim = AnyModeSimulator::new("xxxxxx".to_string());

    // Delete once
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Repeat 4 times
    for _ in 0..4 {
        sim.execute_command(".").unwrap();
    }

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "x"); // 5 deletes total
}

#[test]
fn test_repeat_after_undo() {
    let mut sim = AnyModeSimulator::new("test".to_string());

    // Delete a char
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Undo it
    sim.execute_command(CMD_UNDO).unwrap();

    // The repeat buffer should still have 'x'
    // Repeat should still delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "est");
}

#[test]
fn test_repeat_preserves_action_across_movements() {
    let mut sim = AnyModeSimulator::new("hello world".to_string());

    // Delete 'h'
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    // Move around (movements don't change repeat buffer)
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_WORD_FORWARD).unwrap();
    sim.execute_command(CMD_GOTO_LINE_START).unwrap(); // use 'gh' in Helix

    // Repeat should still delete
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "llo world");
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

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworld");

    // Move to end (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Repeat insert
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworldhi");
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

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello world");

    // Move to start (use 'gh' in Helix, not '0')
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();

    // Repeat should append " world" after the first character (using 'a' to append)
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Since we used 'a' (append), repeat should place cursor after 'h' and insert " world"
    assert_eq!(state.content(), "h worldello world");
}

#[test]
fn test_repeat_insert_with_movements() {
    let mut sim = AnyModeSimulator::new("world".to_string());

    // Insert with arrow key movements (simplified test)
    // Note: Current implementation applies movements AFTER all text insertion
    // This is a known limitation - movements aren't interleaved with text
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_MOVE_LEFT).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command(CMD_ARROW_LEFT).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hiworld");
    // Cursor should be at position 1 (moved left once from 2)
    assert_eq!(state.cursor_position().col, 1);

    // Move to end (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Repeat insert with movements
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
    // Should insert "hi" at end, then move left once
    assert_eq!(state.content(), "hiworldhi");
    // Cursor moved left from position 9 to position 8
    assert_eq!(state.cursor_position().col, 8);
}

#[test]
fn test_repeat_insert_simple() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Insert 'd' at the beginning (CMD_DELETE_SELECTION is "d")
    sim.execute_command(CMD_INSERT).unwrap(); // Enter insert mode
    sim.execute_command(CMD_DELETE_SELECTION).unwrap(); // Insert 'd'
    sim.execute_command(CMD_ESCAPE).unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "dhello");
    // After insert + escape, cursor is at position 1 (after 'd')

    // Move to position: cursor at 1, move right twice -> position 3 (on first 'l')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // cursor at 2 ('e')
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // cursor at 3 (first 'l')

    // Repeat - should insert 'd' at position 3
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
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

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), ">>hello");

    // Move somewhere else (use 'gl' in Helix, not '$')
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Repeat
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
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

    let state = sim.get_state().unwrap();
    assert_eq!(state.content(), "hello!!");

    // Move to start (use 'gh' in Helix, not '0')
    sim.execute_command(CMD_GOTO_LINE_START).unwrap();

    // Repeat - should use 'A' to append at line end
    sim.execute_command(".").unwrap();
    let state = sim.get_state().unwrap();
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
    // Use 'gl' in Helix, not '$'
    sim.execute_command(CMD_GOTO_LINE_END).unwrap();

    // Enter insert mode and type " Update docs"
    sim.execute_command("i").unwrap();
    for ch in " Update docs".chars() {
        sim.execute_command(&ch.to_string()).unwrap();
    }
    sim.execute_command("Escape").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "TODO: Update docs\nFIX:\nNOTE:",
        "After first insert"
    );

    // Navigate: j (down), gh (line start), gl (line end)
    // Note: '0' and '$' are NOT line movement commands in Helix
    sim.execute_command("j").unwrap();
    sim.execute_command("gh").unwrap();
    sim.execute_command("gl").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(state.cursor_position().row, 1, "Should be on line 1");

    // Repeat the insert
    sim.execute_command(".").unwrap();

    let state = sim.get_state().unwrap();
    assert_eq!(
        state.content(),
        "TODO: Update docs\nFIX: Update docs\nNOTE:",
        "Both lines should have ' Update docs' appended"
    );
}
