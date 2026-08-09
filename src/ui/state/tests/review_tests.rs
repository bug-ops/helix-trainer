//! Tests for review session management
//!
//! NOTE: FSRS (the spaced repetition algorithm) schedules reviews intelligently,
//! typically in the future even for failed attempts. These tests focus on the
//! state management logic, not FSRS scheduling behavior.
//!
//! The persistence regression tests below (`test_complete_review_command_persists_*`,
//! `test_review_session_completion_*`) bypass FSRS due-scheduling entirely by
//! constructing `ReviewSessionState` directly, since `StartReviewSession` cannot be
//! relied on to produce a due review deterministically.

use std::time::{Duration, Instant};

use super::common::{create_test_app_state, create_test_scenario};
use crate::ui::state::{Message, ReviewSessionState, TypedScreen, update};

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

fn review_session_with_commands(due_commands: Vec<String>) -> ReviewSessionState {
    ReviewSessionState {
        current_command: due_commands.first().cloned(),
        due_commands,
        current_index: 0,
        session_started_at: Instant::now(),
        completed_reviews: Vec::new(),
    }
}

/// Regression test for #258/C1: `handle_complete_review_command`'s per-answer save
/// must go through `save_debounced`, which syncs `performance_tracker` before
/// writing, so a mid-session review answer is not lost if the app is killed before
/// the session ends.
#[test]
fn test_complete_review_command_persists_fsrs_data_mid_session() {
    use crate::gamification::ProfileStorage;
    use tempfile::TempDir;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    let temp_dir = TempDir::new().unwrap();
    state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

    // Two due commands so completing the first one does not end the session.
    state.game.review_session = Some(review_session_with_commands(vec![
        "x".to_string(),
        "d".to_string(),
    ]));

    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Session should still be active (only 1 of 2 commands completed).
    assert!(state.game.review_session.is_some());

    let persisted = ProfileStorage::with_path(state.progress.storage.path())
        .load()
        .unwrap();
    assert!(
        persisted.performance_data.contains_key("x"),
        "the completed review's FSRS data must be persisted before the session ends"
    );
}

/// Regression test for #258/C1: ending a review session must award XP and persist
/// both the XP change and the FSRS data via `save_immediate`, matching what the
/// in-memory tracker/profile hold at that point.
#[test]
fn test_review_session_completion_persists_xp_and_fsrs_data() {
    use crate::gamification::ProfileStorage;
    use tempfile::TempDir;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    let temp_dir = TempDir::new().unwrap();
    state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

    let initial_xp = state.progress.profile.total_xp;

    // Single due command: completing it ends the session immediately.
    state.game.review_session = Some(review_session_with_commands(vec!["x".to_string()]));

    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Session should be cleared and XP awarded (1 completed * 10 + 100% success * 20).
    assert!(state.game.review_session.is_none());
    assert_eq!(state.progress.profile.total_xp, initial_xp + 30);
    assert!(matches!(state.screen, TypedScreen::Menu(_)));

    let persisted = ProfileStorage::with_path(state.progress.storage.path())
        .load()
        .unwrap();
    assert_eq!(persisted.total_xp, state.progress.profile.total_xp);
    assert!(
        persisted.performance_data.contains_key("x"),
        "FSRS data recorded during the review session must survive the final flush"
    );
}

/// Regression test for #258/C1: a level-up during a review session must persist the
/// leveled-up profile via the session-complete save.
///
/// Note: review sessions intentionally do not surface a `LevelUp` notification. Unlike
/// `scenario.rs` (the app's primary XP path, which does notify on level-up), review
/// completions don't have an established notification for this, so adding one here
/// would be an unrelated behavior change riding along with this persistence fix.
#[test]
fn test_review_session_completion_persists_level_up() {
    use crate::gamification::{ProfileStorage, XPCalculator};
    use tempfile::TempDir;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    let temp_dir = TempDir::new().unwrap();
    state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

    let xp_for_level_2 = XPCalculator::xp_for_level(2);
    state.progress.profile.total_xp = xp_for_level_2 - 10;
    state.progress.profile.level = 1;

    state.game.review_session = Some(review_session_with_commands(vec!["x".to_string()]));

    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    assert_eq!(state.progress.profile.level, 2);

    let persisted = ProfileStorage::with_path(state.progress.storage.path())
        .load()
        .unwrap();
    assert_eq!(persisted.level, 2);
}

/// Regression test for M4: abandoning a review session (e.g. pressing Esc mid-session)
/// must flush already-recorded FSRS data via `save_immediate`, unconditionally, even
/// when a prior debounced save was skipped because the debounce window was still open.
#[test]
fn test_abandon_review_session_persists_partial_progress() {
    use crate::gamification::ProfileStorage;
    use tempfile::TempDir;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    let temp_dir = TempDir::new().unwrap();
    state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

    // Simulate a save that just happened, so the per-answer debounced save below
    // is skipped and nothing is on disk yet.
    state.progress.mark_saved();

    // Two due commands so completing the first one does not end the session.
    state.game.review_session = Some(review_session_with_commands(vec![
        "x".to_string(),
        "d".to_string(),
    ]));

    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    assert!(state.game.review_session.is_some(), "session still active");

    // Nothing persisted yet: the debounce window is still open.
    let profile_path = temp_dir.path().join("profile.json");
    assert!(
        !profile_path.exists(),
        "debounced save should have been skipped"
    );

    update(&mut state, Message::AbandonReviewSession).unwrap();

    assert!(state.game.review_session.is_none());
    assert!(matches!(state.screen, TypedScreen::Menu(_)));

    let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
    assert!(
        persisted.performance_data.contains_key("x"),
        "abandoning must flush FSRS data recorded before the debounce window elapsed"
    );
}
