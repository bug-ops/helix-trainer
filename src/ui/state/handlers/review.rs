//! Review session message handlers
//!
//! Handles spaced repetition review sessions

use crate::gamification::XPCalculator;
use crate::learning::{ReviewOutcome, ReviewSession, ReviewStep, SessionSummary};
use crate::security::error_chain;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::Effect;
use crate::ui::state::{HandlerContext, HandlerOutcome, MenuData, ReviewData, TypedScreen};
use rust_i18n::t;

/// Handle `StartReviewSession` message
///
/// Initializes a new review session with due commands
pub fn handle_start_review_session(ctx: &mut HandlerContext<'_>) -> HandlerOutcome {
    let due_commands = ctx
        .progress
        .scheduler
        .get_due_reviews(&ctx.progress.performance_tracker);

    let Some(session) = ReviewSession::start(due_commands, ctx.progress.monotonic_now()) else {
        ctx.ui
            .notifications
            .push(Notification::new(NotificationType::Info {
                message: t!("review.no_due").to_string(),
            }));
        return HandlerOutcome::Stay;
    };

    HandlerOutcome::Transition(Box::new(TypedScreen::Review(ReviewData::new(session))))
}

/// Handle `CompleteReviewCommand` message
///
/// Records the answer for the current command and either shows the next one or, after the
/// last, awards XP and returns to the menu. Always transitions: the session is consumed by
/// the answer.
pub fn handle_complete_review_command(
    data: ReviewData,
    ctx: &mut HandlerContext<'_>,
    outcome: ReviewOutcome,
) -> HandlerOutcome {
    let step = data.session.answer(
        outcome,
        ctx.progress.monotonic_now(),
        &mut ctx.progress.performance_tracker,
    );

    // Persist periodically so review progress survives a crash mid-session
    // (non-fatal error - log but continue, a review keypress must not kill the session)
    if let Err(e) = ctx.progress.save_debounced() {
        tracing::error!(
            error = %error_chain(&e),
            "Failed to save profile during review session"
        );
    }

    match step {
        ReviewStep::Next(session) => {
            HandlerOutcome::Transition(Box::new(TypedScreen::Review(ReviewData::new(session))))
        }
        ReviewStep::Finished(summary) => finish_review_session(&summary, ctx),
    }
}

/// Award XP for a finished session, show the summary notification and return to the menu
fn finish_review_session(summary: &SessionSummary, ctx: &mut HandlerContext<'_>) -> HandlerOutcome {
    let xp = XPCalculator::review_session_xp(summary.total_reviews, summary.successful);
    let leveled_up = ctx.progress.profile.add_xp(xp);
    let new_level = ctx.progress.profile.level;

    ctx.ui
        .notifications
        .push(Notification::new(NotificationType::ReviewSessionComplete {
            completed: summary.total_reviews,
            success_count: summary.successful,
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

    HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())))
}

/// Handle `AbandonReviewSession` message
///
/// Cancels the current review session
pub fn handle_abandon_review_session(ctx: &mut HandlerContext<'_>) -> HandlerOutcome {
    // Persist attempts already recorded so far - abandoning mid-session (e.g. Esc)
    // is the likelier interrupted-user path, so it should flush just like completion does
    ctx.ui.effects.push(Effect::SaveProfile);

    HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())))
}
