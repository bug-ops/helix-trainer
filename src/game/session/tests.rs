//! Tests for GameSession

use super::*;
use crate::config::{ScoringConfig, Setup, Solution, TargetState};

fn create_test_scenario() -> Scenario {
    Scenario {
        id: "test_001".to_string(),
        name: "Test Scenario".to_string(),
        description: "A test scenario".to_string(),
        setup: Setup {
            file_content: "line 1\nline 2\nline 3\n".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        target: TargetState {
            file_content: "line 2\nline 3\n".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        solution: Solution {
            commands: vec!["x".to_string(), "d".to_string()],
            description: "Delete first line".to_string(),
        },
        alternatives: vec![],
        hints: vec!["Use x to select line, then d to delete".to_string()],
        scoring: ScoringConfig {
            optimal_count: 2,
            max_points: 100,
            tolerance: 0,
        },
        metadata: None,
    }
}

#[test]
fn test_session_creation() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario);
    assert!(session.is_ok());
}

#[test]
fn test_initial_state() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    assert_eq!(session.action_count(), 0);
    // Session type is GameSession<Active> at compile time - no runtime check needed
}

#[test]
fn test_record_action() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    let result = session.record_action("j".to_string()).unwrap();
    match result {
        SessionAfterAction::StillActive(session) => {
            assert_eq!(session.action_count(), 1);
        }
        SessionAfterAction::Completed(_) => {
            panic!("Session should not be completed after single 'j' action");
        }
    }
}

#[test]
fn test_check_completion() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Initially not completed
    assert!(!session.check_completion());
}

#[test]
fn test_completion_detection() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Execute 'x' to select line, then 'd' to delete - this should complete the scenario
    let result = session.record_action("x".to_string()).unwrap();
    let session = match result {
        SessionAfterAction::StillActive(s) => s,
        SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
    };
    let result = session.record_action("d".to_string()).unwrap();

    // Should be completed now
    assert!(
        matches!(result, SessionAfterAction::Completed(_)),
        "Session should be completed after 'x' + 'd' commands"
    );
}

#[test]
fn test_get_hint() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    let hint = session.get_hint();
    assert!(hint.is_some());
    assert_eq!(hint.unwrap(), "Use x to select line, then d to delete");

    // Second call should return None (only one hint)
    let hint2 = session.get_hint();
    assert!(hint2.is_none());
}

#[test]
fn test_abandon_session() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    let abandoned = session.abandon();
    // Type is now GameSession<Abandoned> - no runtime checks needed
    // Get feedback to verify it works
    let feedback = abandoned.feedback();
    assert!(!feedback.success);
    assert_eq!(feedback.score, 0);
}

#[test]
fn test_calculate_score_perfect() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Record optimal number of actions (x to select line, d to delete)
    let result = session.record_action("x".to_string()).unwrap();
    let session = match result {
        SessionAfterAction::StillActive(s) => s,
        SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
    };
    let result = session.record_action("d".to_string()).unwrap();

    // The session should automatically mark as completed when state matches
    match result {
        SessionAfterAction::Completed(completed) => {
            let score = completed.score().unwrap();
            assert_eq!(score, 100); // Perfect score (2 actions, optimal is 2)
        }
        SessionAfterAction::StillActive(_) => {
            panic!("Session should be completed after 'x' + 'd' commands");
        }
    }
}

#[test]
fn test_calculate_score_incomplete() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Session is active, not completed - abandon to get feedback with score 0
    let abandoned = session.abandon();
    let feedback = abandoned.feedback();
    assert_eq!(feedback.score, 0);
    assert!(!feedback.success);
}

#[test]
fn test_get_feedback_success() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Complete with x + d commands
    let result = session.record_action("x".to_string()).unwrap();
    let session = match result {
        SessionAfterAction::StillActive(s) => s,
        SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
    };
    let result = session.record_action("d".to_string()).unwrap();

    // Should be automatically completed
    match result {
        SessionAfterAction::Completed(completed) => {
            let feedback = completed.feedback().unwrap();
            assert!(feedback.success);
            assert_eq!(feedback.score, 100);
            assert_eq!(feedback.rating, PerformanceRating::Perfect);
            assert!(feedback.is_optimal);
        }
        SessionAfterAction::StillActive(_) => {
            panic!("Session should be completed after 'x' + 'd' commands");
        }
    }
}

