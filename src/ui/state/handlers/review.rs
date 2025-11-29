//! Review session message handlers
//!
//! Handles spaced repetition review sessions

use crate::security::UserError;
use crate::ui::state::{AppState, Message, ReviewResult, ReviewSessionState, Screen, update};
use std::time::Instant;

/// Handle StartReviewSession message
///
/// Initializes a new review session with due commands
pub fn handle_start_review_session(state: &mut AppState) -> Result<(), UserError> {
    // Get due commands from scheduler
    let due_commands = state.scheduler.get_due_reviews();

    if due_commands.is_empty() {
        // No reviews due, stay on menu
        return Ok(());
    }

    state.review_session = Some(ReviewSessionState {
        due_commands: due_commands.clone(),
        current_index: 0,
        current_command: due_commands.first().cloned(),
        session_started_at: Instant::now(),
        completed_reviews: Vec::new(),
    });

    state.screen = Screen::Review;
    Ok(())
}

/// Handle CompleteReviewCommand message
///
/// Records review result and updates performance tracker
pub fn handle_complete_review_command(
    state: &mut AppState,
    success: bool,
) -> Result<(), UserError> {
    if let Some(session) = &mut state.review_session {
        if let Some(command) = &session.current_command {
            let duration = session.session_started_at.elapsed();

            // Record result
            session.completed_reviews.push(ReviewResult {
                command: command.clone(),
                success,
                duration,
            });

            // Update performance tracker
            {
                let mut tracker = state.performance_tracker.borrow_mut();
                tracker.record_attempt(
                    command,
                    duration,
                    success,
                    std::time::Duration::from_secs(3), // Optimal time
                );
            }

            // Move to next
            update(state, Message::NextReviewCommand)?;
        }
    }
    Ok(())
}

/// Handle NextReviewCommand message
///
/// Advances to next review command or completes session
pub fn handle_next_review_command(state: &mut AppState) -> Result<(), UserError> {
    if let Some(session) = &mut state.review_session {
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
            {
                let mut profile = state.profile.borrow_mut();
                profile.add_xp(xp);
            }

            // Return to menu
            state.screen = Screen::MainMenu;
            state.review_session = None;
        } else {
            // Move to next command
            session.current_command = session.due_commands.get(session.current_index).cloned();
        }
    }
    Ok(())
}

/// Handle AbandonReviewSession message
///
/// Cancels the current review session
pub fn handle_abandon_review_session(state: &mut AppState) -> Result<(), UserError> {
    state.review_session = None;
    state.screen = Screen::MainMenu;
    Ok(())
}
