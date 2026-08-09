//! Integration tests for mini-game module
//!
//! Tests the interaction between components.

use super::*;
use crate::config::{
    Difficulty, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
};
use std::sync::Arc;

fn create_simple_scenario(id: &str) -> Scenario {
    Scenario {
        id: id.to_string(),
        name: format!("Test {}", id),
        description: "Test scenario".to_string(),
        setup: Setup {
            file_content: "test".to_string(),
            cursor_position: Some((0, 0)),
            selection: None,
            cursors: None,
            selections: None,
        },
        target: TargetState {
            file_content: "".to_string(),
            cursor_position: Some((0, 0)),
            selection: None,
            cursors: None,
            selections: None,
        },
        solution: Solution {
            commands: vec!["x".to_string()],
            description: "Delete char".to_string(),
        },
        alternatives: vec![],
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
            max_points: 100,
            tolerance: 0,
        },
        metadata: Some(ScenarioMetadata {
            difficulty: Some(Difficulty::Beginner),
            ..Default::default()
        }),
    }
}

#[test]
fn test_full_session_flow() {
    let scenarios = Arc::new(vec![
        create_simple_scenario("s1"),
        create_simple_scenario("s2"),
        create_simple_scenario("s3"),
    ]);

    let mut session = MiniGameSession::new(scenarios, None);

    // Start countdown
    session.start();
    assert!(session.state().is_countdown());

    // Tick through countdown
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();
    assert!(session.state().is_playing());

    // Session should have loaded a scenario
    assert!(session.current_scenario().is_some());

    // Game should not be over
    assert!(!session.state().is_game_over());
}

#[test]
fn test_difficulty_adapts_to_performance() {
    let scenarios = Arc::new(vec![
        create_simple_scenario("s1"),
        create_simple_scenario("s2"),
        create_simple_scenario("s3"),
    ]);

    let mut session = MiniGameSession::new(scenarios, None);

    // Start and progress through countdown
    session.start();
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();

    let initial_level = session.difficulty_level();

    // Simulate multiple successes
    for _ in 0..10 {
        session.advance_to_next();
        let _ = session.complete_transition();
    }

    // Difficulty should increase
    assert!(session.difficulty_level() > initial_level);
}

#[test]
fn test_streak_increases_multiplier() {
    let mut stats = MiniGameStats::new();

    // Initially 1.0x
    assert_eq!(stats.multiplier, 1.0);

    // Build streak
    for i in 1..=6 {
        stats.increase_streak();
        if i >= 6 {
            assert_eq!(stats.multiplier, 2.0);
        }
    }
}

#[test]
fn test_game_over_when_lives_depleted() {
    let scenarios = Arc::new(vec![create_simple_scenario("s1")]);
    let mut session = MiniGameSession::new(scenarios, None);

    // Start and countdown to playing
    session.start();
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();

    // Deplete all lives
    for _ in 0..3 {
        session.handle_timeout();
    }

    assert!(session.state().is_game_over());
    assert_eq!(session.stats().lives, 0);
}

#[test]
fn test_stats_track_completions_and_failures() {
    let scenarios = Arc::new(vec![create_simple_scenario("s1")]);
    let mut session = MiniGameSession::new(scenarios, None);

    // Start and countdown to playing
    session.start();
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();

    session.advance_to_next();
    assert_eq!(session.stats().scenarios_completed, 1);

    let _ = session.complete_transition();
    session.handle_timeout();
    assert_eq!(session.stats().scenarios_failed, 1);
}

#[test]
fn test_pause_and_resume() {
    let scenarios = Arc::new(vec![create_simple_scenario("s1")]);
    let mut session = MiniGameSession::new(scenarios, None);

    // Start and countdown to playing
    session.start();
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();

    assert!(session.state().is_playing());

    // Pause
    session.pause();
    assert!(session.state().is_paused());

    // Resume
    session.resume();
    assert!(session.state().is_playing());
}

#[test]
fn test_stats_initial_values() {
    let stats = MiniGameStats::new();
    assert_eq!(stats.score, 0);
    assert_eq!(stats.lives, 3);
    assert_eq!(stats.streak, 0);
    assert_eq!(stats.best_streak, 0);
    assert_eq!(stats.scenarios_completed, 0);
    assert_eq!(stats.scenarios_failed, 0);
    assert_eq!(stats.multiplier, 1.0);
}

