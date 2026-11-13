//! Integration tests for mini-game module
//!
//! Tests the interaction between components.

use super::*;
use crate::config::{
    Difficulty, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
};
use crate::learning::AttemptOutcome;
use crate::testing::sid;
use std::sync::Arc;

fn create_simple_scenario(id: &str) -> Scenario {
    Scenario {
        id: sid(id),
        name: format!("Test {id}"),
        description: "Test scenario".to_string(),
        setup: Setup {
            file_content: "test".to_string(),
            language: None,
            cursor: crate::testing::point(0, 0),
        },
        target: TargetState {
            file_content: String::new(),
            cursor: crate::testing::point(0, 0),
        },
        solution: Solution {
            commands: vec!["x".to_string()],
            description: "Delete char".to_string(),
        },
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
            max_points: 100,
            tolerance: 0,
        },
        metadata: ScenarioMetadata {
            difficulty: Some(Difficulty::Beginner),
            ..Default::default()
        },
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

    let initial_level = session.difficulty_level().get();

    // Simulate multiple successes
    for _ in 0..10 {
        session.advance_to_next();
        let _ = session.complete_transition();
    }

    // Difficulty should increase
    assert!(session.difficulty_level().get() > initial_level);
}

#[test]
fn test_advance_to_next_drives_streak_and_multiplier() {
    let scenarios = Arc::new(vec![
        create_simple_scenario("s1"),
        create_simple_scenario("s2"),
        create_simple_scenario("s3"),
    ]);

    let mut session = MiniGameSession::new(scenarios, None);

    session.start();
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();

    // Drive real completions through the production path (advance_to_next +
    // complete_transition), not direct MiniGameStats writes, to prove the
    // deleted session-level sync is no longer needed for the streak/
    // multiplier tier table to advance correctly.
    for _ in 0..6 {
        session.advance_to_next();
        let _ = session.complete_transition();
    }

    assert_eq!(session.stats().streak(), 6);
    assert_eq!(session.stats().best_streak(), 6);
    assert!((session.stats().multiplier() - 2.0).abs() < f64::EPSILON);
}

#[test]
fn test_handle_timeout_after_real_streak_uses_grace() {
    let scenarios = Arc::new(vec![
        create_simple_scenario("s1"),
        create_simple_scenario("s2"),
        create_simple_scenario("s3"),
    ]);

    let mut session = MiniGameSession::new(scenarios, None);

    session.start();
    session.tick_countdown();
    session.tick_countdown();
    session.tick_countdown();

    // Reach the streak-10 milestone through the real advance_to_next path so
    // grace is granted by MultiplierState, then fail immediately: grace
    // should protect the streak instead of resetting it.
    for _ in 0..10 {
        session.advance_to_next();
        let _ = session.complete_transition();
    }
    assert_eq!(session.stats().grace_remaining(), 1);
    let multiplier_before = session.stats().multiplier();

    session.handle_timeout();

    assert_eq!(session.stats().streak(), 10);
    assert_eq!(session.stats().grace_remaining(), 0);
    assert!((session.stats().multiplier() - multiplier_before).abs() < f64::EPSILON);
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
    assert_eq!(session.stats().lives(), 0);
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
    assert_eq!(stats.lives(), 3);
    assert_eq!(stats.streak(), 0);
    assert_eq!(stats.best_streak(), 0);
    assert_eq!(stats.scenarios_completed, 0);
    assert_eq!(stats.scenarios_failed, 0);
    assert_eq!(stats.multiplier(), 1.0);
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
    assert_eq!(stats.lives(), 3);

    assert!(stats.lose_life()); // Still has lives
    assert_eq!(stats.lives(), 2);

    assert!(stats.lose_life()); // Still has lives
    assert_eq!(stats.lives(), 1);

    assert!(!stats.lose_life()); // Game over
    assert_eq!(stats.lives(), 0);

    // Can't go below 0
    assert!(!stats.lose_life());
    assert_eq!(stats.lives(), 0);
}

#[test]
fn test_difficulty_controller_initial_level() {
    let controller = DifficultyController::new();
    assert_eq!(controller.current_level().get(), 1);
}

#[test]
fn test_difficulty_controller_adjust_up() {
    let mut controller = DifficultyController::new();

    // Record multiple quick successes at different efficiency levels
    for _ in 0..15 {
        let point =
            PerformancePoint::new(AttemptOutcome::Succeeded, 0.3, Difficulty::Beginner, 1.0);
        controller.update_after_scenario(point);
    }

    // Level should have increased
    assert!(controller.current_level().get() > 1);
}

#[test]
fn test_difficulty_controller_adjust_down() {
    let mut controller = DifficultyController::new();

    // First increase level
    for _ in 0..20 {
        let point =
            PerformancePoint::new(AttemptOutcome::Succeeded, 0.3, Difficulty::Beginner, 1.0);
        controller.update_after_scenario(point);
    }

    let high_level = controller.current_level().get();

    // Now record failures
    for _ in 0..15 {
        let point = PerformancePoint::new(AttemptOutcome::Failed, 1.0, Difficulty::Beginner, 0.2);
        controller.update_after_scenario(point);
    }

    // Level should have decreased
    assert!(controller.current_level().get() < high_level);
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

fn arcade_completion_step(commands: &[&str]) -> Option<usize> {
    let undo_redo = crate::config::ScenarioLoader::new()
        .load_from_embedded("en")
        .unwrap()
        .into_iter()
        .find(|s| s.id.as_str() == "undo_redo_001")
        .unwrap();
    let mut session = MiniGameSession::new(Arc::new(vec![undo_redo]), None);
    session.start();
    for _ in 0..3 {
        session.tick_countdown();
    }
    assert_eq!(
        session.current_scenario().unwrap().scenario().id.as_str(),
        "undo_redo_001"
    );
    for (i, cmd) in commands.iter().enumerate() {
        session.handle_command(cmd).unwrap();
        if session.check_completion() {
            return Some(i + 1);
        }
    }
    None
}

#[test]
fn test_arcade_undo_redo_completes_only_at_last_taught_command() {
    assert_eq!(arcade_completion_step(&["x", "d", "u", "U"]), Some(4));
}

#[test]
fn test_arcade_undo_redo_not_completed_when_target_reached_by_untaught_command() {
    assert_eq!(arcade_completion_step(&["u", "U", "x", "d"]), None);
}

#[test]
fn test_arcade_undo_redo_counted_taught_command_completes() {
    assert_eq!(arcade_completion_step(&["x", "d", "2u", "U"]), Some(4));
}

#[test]
fn test_arcade_undo_redo_not_completed_by_noop_redo() {
    assert_eq!(arcade_completion_step(&["x", "d", "U"]), None);
    assert_eq!(arcade_completion_step(&["x", "d", "U", "u", "U"]), Some(5));
}
