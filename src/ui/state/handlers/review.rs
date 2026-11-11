//! Review session message handlers
//!
//! Handles spaced repetition review sessions

use crate::constants::OPTIMAL_REVIEW_TIME;
use crate::gamification::XPCalculator;
use crate::security::error_chain;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::Effect;
use crate::ui::state::{
    HandlerContext, HandlerOutcome, MenuData, ReviewData, ReviewOutcome, ReviewResult,
    ReviewSessionState, TypedScreen,
};
use std::time::Instant;

/// Handle StartReviewSession message
///
/// Initializes a new review session with due commands
pub fn handle_start_review_session(ctx: &mut HandlerContext<'_>) -> HandlerOutcome {
    // Get due commands from scheduler
    let due_commands = ctx
        .progress
        .scheduler
        .get_due_reviews(&ctx.progress.performance_tracker);

    if due_commands.is_empty() {
        // No reviews due - show informative notification
        ctx.ui
            .notifications
            .push(Notification::new(NotificationType::Info {
                message: "No reviews due. Keep practicing!".to_string(),
            }));
        return HandlerOutcome::Stay;
    }

    let review_session = ReviewSessionState {
        due_commands,
        current_index: 0,
        session_started_at: Instant::now(),
        completed_reviews: Vec::new(),
    };

    HandlerOutcome::Transition(Box::new(TypedScreen::Review(ReviewData::new(
        review_session,
    ))))
}

/// Handle CompleteReviewCommand message
///
/// Records review result and updates performance tracker
///
/// Note: This handler mutates review session data and then delegates to
/// handle_next_review_command for screen transitions
pub fn handle_complete_review_command(
    data: &mut ReviewData,
    ctx: &mut HandlerContext<'_>,
    outcome: ReviewOutcome,
) -> HandlerOutcome {
    let session = &mut data.session;
    if let Some(command) = session.current_command().cloned() {
        let duration = session.session_started_at.elapsed();

        // Record result
        session.completed_reviews.push(ReviewResult {
            command: command.clone(),
            outcome,
            duration,
        });

        // Update performance tracker
        let tracker = &mut ctx.progress.performance_tracker;
        tracker.record_attempt(
            &command,
            duration,
            outcome.is_recalled(),
            OPTIMAL_REVIEW_TIME,
        );

        // Persist periodically so review progress survives a crash mid-session
        // (non-fatal error - log but continue, a review keypress must not kill the session)
        if let Err(e) = ctx.progress.save_debounced() {
            tracing::error!(
                error = %error_chain(&e),
                "Failed to save profile during review session"
            );
        }
    }

    // Delegate to next review handler for screen transition logic
    handle_next_review_command(data, ctx)
}

/// Handle NextReviewCommand message
///
/// Advances to next review command or completes session
fn handle_next_review_command(
    data: &mut ReviewData,
    ctx: &mut HandlerContext<'_>,
) -> HandlerOutcome {
    let session = &mut data.session;
    session.current_index += 1;

    if session.current_index >= session.due_commands.len() {
        // Session complete - show summary and award XP
        let completed = session.completed_reviews.len();
        let success_count = session
            .completed_reviews
            .iter()
            .filter(|r| r.outcome.is_recalled())
            .count();
        let xp = XPCalculator::review_session_xp(completed, success_count);
        let leveled_up = ctx.progress.profile.add_xp(xp);
        let new_level = ctx.progress.profile.level;

        // Show session summary notification
        ctx.ui
            .notifications
            .push(Notification::new(NotificationType::ReviewSessionComplete {
                completed,
                success_count,
                xp_earned: xp,
            }));

        if leveled_up {
            ctx.ui
                .notifications
                .push(Notification::new(NotificationType::LevelUp { new_level }));
        }

        // Persist FSRS state and profile now that the review session has ended
        // (non-fatal error - log but continue, matches handle_minigame_game_over's policy)
        ctx.ui.effects.push(Effect::SaveProfile);

        // Return to menu
        return HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())));
    }
    HandlerOutcome::Stay
}

/// Handle AbandonReviewSession message
///
/// Cancels the current review session
pub fn handle_abandon_review_session(ctx: &mut HandlerContext<'_>) -> HandlerOutcome {
    // Persist attempts already recorded so far - abandoning mid-session (e.g. Esc)
    // is the likelier interrupted-user path, so it should flush just like completion does
    ctx.ui.effects.push(Effect::SaveProfile);

    HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())))
}