#[test]
fn test_stats_add_score() {
    let mut stats = MiniGameStats::new();
    stats.add_score(100);
    assert_eq!(stats.score, 100);

    stats.add_score(50);
    assert_eq!(stats.score, 150);
}

#[test]
fn test_stats_lose_life() {
    let mut stats = MiniGameStats::new();
    assert_eq!(stats.lives, 3);

    assert!(stats.lose_life()); // Still has lives
    assert_eq!(stats.lives, 2);

    assert!(stats.lose_life()); // Still has lives
    assert_eq!(stats.lives, 1);

    assert!(!stats.lose_life()); // Game over
    assert_eq!(stats.lives, 0);

    // Can't go below 0
    assert!(!stats.lose_life());
    assert_eq!(stats.lives, 0);
}

#[test]
fn test_stats_max_streak_tracking() {
    let mut stats = MiniGameStats::new();

    // Build streak
    for _ in 0..5 {
        stats.increase_streak();
    }
    assert_eq!(stats.streak, 5);
    assert_eq!(stats.best_streak, 5);

    // Reset streak (not from lose_life, using reset_streak)
    stats.reset_streak();
    assert_eq!(stats.streak, 0);
    assert_eq!(stats.best_streak, 5); // Max preserved

    // Build new streak
    for _ in 0..3 {
        stats.increase_streak();
    }
    assert_eq!(stats.streak, 3);
    assert_eq!(stats.best_streak, 5); // Max still preserved

    // Exceeding best_streak updates it
    for _ in 0..5 {
        stats.increase_streak();
    }
    assert_eq!(stats.streak, 8);
    assert_eq!(stats.best_streak, 8); // Updated
}

#[test]
fn test_difficulty_controller_initial_level() {
    let controller = DifficultyController::new();
    assert_eq!(controller.current_level(), 1);
}

#[test]
fn test_difficulty_controller_adjust_up() {
    let mut controller = DifficultyController::new();

    // Record multiple quick successes at different efficiency levels
    for _ in 0..15 {
        let point = PerformancePoint::new(true, 0.3, Difficulty::Beginner, 1.0);
        controller.update_after_scenario(point);
    }

    // Level should have increased
    assert!(controller.current_level() > 1);
}

#[test]
fn test_difficulty_controller_adjust_down() {
    let mut controller = DifficultyController::new();

    // First increase level
    for _ in 0..20 {
        let point = PerformancePoint::new(true, 0.3, Difficulty::Beginner, 1.0);
        controller.update_after_scenario(point);
    }

    let high_level = controller.current_level();

    // Now record failures
    for _ in 0..15 {
        let point = PerformancePoint::new(false, 1.0, Difficulty::Beginner, 0.2);
        controller.update_after_scenario(point);
    }

    // Level should have decreased
    assert!(controller.current_level() < high_level);
}

#[test]
fn test_minigame_state_transitions() {
    let state = MiniGameState::Countdown { remaining: 3 };
    assert!(state.is_countdown());
    assert!(!state.is_playing());
    assert!(!state.is_paused());
    assert!(!state.is_game_over());

    let state = MiniGameState::Playing;
    assert!(state.is_playing());
    assert!(!state.is_countdown());

    let state = MiniGameState::Paused;
    assert!(state.is_paused());
    assert!(!state.is_playing());

    let state = MiniGameState::GameOver;
    assert!(state.is_game_over());
    assert!(!state.is_playing());
}

#[test]
fn test_session_with_empty_scenarios() {
    let scenarios = Arc::new(Vec::new());
    let session = MiniGameSession::new(scenarios, None);

    // Should handle empty scenario list gracefully
    assert!(session.current_scenario().is_none());
}

#[test]
fn test_multiplier_capped_at_max() {
    let mut stats = MiniGameStats::new();

    // Build streak way beyond cap
    for _ in 0..100 {
        stats.increase_streak();
    }

    // Multiplier should be capped at 5.0 (based on the actual implementation)
    assert_eq!(stats.multiplier, 5.0);
}

#[test]
fn test_countdown_ticks() {
    let scenarios = Arc::new(vec![create_simple_scenario("s1")]);
    let mut session = MiniGameSession::new(scenarios, None);

    session.start();

    // Should start at countdown 3
    assert_eq!(session.state().countdown_remaining(), Some(3));

    session.tick_countdown();
    assert_eq!(session.state().countdown_remaining(), Some(2));

    session.tick_countdown();
    assert_eq!(session.state().countdown_remaining(), Some(1));

    session.tick_countdown();
    assert!(session.state().is_playing());
}
