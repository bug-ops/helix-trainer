//! Integration tests for UI state with multi-key command handling
//!
//! These tests verify that the UI layer correctly handles multi-key commands
//! like 'dd', 'gg', 'r<char>' through the typestate-based InputStateMachine.

use helix_trainer::config::{CursorSpec, Scenario, ScoringConfig, Setup, Solution, TargetState};
use helix_trainer::game::PlayableScenario;
use helix_trainer::gamification::{ProfileStorage, UserProfile};
use helix_trainer::input::keymap::CanonicalKeys;
use helix_trainer::learning::PerformanceTracker;
use helix_trainer::ui::state::{InputStateAccess, TypedScreen};
use helix_trainer::ui::{AppState, Message, update};
use std::borrow::Cow;

/// Build an `ExecuteCommand` message for a single physical key with no
/// keymap remap active - `keys` and `typed` are identical, matching every
/// keystroke these integration tests simulate.
fn exec(key: &'static str) -> Message {
    Message::ExecuteCommand {
        keys: CanonicalKeys::from_static(key),
        typed: Cow::Borrowed(key),
    }
}

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
            cursor: CursorSpec {
                cursor_position: Some(setup_cursor),
                selection: None,
                cursors: None,
                selections: None,
            },
        },
        target: TargetState {
            file_content: target_content.to_string(),
            cursor: CursorSpec {
                cursor_position: Some(target_cursor),
                selection: None,
                cursors: None,
                selections: None,
            },
        },
        solution: Solution {
            commands: vec!["test".to_string()],
            description: "Test solution".to_string(),
        },
        alternatives: vec![],
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
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

    // First key: 'r' - should transition to ReplaceCharPending state
    update(&mut state, exec("r")).unwrap();

    // Check that input state is ReplaceCharPending and command hasn't executed yet
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(
            task_data.input_state().state().is_replace_char_pending(),
            "Expected ReplaceCharPending state after 'r', got {:?}",
            task_data.input_state().state()
        );
        // Content should still be "Hxllo" - nothing changed yet
        assert_eq!(task_data.session.current_content(), "Hxllo");
    } else {
        panic!("Should be on Task screen");
    }

    // Second key: 'e' - should complete the 'r' command
    update(&mut state, exec("e")).unwrap();

    // After typestate refactoring and success popup changes:
    // - Session completes when target is reached
    // - We stay on Task screen with completion_time set
    // - pending_completed_session contains the completed session
    // - After delay, CompleteScenario transitions to Results
    match &state.screen {
        TypedScreen::Task(task_data) => {
            // Input state should be back to Base after command execution
            assert!(
                task_data.input_state().state().is_base(),
                "Expected Base state after command, got {:?}",
                task_data.input_state().state()
            );
            // Check completion via pending session or completion_time
            if state.ui.completion_time.is_some() {
                // Scenario completed - check pending session has correct content
                let pending = state.game.pending_completed_session.as_ref();
                assert!(pending.is_some(), "Should have pending completed session");
                assert_eq!(pending.unwrap().current_content(), "Hello");
            } else {
                // Scenario not yet complete - check session content
                assert_eq!(task_data.session.current_content(), "Hello");
            }
        }
        TypedScreen::Results(results_data) => {
            // Session completed and transitioned to Results
            assert!(results_data.feedback.success);
        }
        _ => panic!("Should be on Task or Results screen"),
    }
}

