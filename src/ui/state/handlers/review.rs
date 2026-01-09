//! Review session message handlers
//!
//! Handles spaced repetition review sessions

use crate::constants::OPTIMAL_REVIEW_TIME;
use crate::security::UserError;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::{
    HandlerContext, HandlerOutcome, MenuData, ReviewData, ReviewResult, ReviewSessionState,
    TypedScreen,
};
use std::time::Instant;

/// Handle StartReviewSession message
///
/// Initializes a new review session with due commands
pub fn handle_start_review_session(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
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
        return Ok(HandlerOutcome::Stay);
    }

    let review_session = ReviewSessionState {
        due_commands: due_commands.clone(),
        current_index: 0,
        current_command: due_commands.first().cloned(),
        session_started_at: Instant::now(),
        completed_reviews: Vec::new(),
    };

    ctx.game.review_session = Some(review_session.clone());
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Review(
        ReviewData::new(review_session),
    ))))
}

/// Handle CompleteReviewCommand message
///
/// Records review result and updates performance tracker
///
/// Note: This handler mutates review session data and then delegates to
/// handle_next_review_command for screen transitions
pub fn handle_complete_review_command(
    ctx: &mut HandlerContext<'_>,
    success: bool,
) -> Result<HandlerOutcome, UserError> {
    if let Some(session) = &mut ctx.game.review_session
        && let Some(command) = &session.current_command
    {
        let duration = session.session_started_at.elapsed();

        // Record result
        session.completed_reviews.push(ReviewResult {
            command: command.clone(),
            success,
            duration,
        });

        // Update performance tracker
        let tracker = &mut ctx.progress.performance_tracker;
        tracker.record_attempt(command, duration, success, OPTIMAL_REVIEW_TIME);
    }

    // Delegate to next review handler for screen transition logic
    handle_next_review_command(ctx)
}

/// Handle NextReviewCommand message
///
/// Advances to next review command or completes session
pub fn handle_next_review_command(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    if let Some(session) = &mut ctx.game.review_session {
        session.current_index += 1;

        if session.current_index >= session.due_commands.len() {
            // Session complete - show summary and award XP
            let completed = session.completed_reviews.len();
            let success_count = session
                .completed_reviews
                .iter()
                .filter(|r| r.success)
                .count();
            let success_rate = if completed > 0 {
                success_count as f64 / completed as f64
            } else {
                0.0
            };

            // Award XP for review session
            let xp = (completed as u64 * 10) + (success_rate * 20.0) as u64;
            let profile = &mut ctx.progress.profile;
            profile.add_xp(xp);

            // Show session summary notification
            ctx.ui
                .notifications
                .push(Notification::new(NotificationType::ReviewSessionComplete {
                    completed,
                    success_count,
                    xp_earned: xp,
                }));

            // Clear review session
            ctx.game.review_session = None;

            // Return to menu
            return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
                MenuData::default(),
            ))));
        } else {
            // Move to next command
            session.current_command = session.due_commands.get(session.current_index).cloned();
        }
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle AbandonReviewSession message
///
/// Cancels the current review session
pub fn handle_abandon_review_session(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    ctx.game.review_session = None;
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData::default(),
    ))))
}