#[test]
fn test_get_feedback_with_hint() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Take many actions (>2x optimal)
    // Optimal is 2 (x + d commands), so we need >4 actions
    // Use j/k movements that will return cursor to (0,0)
    let mut session_or_completed = SessionAfterAction::StillActive(session);

    // Move down twice, then up twice (4 actions, returns to start)
    let movements = ["j", "j", "k", "k", "j"];
    for cmd in movements {
        session_or_completed = match session_or_completed {
            SessionAfterAction::StillActive(s) => s.record_action(cmd.to_string()).unwrap(),
            SessionAfterAction::Completed(c) => {
                // Already completed early, just return
                SessionAfterAction::Completed(c)
            }
        };

        // If completed early, stop trying
        if matches!(session_or_completed, SessionAfterAction::Completed(_)) {
            break;
        }
    }

    // Move back up to row 0 before completing
    session_or_completed = match session_or_completed {
        SessionAfterAction::StillActive(s) => s.record_action("k".to_string()).unwrap(),
        SessionAfterAction::Completed(c) => SessionAfterAction::Completed(c),
    };

    // Complete with x + d commands (if not already completed)
    // First select line with x
    session_or_completed = match session_or_completed {
        SessionAfterAction::StillActive(s) => s.record_action("x".to_string()).unwrap(),
        SessionAfterAction::Completed(c) => SessionAfterAction::Completed(c),
    };

    // Then delete with d
    let result = match session_or_completed {
        SessionAfterAction::StillActive(s) => s.record_action("d".to_string()).unwrap(),
        SessionAfterAction::Completed(c) => SessionAfterAction::Completed(c),
    };

    match result {
        SessionAfterAction::Completed(completed) => {
            let feedback = completed.feedback().unwrap();
            assert!(feedback.success);
            assert!(feedback.hint.is_some()); // Should provide hint
            assert!(!feedback.is_optimal);
        }
        SessionAfterAction::StillActive(_) => {
            panic!("Session should be completed after 'x' + 'd' commands");
        }
    }
}

#[test]
fn test_reset_session() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Record some actions
    let result1 = session.record_action("j".to_string()).unwrap();
    let mut current_session = match result1 {
        SessionAfterAction::StillActive(s) => s,
        SessionAfterAction::Completed(_) => panic!("Should not complete on 'j'"),
    };

    let result2 = current_session.record_action("k".to_string()).unwrap();
    current_session = match result2 {
        SessionAfterAction::StillActive(s) => s,
        SessionAfterAction::Completed(_) => panic!("Should not complete on 'k'"),
    };

    // Reset
    current_session.reset().unwrap();

    assert_eq!(current_session.action_count(), 0);
    // Type is still GameSession<Active> after reset
}

#[test]
fn test_elapsed_time() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let elapsed = session.elapsed();
    assert!(elapsed.as_millis() >= 10);
}

#[test]
fn test_feedback_summary() {
    let feedback = Feedback {
        scenario_id: "test_001".to_string(),
        success: true,
        score: 100,
        max_points: 100,
        rating: PerformanceRating::Perfect,
        actions_taken: 2,
        optimal_actions: 2,
        duration: Duration::from_secs(5),
        hint: None,
        is_optimal: true,
        user_actions: vec![],
    };

    let summary = feedback.summary();
    assert!(summary.contains("100/100"));
    assert!(summary.contains("2 actions"));
}

#[test]
fn test_timer_fixed_on_completion() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Complete with x + d commands
    let result = session.record_action("x".to_string()).unwrap();
    let session = match result {
        SessionAfterAction::StillActive(s) => s,
        SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
    };
    let result = session.record_action("d".to_string()).unwrap();

    match result {
        SessionAfterAction::Completed(completed) => {
            // Get feedback immediately after completion
            let feedback1 = completed.feedback().unwrap();
            let duration1 = feedback1.duration;

            // Wait a bit
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Get feedback again - duration should be the same (fixed)
            let feedback2 = completed.feedback().unwrap();
            let duration2 = feedback2.duration;

            // Durations should be equal (or very close)
            let diff = duration2.abs_diff(duration1);
            assert!(
                diff.as_millis() < 5,
                "Timer should be fixed after completion"
            );
        }
        SessionAfterAction::StillActive(_) => {
            panic!("Session should be completed after 'x' + 'd' commands");
        }
    }
}
