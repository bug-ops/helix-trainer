//! Async event loop
//!
//! Core event loop using tokio::select! for non-blocking I/O.

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::sync::mpsc;

use helix_trainer::{
    async_state::DataLoadMessage,
    constants::{ANIMATION_TICK_INTERVAL, COUNTDOWN_TICK_INTERVAL, SUCCESS_SCREEN_DELAY},
    security::UserError,
    ui::{self, AppState, Message, Notification, NotificationType},
};

use crate::data_handling::handle_data_message;
use helix_trainer::input::handle_key_event;

/// Consecutive terminal input errors tolerated before the event loop gives up.
///
/// crossterm yields an error on every poll while the input source is failing,
/// so an unbounded retry would spin; a transient glitch recovers well within
/// this many attempts.
const MAX_CONSECUTIVE_INPUT_ERRORS: u32 = 5;

/// Report a recoverable error without ending the session.
///
/// A message for a screen other than the current one is a stale-timer race,
/// not a user-facing fault, so it is a debug-level no-op. Anything else is
/// logged and shown as a notification.
fn report_update_error(state: &mut AppState, error: &UserError) {
    if matches!(error, UserError::WrongScreen) {
        tracing::debug!(
            current = state.screen.screen_type(),
            "Ignoring message for a different screen"
        );
        return;
    }
    tracing::warn!(error = %error, "Update failed");
    state
        .ui
        .notifications
        .push(Notification::new(NotificationType::Info {
            message: error.to_string(),
        }));
}

/// Apply `msg` to `state`; recoverable errors are reported, never propagated.
///
/// The single boundary that owns the event loop's error policy (see
/// constitution section V): no `?` on `update()` inside the loop.
fn apply_update(state: &mut AppState, msg: Message) {
    if let Err(error) = ui::update(state, msg) {
        report_update_error(state, &error);
    }
}

/// Apply a background data message; failures are reported, never propagated.
fn apply_data_message(state: &mut AppState, msg: DataLoadMessage) {
    if let Err(error) = handle_data_message(state, msg) {
        tracing::warn!(error = %error, "Handling data message failed");
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::Info {
                message: error.to_string(),
            }));
    }
}

/// Check if mini-game is in countdown state
fn is_minigame_countdown(state: &AppState) -> bool {
    state
        .game
        .minigame_session
        .as_ref()
        .map(|s| s.state().is_countdown())
        .unwrap_or(false)
}

/// Check if mini-game is in playing state
fn is_minigame_playing(state: &AppState) -> bool {
    state
        .game
        .minigame_session
        .as_ref()
        .map(|s| s.state().is_playing())
        .unwrap_or(false)
}

/// Check if current mini-game scenario has timed out
fn is_minigame_timed_out(state: &AppState) -> bool {
    state
        .game
        .minigame_session
        .as_ref()
        .map(|s| s.is_timed_out())
        .unwrap_or(false)
}

/// Check if the mini-game's session-level time limit has elapsed (e.g. Arcade's 60 seconds)
///
/// Checked across both `Playing` and `Transition` (not just `Playing`, unlike
/// [`is_minigame_timed_out`]): the session clock keeps running during the brief
/// between-scenario transition, so gating this on `Playing` alone would let a
/// run overrun the limit by up to the transition delay before the check fires
/// again on the next scenario.
fn is_minigame_session_expired(state: &AppState) -> bool {
    state
        .game
        .minigame_session
        .as_ref()
        .map(|s| {
            let playing_or_transitioning = s.state().is_playing() || s.state().is_transition();
            playing_or_transitioning && s.is_session_expired()
        })
        .unwrap_or(false)
}

/// Check if mini-game should auto-advance to next scenario
fn should_minigame_advance(state: &AppState) -> bool {
    state
        .game
        .minigame_session
        .as_ref()
        .map(|s| s.should_advance_to_next())
        .unwrap_or(false)
}

