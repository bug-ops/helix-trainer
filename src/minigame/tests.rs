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
            cursor_position: (0, 0),
        },
        target: TargetState {
            file_content: "".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        solution: Solution {
            commands: vec!["x".to_string()],
            description: "Delete char".to_string(),
        },
        alternatives: vec![],
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: 1,
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

    let mut session = MiniGameSession::new(scenarios);

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

    let mut session = MiniGameSession::new(scenarios);

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
    let mut session = MiniGameSession::new(scenarios);

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
    let mut session = MiniGameSession::new(scenarios);

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
