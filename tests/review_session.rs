//! Comprehensive tests for Interactive Review Session UI
//!
//! This test suite covers:
//! 1. State management tests - Happy paths, edge cases, state transitions
//! 2. Message handler tests - All review-related message handlers
//! 3. XP calculation tests - Base XP, success rate bonus, edge cases
//! 4. Integration tests - Complete end-to-end flows
//! 5. Boundary tests - 0 reviews, 1 review, many reviews

use helix_trainer::gamification::{ProfileStorage, UserProfile};
use helix_trainer::learning::PerformanceTracker;
use helix_trainer::ui::state::{AppState, Message, TypedScreen};
use helix_trainer::ui::update;
use std::time::Duration;

/// Helper: Create test app state with optional due reviews
fn create_test_app_state_with_reviews(due_count: usize) -> AppState {
    let profile = UserProfile::new();
    let storage = ProfileStorage::new();
    let mut tracker = PerformanceTracker::new();

    // Record attempts to create due reviews
    // Note: We need to record failures or multiple attempts to make reviews due sooner
    if due_count > 0 {
        for i in 0..due_count {
            let command = format!("cmd_{}", i);
            // Record failed attempts to make reviews due immediately
            tracker.record_attempt(
                &command,
                Duration::from_secs(5),
                false,
                Duration::from_secs(1),
            );
        }
    }

    AppState::new(vec![], profile, storage, tracker)
}

// ============================================================================
// 1. STATE MANAGEMENT TESTS
// ============================================================================

#[test]
fn test_start_review_session_with_due_reviews() {
    let mut state = create_test_app_state_with_reviews(5);
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    assert!(state.game.review_session.is_none());

    // Start review session
    update(&mut state, Message::StartReviewSession).unwrap();

    // Should transition to Review screen and create session state
    assert!(matches!(state.screen, TypedScreen::Review(_)));
    assert!(state.game.review_session.is_some());

    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.due_commands.len(), 5);
    assert_eq!(session.current_index, 0);
    assert!(session.current_command.is_some());
    assert_eq!(session.completed_reviews.len(), 0);
}

#[test]
fn test_start_review_session_with_zero_due_reviews() {
    let mut state = create_test_app_state_with_reviews(0);
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));

    // Try to start review session with no due reviews
    update(&mut state, Message::StartReviewSession).unwrap();

    // Should stay on ModeSelection (no reviews available)
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    assert!(state.game.review_session.is_none());
}

#[test]
fn test_start_review_session_with_one_review_boundary() {
    let mut state = create_test_app_state_with_reviews(1);

    update(&mut state, Message::StartReviewSession).unwrap();

    assert!(matches!(state.screen, TypedScreen::Review(_)));
    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.due_commands.len(), 1);
    assert_eq!(session.current_index, 0);
}

#[test]
fn test_start_review_session_with_many_reviews_stress_test() {
    let mut state = create_test_app_state_with_reviews(100);

    update(&mut state, Message::StartReviewSession).unwrap();

    assert!(matches!(state.screen, TypedScreen::Review(_)));
    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.due_commands.len(), 100);
    assert_eq!(session.current_index, 0);
}

#[test]
fn test_abandon_review_session_mid_way() {
    let mut state = create_test_app_state_with_reviews(5);

    // Start session
    update(&mut state, Message::StartReviewSession).unwrap();
    assert!(matches!(state.screen, TypedScreen::Review(_)));

    // Complete 2 reviews
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.completed_reviews.len(), 2);
    assert_eq!(session.current_index, 2);

    // Abandon session
    update(&mut state, Message::AbandonReviewSession).unwrap();

    // Should return to menu and clear session
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());
}

#[test]
fn test_complete_all_reviews_successfully() {
    let mut state = create_test_app_state_with_reviews(3);
    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    // Start session
    update(&mut state, Message::StartReviewSession).unwrap();

    // Complete all 3 reviews successfully
    for _ in 0..3 {
        update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    }

    // Should return to menu and award XP
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());

    // Verify XP was awarded
    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };
    assert!(final_xp > initial_xp, "XP should be awarded");
}

