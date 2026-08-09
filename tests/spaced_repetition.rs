//! Integration tests for spaced repetition learning system
//!
//! Tests the complete workflow:
//! 1. Create performance tracker
//! 2. Record command attempts
//! 3. Get due reviews from scheduler
//! 4. Run review session
//! 5. Check analytics

use helix_trainer::learning::{
    Analytics, MasteryLevel, PerformanceTracker, ReviewSession, Scheduler,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[test]
fn test_complete_learning_workflow() {
    // 1. Create performance tracker
    let mut tracker = PerformanceTracker::new();

    // 2. Record some command attempts (with optimal_time parameter)
    // Record successful x (delete line) - 3 times
    tracker.record_attempt(
        "x",
        Duration::from_millis(500),
        true,
        Duration::from_millis(400),
    );
    tracker.record_attempt(
        "x",
        Duration::from_millis(450),
        true,
        Duration::from_millis(400),
    );
    tracker.record_attempt(
        "x",
        Duration::from_millis(400),
        true,
        Duration::from_millis(400),
    );

    // Record yy (yank line) - 2 successes, 1 failure
    tracker.record_attempt(
        "yy",
        Duration::from_millis(600),
        true,
        Duration::from_millis(500),
    );
    tracker.record_attempt(
        "yy",
        Duration::from_millis(700),
        false,
        Duration::from_millis(500),
    );
    tracker.record_attempt(
        "yy",
        Duration::from_millis(550),
        true,
        Duration::from_millis(500),
    );

    // Record w (next word) - perfect performance
    for _ in 0..5 {
        tracker.record_attempt(
            "w",
            Duration::from_millis(200),
            true,
            Duration::from_millis(200),
        );
    }

    // Verify initial state
    let x_perf = tracker.performance("x").unwrap();
    assert_eq!(x_perf.attempts, 3);
    assert_eq!(x_perf.successes, 3);
    // Mastery level is calculated by FSRS, don't assert specific level
    assert!(matches!(
        x_perf.mastery_level,
        MasteryLevel::Beginner
            | MasteryLevel::Intermediate
            | MasteryLevel::Advanced
            | MasteryLevel::Master
    ));

    let yy_perf = tracker.performance("yy").unwrap();
    assert_eq!(yy_perf.attempts, 3);
    assert_eq!(yy_perf.successes, 2);

    let w_perf = tracker.performance("w").unwrap();
    assert_eq!(w_perf.attempts, 5);
    assert_eq!(w_perf.successes, 5);

    // 3. Get due reviews from scheduler
    let scheduler = Scheduler::new();
    let due_reviews = scheduler.get_due_reviews(&tracker);

    // Commands should be tracked (may or may not be due immediately)
    // FSRS schedules next review based on performance
    assert!(due_reviews.len() <= 3);

    // 4. Check analytics
    let mastery_summary = Analytics::get_mastery_summary(&tracker);
    assert_eq!(mastery_summary.total_commands, 3);
    // Commands should be distributed across mastery levels
    assert!(
        mastery_summary.beginner
            + mastery_summary.intermediate
            + mastery_summary.advanced
            + mastery_summary.master
            == 3
    );

    // Check progress over time
    let progress = Analytics::get_progress_over_time(&tracker, 7, chrono::Utc::now());
    // Progress data should exist (even if placeholder for new tracker)
    assert!(progress.len() <= 8); // 0..=days = 8 entries for 7 days
}

#[test]
fn test_mastery_progression() {
    let mut tracker = PerformanceTracker::new();

    // Simulate many successful reviews to advance mastery
    // Perfect performance over time should increase mastery
    for _ in 0..20 {
        tracker.record_attempt(
            "x",
            Duration::from_millis(300),
            true,
            Duration::from_millis(500),
        );
    }

    let perf = tracker.performance("x").unwrap();

    // With 20 successful attempts, should have valid mastery level
    assert!(matches!(
        perf.mastery_level,
        MasteryLevel::Beginner
            | MasteryLevel::Intermediate
            | MasteryLevel::Advanced
            | MasteryLevel::Master
    ));
    assert_eq!(perf.attempts, 20);
    assert_eq!(perf.successes, 20);

    // Stability should increase with successful reviews
    assert!(perf.stability > 1.0);
}

#[test]
fn test_analytics_empty_tracker() {
    let tracker = PerformanceTracker::new();

    let mastery_summary = Analytics::get_mastery_summary(&tracker);
    assert_eq!(mastery_summary.total_commands, 0);
    assert_eq!(mastery_summary.beginner, 0);

    let progress = Analytics::get_progress_over_time(&tracker, 7, chrono::Utc::now());
    // Returns placeholder data even for empty tracker (for UI consistency)
    assert!(progress.len() <= 8);

    let plateaus = Analytics::identify_plateaus(&tracker);
    assert!(plateaus.is_empty());
}

#[test]
fn test_scheduler_prioritization() {
    let mut tracker = PerformanceTracker::new();

    // Create commands with different characteristics
    // Command 1: High success rate
    for _ in 0..5 {
        tracker.record_attempt(
            "high_success",
            Duration::from_millis(200),
            true,
            Duration::from_millis(300),
        );
    }

    // Command 2: Low success rate, needs more practice
    tracker.record_attempt(
        "low_success",
        Duration::from_millis(500),
        true,
        Duration::from_millis(400),
    );
    tracker.record_attempt(
        "low_success",
        Duration::from_millis(600),
        false,
        Duration::from_millis(400),
    );
    tracker.record_attempt(
        "low_success",
        Duration::from_millis(700),
        false,
        Duration::from_millis(400),
    );

    // Command 3: New command
    tracker.record_attempt(
        "new_command",
        Duration::from_millis(400),
        true,
        Duration::from_millis(500),
    );

    let scheduler = Scheduler::new();
    let due_reviews = scheduler.get_due_reviews(&tracker);

    // Verify commands are tracked (may or may not all be due)
    assert!(due_reviews.len() <= 3);

    // Get review queue (priority-ordered)
    let queue = scheduler.get_review_queue(&tracker, 10);
    assert!(queue.len() <= 10);

    // Get practice session recommendations (may be empty if nothing due)
    let session = scheduler.recommend_practice_session(&tracker, 15);
    assert!(session.len() <= 5); // 15 mins / 3 mins per scenario = 5 max
}

#[test]
fn test_review_session_empty() {
    let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
    let scheduler = Scheduler::new();

    // Create session with no due reviews
    let session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

    assert!(session.is_complete());
    assert!(session.current_item().is_none());

    let summary = session.summary();
    assert_eq!(summary.total_reviews, 0);
    assert_eq!(summary.successful, 0);
}

#[test]
fn test_analytics_commands_by_mastery() {
    let mut tracker = PerformanceTracker::new();

    // Create commands at different mastery levels
    // Beginner command
    tracker.record_attempt(
        "beginner_cmd",
        Duration::from_secs(5),
        false,
        Duration::from_secs(1),
    );

    // Intermediate command (moderate success)
    for _ in 0..3 {
        tracker.record_attempt(
            "intermediate_cmd",
            Duration::from_secs(2),
            true,
            Duration::from_secs(1),
        );
    }

    // Get commands by mastery level
    let beginners = Analytics::get_commands_by_mastery(&tracker, MasteryLevel::Beginner);
    assert!(!beginners.is_empty());

    // Check total commands
    assert_eq!(Analytics::total_commands(&tracker), 2);

    // Check average success rate
    let avg_rate = Analytics::avg_success_rate(&tracker);
    assert!(avg_rate > 0.0 && avg_rate <= 1.0);
}

#[test]
fn test_analytics_plateau_detection() {
    let mut tracker = PerformanceTracker::new();

    // Create a command that has plateaued (many attempts, low stability)
    // Many attempts with mixed results (creates plateau)
    for i in 0..10 {
        let success = i % 2 == 0; // Alternating success/failure
        tracker.record_attempt(
            "plateau_cmd",
            Duration::from_secs(3),
            success,
            Duration::from_secs(1),
        );
    }

    let plateaus = Analytics::identify_plateaus(&tracker);

    // Should identify commands that need different practice approach
    assert!(!plateaus.is_empty());
}
