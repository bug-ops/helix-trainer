//! Integration tests for UI state with multi-key command handling
//!
//! These tests verify that the UI layer correctly handles multi-key commands
//! like 'dd', 'gg', 'r<char>' through the command buffer mechanism.

use helix_trainer::config::{Scenario, ScoringConfig, Setup, Solution, TargetState};
use helix_trainer::ui::{AppState, Message, update};
use std::borrow::Cow;

/// Helper to create a simple test scenario
fn create_test_scenario(
    id: &str,
    setup_content: &str,
    setup_cursor: (usize, usize),
    target_content: &str,
    target_cursor: (usize, usize),
) -> Scenario {
    Scenario {
        id: id.to_string(),
        name: "Test Scenario".to_string(),
        description: "Test scenario for integration testing".to_string(),
        setup: Setup {
            file_content: setup_content.to_string(),
            cursor_position: setup_cursor,
        },
        target: TargetState {
            file_content: target_content.to_string(),
            cursor_position: target_cursor,
            selection: None,
        },
        solution: Solution {
            commands: vec!["test".to_string()],
            description: "Test solution".to_string(),
        },
        alternatives: vec![],
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: 1,
            max_points: 100,
            tolerance: 0,
        },
    }
}

#[test]
fn test_replace_command_multi_key() {
    // Test scenario: replace 'x' with 'e' in "Hxllo"
    let scenario = create_test_scenario(
        "test_replace",
        "Hxllo",
        (0, 1), // cursor on 'x'
        "Hello",
        (0, 1),
    );

    let mut state = AppState::new(vec![scenario.clone()]);

    // Start the scenario
    update(&mut state, Message::StartScenario(0)).unwrap();

    // First key: 'r' - should be stored in command buffer
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();

    // Check that command buffer contains 'r' and command hasn't executed yet
    assert_eq!(state.command_buffer, "r");
    if let Some(session) = &state.session {
        // Content should still be "Hxllo" - nothing changed yet
        assert_eq!(session.current_state().content(), "Hxllo");
    } else {
        panic!("Session should exist");
    }

    // Second key: 'e' - should complete the 'r' command
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("e"))).unwrap();

    // Command buffer should be cleared
    assert_eq!(state.command_buffer, "");

    // Content should now be "Hello"
    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "Hello");
    } else {
        panic!("Session should exist");
    }
}

#[test]
fn test_dd_command_multi_key() {
    // Test scenario: delete line
    let scenario = create_test_scenario(
        "test_dd",
        "line1\nline2\nline3",
        (1, 0), // cursor on line2
        "line1\nline3",
        (1, 0),
    );

    let mut state = AppState::new(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // First 'd' - stored in buffer
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();
    assert_eq!(state.command_buffer, "d");

    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "line1\nline2\nline3");
    }

    // Second 'd' - completes 'dd'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();
    assert_eq!(state.command_buffer, "");

    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "line1\nline3");
    }
}

#[test]
fn test_gg_command_multi_key() {
    // Test scenario: go to document start
    let scenario = create_test_scenario(
        "test_gg",
        "line1\nline2\nline3",
        (2, 0), // cursor on line3
        "line1\nline2\nline3",
        (0, 0), // cursor at start
    );

    let mut state = AppState::new(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // First 'g'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("g"))).unwrap();
    assert_eq!(state.command_buffer, "g");

    // Second 'g'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("g"))).unwrap();
    assert_eq!(state.command_buffer, "");

    if let Some(session) = &state.session {
        let cursor = session.current_state().cursor_position();
        assert_eq!((cursor.row, cursor.col), (0, 0));
    }
}

#[test]
fn test_replace_command_valid_sequence() {
    // Test that 'rr' is valid - replace character with 'r'
    let scenario = create_test_scenario("test", "rest", (0, 0), "rest", (0, 0));

    let mut state = AppState::new(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Press 'r'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();
    assert_eq!(state.command_buffer, "r");

    // Press 'r' again - this completes 'rr' (replace 't' with 'r')
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();

    // Buffer should be cleared after command executes
    assert_eq!(state.command_buffer, "");

    // Content should be "rest" (replaced 't' with 'r')
    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "rest");
    }
}

#[test]
fn test_single_key_command_immediate_execution() {
    // Test that single-key commands execute immediately
    let scenario = create_test_scenario(
        "test_single",
        "hello",
        (0, 0),
        "hello",
        (0, 1), // moved right
    );

    let mut state = AppState::new(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Press 'l' (move right) - should execute immediately
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("l"))).unwrap();

    // Buffer should be empty (command executed)
    assert_eq!(state.command_buffer, "");

    // Cursor should have moved
    if let Some(session) = &state.session {
        assert_eq!(session.current_state().cursor_position().col, 1);
    }
}

#[test]
fn test_replace_with_special_chars() {
    // Test replacing with various characters
    let scenario = create_test_scenario("test_special", "x", (0, 0), "!", (0, 0));

    let mut state = AppState::new(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Replace with '!'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("!"))).unwrap();

    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "!");
    }
}

#[test]
fn test_undo_integration() {
    // Test undo functionality (redo not yet implemented)
    let scenario =
        create_test_scenario("test_undo", "line1\nline2", (0, 0), "line1\nline2", (0, 0));

    let mut state = AppState::new(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Delete line: dd
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();

    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "line2");
    }

    // Undo: u
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("u"))).unwrap();

    if let Some(session) = &state.session {
        assert_eq!(session.current_state().content(), "line1\nline2");
    }

    // Note: Redo functionality (ctrl-r, U) is not yet implemented in HelixSimulator
    // The redo() method is currently a placeholder
}