#[test]
fn test_xd_command_line_delete() {
    // Test scenario: delete line using Helix idiom 'xd' (select line + delete selection)
    // Target is different so xd won't auto-complete
    let scenario = create_test_scenario(
        "test_xd",
        "line1\nline2\nline3",
        (1, 0),                // cursor on line2
        "line1\nline3\nextra", // Different target - won't auto-complete on xd
        (1, 0),
    );

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // 'x' - selects current line (executes immediately)
    update(&mut state, exec("x")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        // Input state should be Base (single-key command executes immediately)
        assert!(task_data.input_state().state().is_base());
        // Content unchanged after x (selection only)
        assert_eq!(task_data.session.current_content(), "line1\nline2\nline3");
        // Selection should be set
        assert!(
            task_data.session.current_selection().is_some(),
            "Selection should be set after 'x'"
        );
    } else {
        panic!("Should be on Task screen");
    }

    // 'd' - deletes the selection (executes immediately)
    update(&mut state, exec("d")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        // Input state should be Base
        assert!(task_data.input_state().state().is_base());
        assert_eq!(task_data.session.current_content(), "line1\nline3");
    } else {
        panic!("Should be on Task screen after xd (scenario incomplete)");
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

    // First 'g' - should transition to GotoPending state
    update(&mut state, exec("g")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(
            task_data.input_state().state().is_goto_pending(),
            "Expected GotoPending state after 'g', got {:?}",
            task_data.input_state().state()
        );
    } else {
        panic!("Should be on Task screen");
    }

    // Second 'g' - executes 'gg' command but doesn't complete scenario
    update(&mut state, exec("g")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        // Input state should be back to Base
        assert!(task_data.input_state().state().is_base());
        let cursor = task_data.session.current_cursor();
        assert_eq!(cursor, (0, 0));
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

    // Press 'r' - should transition to ReplaceCharPending state
    update(&mut state, exec("r")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(
            task_data.input_state().state().is_replace_char_pending(),
            "Expected ReplaceCharPending state after 'r', got {:?}",
            task_data.input_state().state()
        );
    } else {
        panic!("Should be on Task screen");
    }

    // Press 'r' again - this completes 'rr' (replace 't' with 'r')
    update(&mut state, exec("r")).unwrap();

    // Input state should be back to Base after command executes
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        // Content should be "rest" (replaced 't' with 'r')
        assert_eq!(task_data.session.current_content(), "rest");
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
    update(&mut state, exec("l")).unwrap();

    // Input state should be Base (single-key command executes immediately)
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        // Cursor should have moved
        assert_eq!(task_data.session.current_cursor().1, 1);
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
    update(&mut state, exec("r")).unwrap();
    update(&mut state, exec("!")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.current_content(), "!");
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

    // Delete line using Helix idiom: xd (select line + delete selection)
    update(&mut state, exec("x")).unwrap();
    update(&mut state, exec("d")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.current_content(), "line2");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }

    // Undo: u (undoes the delete, but note: undo is per-transaction so may need multiple u)
    update(&mut state, exec("u")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        // After undo, line1 should be restored
        assert_eq!(task_data.session.current_content(), "line1\nline2");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }

    // Note: Redo functionality (ctrl-r, U) is not yet implemented in HelixSimulator
    // The redo() method is currently a placeholder
}

#[test]
fn test_register_yank_paste_multi_key() {
    // "ay yanks into register a; a later plain 'y' overwrites only the
    // default register; "ap then proves register a survived independently.
    let scenario =
        create_test_scenario("test_register", "alpha beta", (0, 0), "aalpha beta", (0, 1));

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    // '"' - transitions to RegisterPending
    update(&mut state, exec("\"")).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(matches!(
            task_data.input_state().state(),
            helix_trainer::input::typestate::InputState::RegisterPending
        ));
    } else {
        panic!("Should be on Task screen");
    }

    // 'a' - selects register a, transitions to RegisterOpPending
    update(&mut state, exec("a")).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(matches!(
            task_data.input_state().state(),
            helix_trainer::input::typestate::InputState::RegisterOpPending { register: 'a' }
        ));
        // Nothing executed yet
        assert_eq!(task_data.session.current_content(), "alpha beta");
    } else {
        panic!("Should be on Task screen");
    }

    // 'y' - completes "ay, yanking the char under cursor ('a') into register a
    update(&mut state, exec("y")).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        assert_eq!(task_data.session.current_content(), "alpha beta");
    } else {
        panic!("Should be on Task screen (scenario incomplete)");
    }

    // Overwrite the default register - must not disturb register a
    update(&mut state, exec("y")).unwrap();

    // '"' -> 'a' -> 'p' pastes register a's content ('a') after the cursor
    update(&mut state, exec("\"")).unwrap();
    update(&mut state, exec("a")).unwrap();
    update(&mut state, exec("p")).unwrap();

    if state.ui.completion_time.is_some() {
        let pending = state.game.pending_completed_session.as_ref();
        assert!(pending.is_some(), "Should have pending completed session");
        assert_eq!(pending.unwrap().current_content(), "aalpha beta");
    } else if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        assert_eq!(task_data.session.current_content(), "aalpha beta");
    } else {
        panic!("Should be on Task screen (or completed) after the register sequence");
    }
}