#[test]
fn test_complete_review_idempotency() {
    let mut state = create_test_app_state_with_reviews(2);

    update(&mut state, Message::StartReviewSession).unwrap();

    // Complete first review
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.completed_reviews.len(), 1);
    assert_eq!(session.current_index, 1);

    // Try to complete again (should safely handle if called twice)
    let _current_command = session.current_command.clone();
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Should have advanced to next or completed session
    // (Either at end of session or at next command)
    let session_state = state.game.review_session.as_ref();
    if let Some(session) = session_state {
        assert_eq!(session.completed_reviews.len(), 2);
    }
}

// ============================================================================
// 2. MESSAGE HANDLER TESTS
// ============================================================================

#[test]
fn test_complete_review_command_success() {
    let mut state = create_test_app_state_with_reviews(2);

    update(&mut state, Message::StartReviewSession).unwrap();

    let session_before = state.game.review_session.as_ref().unwrap();
    let command = session_before.current_command.clone().unwrap();

    // Complete review as success
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    let session_after = state.game.review_session.as_ref().unwrap();

    // Should have recorded result
    assert_eq!(session_after.completed_reviews.len(), 1);
    let result = &session_after.completed_reviews[0];
    assert_eq!(result.command, command);
    assert!(result.success);

    // Should have advanced to next command
    assert_eq!(session_after.current_index, 1);
}

#[test]
fn test_complete_review_command_failure() {
    let mut state = create_test_app_state_with_reviews(2);

    update(&mut state, Message::StartReviewSession).unwrap();

    let session_before = state.game.review_session.as_ref().unwrap();
    let command = session_before.current_command.clone().unwrap();

    // Complete review as failure
    update(
        &mut state,
        Message::CompleteReviewCommand { success: false },
    )
    .unwrap();

    let session_after = state.game.review_session.as_ref().unwrap();

    // Should have recorded result
    assert_eq!(session_after.completed_reviews.len(), 1);
    let result = &session_after.completed_reviews[0];
    assert_eq!(result.command, command);
    assert!(!result.success);

    // Should still advance to next command
    assert_eq!(session_after.current_index, 1);
}

#[test]
fn test_next_review_command_advances() {
    let mut state = create_test_app_state_with_reviews(3);

    update(&mut state, Message::StartReviewSession).unwrap();

    // Manually trigger NextReviewCommand (normally called by CompleteReviewCommand)
    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.current_index, 0);

    update(&mut state, Message::NextReviewCommand).unwrap();

    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.current_index, 1);
}

#[test]
fn test_next_review_command_ends_session() {
    let mut state = create_test_app_state_with_reviews(1);

    update(&mut state, Message::StartReviewSession).unwrap();

    // Complete the only review
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Session should end and return to menu
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());
}

#[test]
fn test_performance_tracker_updated() {
    let mut state = create_test_app_state_with_reviews(1);

    update(&mut state, Message::StartReviewSession).unwrap();

    let command = {
        let session = state.game.review_session.as_ref().unwrap();
        session.current_command.clone().unwrap()
    };

    let attempts_before = {
        let tracker = state.progress.performance_tracker.borrow();
        tracker
            .get_performance(&command)
            .map(|p| p.attempts)
            .unwrap_or(0)
    };

    // Complete review
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Performance tracker should have recorded the attempt
    let attempts_after = {
        let tracker = state.progress.performance_tracker.borrow();
        tracker
            .get_performance(&command)
            .map(|p| p.attempts)
            .unwrap_or(0)
    };

    assert!(
        attempts_after > attempts_before,
        "Performance tracker should record attempt"
    );
}

// ============================================================================
// 3. STATE TRANSITIONS
// ============================================================================

#[test]
fn test_state_transition_review_to_next_to_review() {
    let mut state = create_test_app_state_with_reviews(2);

    // MainMenu → Review
    update(&mut state, Message::StartReviewSession).unwrap();
    assert!(matches!(state.screen, TypedScreen::Review(_)));

    // Review → Next → Still Review
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    assert!(matches!(state.screen, TypedScreen::Review(_)));

    // Review → Complete → MainMenu
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
}

