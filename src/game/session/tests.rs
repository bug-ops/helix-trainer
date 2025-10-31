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
        },
        target: TargetState {
            file_content: "line 2\nline 3\n".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        solution: Solution {
            commands: vec!["d".to_string(), "d".to_string()],
            description: "Delete first line".to_string(),
        },
        alternatives: vec![],
        hints: vec!["Use dd to delete a line".to_string()],
        scoring: ScoringConfig {
            optimal_count: 2,
            max_points: 100,
            tolerance: 0,
        },
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
    assert_eq!(session.state(), SessionState::Active);
    assert!(session.is_active());
    assert!(!session.is_completed());
}

#[test]
fn test_record_action() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    session.record_action("j".to_string()).unwrap();
    assert_eq!(session.action_count(), 1);
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
    let mut session = GameSession::new(scenario).unwrap();

    // Update to target state
    let target = session.target_state().clone();
    session.update_state(target).unwrap();

    // Should be completed now
    assert!(session.is_completed());
    assert_eq!(session.state(), SessionState::Completed);
}

#[test]
fn test_get_hint() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    let hint = session.get_hint();
    assert!(hint.is_some());
    assert_eq!(hint.unwrap(), "Use dd to delete a line");

    // Second call should return None (only one hint)
    let hint2 = session.get_hint();
    assert!(hint2.is_none());
}

#[test]
fn test_abandon_session() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    session.abandon();
    assert_eq!(session.state(), SessionState::Abandoned);
    assert!(!session.is_active());
}

#[test]
fn test_calculate_score_perfect() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    // Record optimal number of actions (execute dd to delete first line)
    session.record_action("dd".to_string()).unwrap();

    // The session should automatically mark as completed when state matches
    assert!(session.is_completed());

    let score = session.calculate_score().unwrap();
    assert_eq!(score, 100); // Perfect score (1 action, optimal is 2, tolerance is 0)
}

#[test]
fn test_calculate_score_incomplete() {
    let scenario = create_test_scenario();
    let session = GameSession::new(scenario).unwrap();

    // Session not completed
    let score = session.calculate_score().unwrap();
    assert_eq!(score, 0);
}

#[test]
fn test_get_feedback_success() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    // Complete with dd command
    session.record_action("dd".to_string()).unwrap();

    // Should be automatically completed
    assert!(session.is_completed());

    let feedback = session.get_feedback().unwrap();
    assert!(feedback.success);
    assert_eq!(feedback.score, 100);
    assert_eq!(feedback.rating, PerformanceRating::Perfect);
    assert!(feedback.is_optimal);
}

#[test]
fn test_get_feedback_with_hint() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    // Take many actions (>2x optimal)
    for _ in 0..6 {
        session.record_action("x".to_string()).unwrap();
    }

    // Complete
    let target = session.target_state().clone();
    session.update_state(target).unwrap();

    let feedback = session.get_feedback().unwrap();
    assert!(feedback.success);
    assert!(feedback.hint.is_some()); // Should provide hint
    assert!(!feedback.is_optimal);
}

#[test]
fn test_reset_session() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    // Record some actions
    session.record_action("j".to_string()).unwrap();
    session.record_action("k".to_string()).unwrap();

    // Reset
    session.reset().unwrap();

    assert_eq!(session.action_count(), 0);
    assert_eq!(session.state(), SessionState::Active);
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
        success: true,
        score: 100,
        max_points: 100,
        rating: PerformanceRating::Perfect,
        actions_taken: 2,
        optimal_actions: 2,
        duration: Duration::from_secs(5),
        hint: None,
        is_optimal: true,
    };

    let summary = feedback.summary();
    assert!(summary.contains("100/100"));
    assert!(summary.contains("2 actions"));
}

#[test]
fn test_timer_fixed_on_completion() {
    let scenario = create_test_scenario();
    let mut session = GameSession::new(scenario).unwrap();

    // Complete with dd command
    session.record_action("dd".to_string()).unwrap();

    // Get feedback immediately after completion
    let feedback1 = session.get_feedback().unwrap();
    let duration1 = feedback1.duration;

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Get feedback again - duration should be the same (fixed)
    let feedback2 = session.get_feedback().unwrap();
    let duration2 = feedback2.duration;

    // Durations should be equal (or very close)
    let diff = duration2.abs_diff(duration1);
    assert!(
        diff.as_millis() < 5,
        "Timer should be fixed after completion"
    );
}