#[test]
fn test_register_op_cancels_on_out_of_scope_operator() {
    // Register scope is limited to y/p/P/R; any other operator cancels back
    // to Base rather than executing a bare command.
    let scenario = create_test_scenario("test_register_cancel", "hello", (0, 0), "world", (0, 0));

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    update(&mut state, exec("\"")).unwrap();
    update(&mut state, exec("a")).unwrap();
    // 'd' is not register-scoped - should cancel, not delete anything
    update(&mut state, exec("d")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        assert_eq!(task_data.session.current_content(), "hello");
    } else {
        panic!("Should be on Task screen");
    }
}

#[test]
fn test_command_line_goto_multi_key() {
    // ':' -> "goto 3" -> Enter assembles and executes ":goto 3" atomically.
    let scenario = create_test_scenario(
        "test_command_line",
        "line1\nline2\nline3\nline4",
        (0, 0),
        "line1\nline2\nline3\nline4",
        (2, 0),
    );

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    update(&mut state, exec(":")).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.input_state().pending_command_line(), Some(""));
    } else {
        panic!("Should be on Task screen");
    }

    for c in "goto 3".chars() {
        let owned = c.to_string();
        update(
            &mut state,
            Message::ExecuteCommand {
                keys: CanonicalKeys::from_owned(owned.clone()),
                typed: Cow::Owned(owned),
            },
        )
        .unwrap();
    }
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(
            task_data.input_state().pending_command_line(),
            Some("goto 3")
        );
        // Nothing executed yet - the buffer only assembles until Enter
        assert_eq!(task_data.session.current_cursor(), (0, 0));
    } else {
        panic!("Should be on Task screen");
    }

    update(&mut state, exec("Enter")).unwrap();

    // ':goto 3' reaches the target cursor exactly, so the scenario completes
    // immediately; the completed session lives in `pending_completed_session`
    // while the screen stays on Task for the success popup (see
    // `process_session_result`), matching the pattern other multi-key tests
    // in this file use to check post-completion state.
    if state.ui.completion_time.is_some() {
        let pending = state.game.pending_completed_session.as_ref();
        assert!(pending.is_some(), "Should have pending completed session");
        assert_eq!(pending.unwrap().current_cursor(), (2, 0));
    } else if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        assert_eq!(task_data.session.current_cursor(), (2, 0));
    } else {
        panic!("Should be on Task screen (or completed) after ':goto 3'");
    }
}

#[test]
fn test_command_line_escape_cancels_buffer() {
    let scenario = create_test_scenario("test_cmdline_cancel", "hello", (0, 0), "world", (0, 0));

    let mut state = create_test_app_state(vec![scenario.clone()]);
    update(&mut state, Message::StartScenario(0)).unwrap();

    update(&mut state, exec(":")).unwrap();
    update(&mut state, exec("g")).unwrap();
    update(&mut state, exec("Escape")).unwrap();

    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.input_state().state().is_base());
        assert_eq!(task_data.session.current_content(), "hello");
    } else {
        panic!("Should be on Task screen");
    }
}