/// Async event loop using tokio::select!
///
/// This function runs the core event loop that:
/// 1. Renders the current state
/// 2. Handles user input (async)
/// 3. Handles background data loading results
/// 4. Updates state based on messages
/// 5. Repeats until the app exits
///
/// Recoverable errors from state updates are reported and the loop keeps
/// running (see [`apply_update`]).
///
/// # Errors
///
/// Returns an error only on unrecoverable terminal failure: a failed draw, a
/// closed input stream, or [`MAX_CONSECUTIVE_INPUT_ERRORS`] input errors in a
/// row.
pub async fn run_async_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    data_rx: &mut mpsc::Receiver<DataLoadMessage>,
) -> Result<()> {
    // Create event stream from crossterm
    let mut event_stream = EventStream::new();

    // Tick interval for animations and mini-game
    let mut tick_interval = tokio::time::interval(ANIMATION_TICK_INTERVAL);

    // Countdown tick interval for mini-game
    let mut countdown_tick_interval = tokio::time::interval(COUNTDOWN_TICK_INTERVAL);

    let mut consecutive_input_errors = 0u32;

    loop {
        // Render the current state
        terminal.draw(|f| ui::render(f, state))?;

        // Check if we should exit
        if !state.ui.running {
            break;
        }

        // Check if scenario completed and delay elapsed
        if let Some(completion_time) = state.ui.completion_time
            && completion_time.elapsed() >= SUCCESS_SCREEN_DELAY
        {
            tracing::debug!("Success screen delay elapsed, transitioning to results");
            apply_update(state, Message::CompleteScenario);
            state.ui.completion_time = None;
        }

        // Select on multiple event sources (non-blocking)
        // Use biased to prioritize keyboard events for responsive UI
        tokio::select! {
            biased;

            // Terminal events (keyboard input) - highest priority
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        consecutive_input_errors = 0;
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        // Handle global quit shortcut first
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            tracing::debug!("User pressed Ctrl+C");
                            apply_update(state, Message::QuitApp);
                            continue;
                        }

                        // Dispatch to screen-specific handlers
                        if let Some(msg) = handle_key_event(key, state) {
                            tracing::debug!("Message: {:?}", msg);
                            apply_update(state, msg);
                        }
                    }
                    Some(Ok(_)) => consecutive_input_errors = 0,
                    Some(Err(error)) => {
                        consecutive_input_errors += 1;
                        tracing::error!(
                            error = %error,
                            consecutive = consecutive_input_errors,
                            "Terminal input error"
                        );
                        if consecutive_input_errors >= MAX_CONSECUTIVE_INPUT_ERRORS {
                            return Err(anyhow::anyhow!(
                                "terminal input failed {consecutive_input_errors} times in a row: {error}"
                            ));
                        }
                    }
                    None => return Err(anyhow::anyhow!("terminal input stream closed")),
                }
            }

            // Data loading results
            Some(data_msg) = data_rx.recv() => {
                apply_data_message(state, data_msg);
            }

            // Countdown tick for mini-game (1 second)
            _ = countdown_tick_interval.tick(), if is_minigame_countdown(state) => {
                apply_update(state, Message::MiniGameTick);
            }

            // Fast tick for animations and mini-game timeout checking (100ms)
            _ = tick_interval.tick() => {
                // Check for mini-game session-level expiry (e.g. Arcade's 60s limit) first —
                // it takes priority over a per-scenario timeout landing on the same tick.
                // Not gated on is_minigame_playing: it must also fire during the
                // between-scenario Transition state (see is_minigame_session_expired).
                if is_minigame_session_expired(state) {
                    apply_update(state, Message::MiniGameSessionTimeout);
                } else if is_minigame_playing(state) && is_minigame_timed_out(state) {
                    apply_update(state, Message::MiniGameTimeout);
                }
                // Check for mini-game transition auto-advance
                if should_minigame_advance(state) {
                    apply_update(state, Message::MiniGameNextScenario);
                }
                // Cleanup expired notifications
                apply_update(state, Message::CleanupNotifications);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_trainer::{
        gamification::{ProfileStorage, UserProfile},
        learning::PerformanceTracker,
    };

    fn test_state() -> AppState {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
        AppState::new(
            vec![],
            UserProfile::new(),
            storage,
            PerformanceTracker::new(),
        )
    }

    /// Regression test for #466: a message for another screen is a silent
    /// no-op; the session keeps running and no notification is raised.
    #[test]
    fn test_wrong_screen_message_is_silent_noop() {
        let mut state = test_state();

        apply_update(&mut state, Message::RetryScenario);

        assert!(state.ui.running);
        assert_eq!(state.ui.notifications.count(), 0);
    }

    /// Regression test for #466: any other recoverable error becomes a
    /// notification and never stops the session.
    #[test]
    fn test_recoverable_error_is_reported_and_session_continues() {
        let mut state = test_state();

        report_update_error(&mut state, &UserError::OperationFailed);

        assert!(state.ui.running);
        assert_eq!(state.ui.notifications.count(), 1);
    }
}
