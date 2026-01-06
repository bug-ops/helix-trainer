//! Async event loop
//!
//! Core event loop using tokio::select! for non-blocking I/O.

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::sync::mpsc;

use helix_trainer::{
    async_state::DataLoadMessage,
    constants::{ANIMATION_TICK_INTERVAL, COUNTDOWN_TICK_INTERVAL, SUCCESS_SCREEN_DELAY},
    ui::{self, AppState, Message},
};

use crate::data_handling::handle_data_message;
use helix_trainer::input::handle_key_event;

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
            ui::update(state, Message::CompleteScenario)?;
            state.ui.completion_time = None;
        }

        // Select on multiple event sources (non-blocking)
        // Use biased to prioritize keyboard events for responsive UI
        tokio::select! {
            biased;

            // Terminal events (keyboard input) - highest priority
            maybe_event = event_stream.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    // Handle global quit shortcut first
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        tracing::debug!("User pressed Ctrl+C");
                        ui::update(state, Message::QuitApp)?;
                        continue;
                    }

                    // Dispatch to screen-specific handlers
                    if let Some(msg) = handle_key_event(key, state) {
                        tracing::debug!("Message: {:?}", msg);
                        ui::update(state, msg)?;
                    }
                }
            }

            // Data loading results
            Some(data_msg) = data_rx.recv() => {
                handle_data_message(state, data_msg)?;
            }

            // Countdown tick for mini-game (1 second)
            _ = countdown_tick_interval.tick(), if is_minigame_countdown(state) => {
                ui::update(state, Message::MiniGameTick)?;
            }

            // Fast tick for animations and mini-game timeout checking (100ms)
            _ = tick_interval.tick() => {
                // Check for mini-game timeout
                if is_minigame_playing(state) && is_minigame_timed_out(state) {
                    ui::update(state, Message::MiniGameTimeout)?;
                }
                // Check for mini-game transition auto-advance
                if should_minigame_advance(state) {
                    ui::update(state, Message::MiniGameNextScenario)?;
                }
                // Cleanup expired notifications
                ui::update(state, Message::CleanupNotifications)?;
            }
        }
    }

    Ok(())
}
