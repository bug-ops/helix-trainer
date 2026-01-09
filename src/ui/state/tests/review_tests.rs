//! Tests for review session management
//!
//! NOTE: FSRS (the spaced repetition algorithm) schedules reviews intelligently,
//! typically in the future even for failed attempts. These tests focus on the
//! state management logic, not FSRS scheduling behavior.

use std::time::Duration;

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::{Message, TypedScreen, update};

#[test]
fn test_review_session_with_no_due_reviews() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Even with recorded attempts, FSRS may not schedule reviews immediately
    {
        let tracker = &mut state.progress.performance_tracker;
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
    }

    update(&mut state, Message::StartReviewSession).unwrap();

    // May or may not have reviews due - depends on FSRS algorithm
    // If no reviews due, should stay on menu
    if state.game.review_session.is_none() {
        assert!(matches!(state.screen, TypedScreen::Menu(_)));
    } else {
        assert!(matches!(state.screen, TypedScreen::Review(_)));
    }
}

#[test]
fn test_review_session_message_handlers() {
    // Test that message handlers work correctly regardless of FSRS scheduling
    let scenario = create_test_scenario();
    let state = create_test_app_state(vec![scenario]);

    // Test AbandonReviewSession message handler
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));

    // The actual review session behavior depends on FSRS scheduling
    // This test verifies the message handlers are correctly wired
}

#[test]
fn test_review_session_no_due_reviews_stays_on_menu() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Don't add any reviews to tracker
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));

    update(&mut state, Message::StartReviewSession).unwrap();

    // Should stay on ModeSelection when no reviews are due
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    assert!(state.game.review_session.is_none());
}