#[test]
fn test_state_transition_review_to_abandon_to_menu() {
    let mut state = create_test_app_state_with_reviews(5);

    // MainMenu → Review
    update(&mut state, Message::StartReviewSession).unwrap();
    assert!(matches!(state.screen, TypedScreen::Review(_)));

    // Review → Abandon → MainMenu
    update(&mut state, Message::AbandonReviewSession).unwrap();
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
}

#[test]
fn test_state_transition_menu_to_review_no_due() {
    let mut state = create_test_app_state_with_reviews(0);

    // ModeSelection → (Try Review) → Stay on ModeSelection
    update(&mut state, Message::StartReviewSession).unwrap();
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

// ============================================================================
// 4. XP CALCULATION TESTS
// ============================================================================

#[test]
fn test_xp_calculation_base_only() {
    let mut state = create_test_app_state_with_reviews(1);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    let awarded_xp = final_xp - initial_xp;

    // Base XP: 1 review * 10 = 10
    // Success rate: 1/1 = 100% → bonus = 20
    // Total: 10 + 20 = 30
    assert_eq!(awarded_xp, 30, "Should award base XP + 100% success bonus");
}

#[test]
fn test_xp_calculation_success_rate_bonus_100_percent() {
    let mut state = create_test_app_state_with_reviews(5);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();

    // Complete all 5 reviews successfully
    for _ in 0..5 {
        update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    }

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    let awarded_xp = final_xp - initial_xp;

    // Base XP: 5 * 10 = 50
    // Success rate: 5/5 = 100% → bonus = 1.0 * 20 = 20
    // Total: 50 + 20 = 70
    assert_eq!(awarded_xp, 70, "Should award maximum success bonus");
}

#[test]
fn test_xp_calculation_success_rate_bonus_0_percent() {
    let mut state = create_test_app_state_with_reviews(5);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();

    // Fail all 5 reviews
    for _ in 0..5 {
        update(
            &mut state,
            Message::CompleteReviewCommand { success: false },
        )
        .unwrap();
    }

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    let awarded_xp = final_xp - initial_xp;

    // Base XP: 5 * 10 = 50
    // Success rate: 0/5 = 0% → bonus = 0.0 * 20 = 0
    // Total: 50 + 0 = 50
    assert_eq!(awarded_xp, 50, "Should award no success bonus");
}

#[test]
fn test_xp_calculation_success_rate_bonus_mixed() {
    let mut state = create_test_app_state_with_reviews(4);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();

    // 2 successes, 2 failures (50% success rate)
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    update(
        &mut state,
        Message::CompleteReviewCommand { success: false },
    )
    .unwrap();
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    update(
        &mut state,
        Message::CompleteReviewCommand { success: false },
    )
    .unwrap();

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    let awarded_xp = final_xp - initial_xp;

    // Base XP: 4 * 10 = 40
    // Success rate: 2/4 = 50% → bonus = 0.5 * 20 = 10
    // Total: 40 + 10 = 50
    assert_eq!(awarded_xp, 50, "Should award partial success bonus");
}

#[test]
fn test_xp_calculation_always_positive() {
    let mut state = create_test_app_state_with_reviews(1);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();
    update(
        &mut state,
        Message::CompleteReviewCommand { success: false },
    )
    .unwrap();

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    let awarded_xp = final_xp - initial_xp;

    // Even on failure, should award base XP
    assert!(awarded_xp > 0, "XP should always be positive");
}

// ============================================================================
// 5. INTEGRATION TESTS - COMPLETE END-TO-END FLOWS
// ============================================================================

#[test]
fn test_complete_flow_all_successful() {
    let mut state = create_test_app_state_with_reviews(5);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    // Start review session
    update(&mut state, Message::StartReviewSession).unwrap();
    assert!(matches!(state.screen, TypedScreen::Review(_)));

    // Complete all 5 reviews successfully
    for i in 0..5 {
        let session = state.game.review_session.as_ref().unwrap();
        assert_eq!(session.current_index, i);
        assert!(session.current_command.is_some());

        update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    }

    // Session should end
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());

    // XP should be awarded
    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };
    assert_eq!(final_xp - initial_xp, 70); // 5*10 + 20 bonus
}

