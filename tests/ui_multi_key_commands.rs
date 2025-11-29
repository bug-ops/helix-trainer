//! Integration tests for UI state with multi-key command handling
//!
//! These tests verify that the UI layer correctly handles multi-key commands
//! like 'dd', 'gg', 'r<char>' through the command buffer mechanism.

use helix_trainer::config::{Scenario, ScoringConfig, Setup, Solution, TargetState};
use helix_trainer::gamification::{ProfileStorage, UserProfile};
use helix_trainer::learning::PerformanceTracker;
use helix_trainer::ui::state::TypedScreen;
use helix_trainer::ui::{AppState, Message, update};
use std::borrow::Cow;

/// Helper to create AppState for testing
fn create_test_app_state(scenarios: Vec<Scenario>) -> AppState {
    let profile = UserProfile::new();
    let storage = ProfileStorage::new();
    let tracker = PerformanceTracker::new();
    AppState::new(scenarios, profile, storage, tracker)
}

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
        metadata: None,
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

    let mut state = create_test_app_state(vec![scenario.clone()]);

    // Start the scenario
    update(&mut state, Message::StartScenario(0)).unwrap();

    // First key: 'r' - should be stored in command buffer
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();

    // Check that command buffer contains 'r' and command hasn't executed yet
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "r");
        // Content should still be "Hxllo" - nothing changed yet
        assert_eq!(task_data.session.current_state().content(), "Hxllo");
    } else {
        panic!("Should be on Task screen");
    }

    // Second key: 'e' - should complete the 'r' command
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("e"))).unwrap();

    // After typestate refactoring, the session completes when target is reached
    // Check screen state and verify completion
    match &state.screen {
        TypedScreen::Task(task_data) => {
            // Session still active (if target was different)
            assert_eq!(task_data.command_buffer, "");
            assert_eq!(task_data.session.current_state().content(), "Hello");
        }
        TypedScreen::Results(results_data) => {
            // Session completed!
            assert!(results_data.feedback.success);
        }
        _ => panic!("Should be on Task or Results screen"),
    }
}

#[test]
fn test_dd_command_multi_key() {
    // Test scenario: delete line (target is different so dd won't auto-complete)
    let scenario = create_test_scenario(
        "test_dd",
        "line1\nline2\nline3",
        (1, 0),                // cursor on line2
        "line1\nline3\nextra", // Different target - won't auto-complete on dd
        (1, 0),
    );

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // First 'd' - stored in buffer
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "d");
        assert_eq!(
            task_data.session.current_state().content(),
            "line1\nline2\nline3"
        );
    } else {
        panic!("Should be on Task screen");
    }

    // Second 'd' - completes 'dd' command but doesn't complete scenario
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "");
        assert_eq!(task_data.session.current_state().content(), "line1\nline3");
    } else {
        panic!("Should be on Task screen after dd (scenario incomplete)");
    }
}

#[test]
fn test_gg_command_multi_key() {
    // Test scenario: go to document start (target cursor different to avoid auto-complete)
    let scenario = create_test_scenario(
        "test_gg",
        "line1\nline2\nline3",
        (2, 0), // cursor on line3
        "line1\nline2\nline3",
        (0, 2), // cursor at different position - won't auto-complete on gg
    );

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // First 'g'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("g"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "g");
    } else {
        panic!("Should be on Task screen");
    }

    // Second 'g' - executes command but doesn't complete scenario
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("g"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "");
        let cursor = task_data.session.current_state().cursor_position();
        assert_eq!((cursor.row, cursor.col), (0, 0));
    } else {
        panic!("Should be on Task screen after gg (scenario incomplete)");
    }
}

#[test]
fn test_replace_command_valid_sequence() {
    // Test that 'rr' is valid - replace character with 'r'
    // Target is different to avoid auto-complete
    let scenario = create_test_scenario("test", "test", (0, 0), "best", (0, 0));

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Press 'r'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "r");
    } else {
        panic!("Should be on Task screen");
    }

    // Press 'r' again - this completes 'rr' (replace 't' with 'r')
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();

    // Buffer should be cleared after command executes
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "");
        // Content should be "rest" (replaced 't' with 'r')
        assert_eq!(task_data.session.current_state().content(), "rest");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }
}

#[test]
fn test_single_key_command_immediate_execution() {
    // Test that single-key commands execute immediately
    // Target is different to avoid auto-complete
    let scenario = create_test_scenario(
        "test_single",
        "hello",
        (0, 0),
        "hello",
        (0, 2), // Different target position - won't auto-complete on 'l'
    );

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Press 'l' (move right) - should execute immediately
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("l"))).unwrap();

    // Buffer should be empty (command executed)
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.command_buffer, "");
        // Cursor should have moved
        assert_eq!(task_data.session.current_state().cursor_position().col, 1);
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }
}

#[test]
fn test_replace_with_special_chars() {
    // Test replacing with various characters
    // Target is different to avoid auto-complete
    let scenario = create_test_scenario("test_special", "x", (0, 0), "!!", (0, 0));

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Replace with '!'
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("r"))).unwrap();
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("!"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.current_state().content(), "!");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }
}

#[test]
fn test_undo_integration() {
    // Test undo functionality (redo not yet implemented)
    // Target is different to avoid auto-complete
    let scenario = create_test_scenario(
        "test_undo",
        "line1\nline2",
        (0, 0),
        "line1\nline2\nline3",
        (0, 0),
    );

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Delete line: dd
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("d"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.current_state().content(), "line2");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }

    // Undo: u
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("u"))).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.current_state().content(), "line1\nline2");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }

    // Note: Redo functionality (ctrl-r, U) is not yet implemented in HelixSimulator
    // The redo() method is currently a placeholder
}