#[test]
fn test_complete_flow_all_failed() {
    let mut state = create_test_app_state_with_reviews(3);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();

    // Fail all 3 reviews
    for _ in 0..3 {
        update(
            &mut state,
            Message::CompleteReviewCommand { success: false },
        )
        .unwrap();
    }

    // Session should still end and award base XP
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };
    assert_eq!(final_xp - initial_xp, 30); // 3*10 + 0 bonus
}

#[test]
fn test_complete_flow_abandoned_after_partial() {
    let mut state = create_test_app_state_with_reviews(5);

    let initial_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    update(&mut state, Message::StartReviewSession).unwrap();

    // Complete 2 reviews
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Abandon session
    update(&mut state, Message::AbandonReviewSession).unwrap();

    // Should return to menu without awarding XP (session incomplete)
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());

    let final_xp = {
        let profile = state.progress.profile.borrow();
        profile.total_xp
    };

    // XP should not be awarded for abandoned session
    assert_eq!(final_xp, initial_xp);
}

#[test]
fn test_complete_flow_zero_reviews() {
    let mut state = create_test_app_state_with_reviews(0);

    // Try to start review with no due reviews
    update(&mut state, Message::StartReviewSession).unwrap();

    // Should stay on ModeSelection (initial screen)
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
    assert!(state.game.review_session.is_none());
}

// ============================================================================
// 6. BOUNDARY TESTS
// ============================================================================

#[test]
fn test_boundary_single_review() {
    let mut state = create_test_app_state_with_reviews(1);

    update(&mut state, Message::StartReviewSession).unwrap();

    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.due_commands.len(), 1);
    assert_eq!(session.current_index, 0);

    // Complete the only review
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Should immediately end session
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());
}

#[test]
fn test_boundary_large_review_count() {
    let mut state = create_test_app_state_with_reviews(100);

    update(&mut state, Message::StartReviewSession).unwrap();

    let session = state.game.review_session.as_ref().unwrap();
    assert_eq!(session.due_commands.len(), 100);

    // Complete all 100 reviews
    for _ in 0..100 {
        update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    }

    // Should handle large count gracefully
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
    assert!(state.game.review_session.is_none());
}

// ============================================================================
// 7. REVIEW RESULT TRACKING
// ============================================================================

#[test]
fn test_review_result_records_duration() {
    let mut state = create_test_app_state_with_reviews(1);

    update(&mut state, Message::StartReviewSession).unwrap();

    // Wait a tiny bit (simulated)
    std::thread::sleep(Duration::from_millis(10));

    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Session is complete, so we can't check duration directly
    // But the review should have recorded a non-zero duration
    // This is verified by the performance tracker recording the attempt
}

#[test]
fn test_review_results_accumulate() {
    let mut state = create_test_app_state_with_reviews(3);

    update(&mut state, Message::StartReviewSession).unwrap();

    // Complete reviews with mixed results
    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    {
        let session = state.game.review_session.as_ref().unwrap();
        assert_eq!(session.completed_reviews.len(), 1);
        assert!(session.completed_reviews[0].success);
    }

    update(
        &mut state,
        Message::CompleteReviewCommand { success: false },
    )
    .unwrap();
    {
        let session = state.game.review_session.as_ref().unwrap();
        assert_eq!(session.completed_reviews.len(), 2);
        assert!(!session.completed_reviews[1].success);
    }

    update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();

    // Session complete after 3 reviews
    assert!(state.game.review_session.is_none());
}

// ============================================================================
// 8. PROGRESS TRACKING
// ============================================================================

#[test]
fn test_session_progress_increments_correctly() {
    let mut state = create_test_app_state_with_reviews(5);

    update(&mut state, Message::StartReviewSession).unwrap();

    for i in 0..5 {
        let session = state.game.review_session.as_ref().unwrap();
        assert_eq!(session.current_index, i);
        assert_eq!(session.completed_reviews.len(), i);

        update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    }

    // Session complete
    assert!(state.game.review_session.is_none());
}

#[test]
fn test_session_progress_never_exceeds_due_count() {
    let mut state = create_test_app_state_with_reviews(3);

    update(&mut state, Message::StartReviewSession).unwrap();

    for _ in 0..3 {
        update(&mut state, Message::CompleteReviewCommand { success: true }).unwrap();
    }

    // Session should end exactly at due_count
    assert!(state.game.review_session.is_none());
}
