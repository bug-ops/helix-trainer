//! Async event loop
//!
//! Core event loop using `tokio::select`! for non-blocking I/O.

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::future::Future;
use std::io;
use tokio::sync::mpsc;
use tokio::task::JoinError;

use helix_trainer::{
    async_state::DataLoadMessage,
    constants::{ANIMATION_TICK_INTERVAL, COUNTDOWN_TICK_INTERVAL, SUCCESS_SCREEN_DELAY},
    error::UserError,
    gamification::ProfileStorage,
    security::error_chain,
    ui::{self, AppState, Message, Notification, NotificationType, Viewport, state::Effect},
};

use crate::data_handling::handle_data_message;
use helix_trainer::data_loader::{restore_previous_profile_message, start_fresh_profile_message};
use helix_trainer::ui::handle_key_event;

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
    tracing::warn!(error = %error_chain(error), "Update failed");
    state
        .ui
        .notifications
        .push(Notification::new(NotificationType::Info {
            message: error.to_string(),
        }));
}

/// Execute the effects an update requested; those that need the async runtime are returned.
fn run_effects(state: &mut AppState, effects: Vec<Effect>) -> Vec<Effect> {
    effects
        .into_iter()
        .filter_map(|effect| effect.execute(&mut state.progress))
        .collect()
}

/// Replace the quarantined profile in the background and report the outcome over `data_tx`
/// (#518 Start fresh, #531 Restore previous); `action` names the replacement in logs.
fn spawn_profile_replacement<Fut>(
    state: &AppState,
    data_tx: &mpsc::Sender<DataLoadMessage>,
    action: &'static str,
    run: fn(ProfileStorage) -> Fut,
) where
    Fut: Future<Output = Result<DataLoadMessage, JoinError>> + Send + 'static,
{
    let storage = state.progress.storage.clone();
    let data_tx = data_tx.clone();
    tokio::spawn(async move {
        match run(storage).await {
            Ok(msg) => {
                if let Err(e) = data_tx.send(msg).await {
                    tracing::error!(error = %error_chain(&e), action, "Failed to report the profile");
                }
            }
            Err(e) => {
                tracing::error!(error = %error_chain(&e), action, "Profile replacement task failed");
            }
        }
    });
}

/// Apply `msg` to `state`; recoverable errors are reported, never propagated.
///
/// The single boundary that owns the event loop's error policy (see
/// constitution section V): no `?` on `update()` inside the loop. The requested effects are
/// executed whether or not the update failed, so a save or sound queued before the failing
/// step is not lost.
fn apply_update(state: &mut AppState, msg: Message, data_tx: &mpsc::Sender<DataLoadMessage>) {
    let mut effects = Vec::new();
    let result = ui::update(state, msg, &mut effects);
    settle_update(state, result, effects, data_tx);
}

/// Execute the `effects` of an update and report its `result`, in that order.
fn settle_update(
    state: &mut AppState,
    result: Result<(), UserError>,
    effects: Vec<Effect>,
    data_tx: &mpsc::Sender<DataLoadMessage>,
) {
    for deferred in run_effects(state, effects) {
        match deferred {
            Effect::StartFreshProfile => {
                spawn_profile_replacement(
                    state,
                    data_tx,
                    "start fresh",
                    start_fresh_profile_message,
                );
            }
            Effect::RestoreProfile => spawn_profile_replacement(
                state,
                data_tx,
                "restore previous",
                restore_previous_profile_message,
            ),
            // Already executed by `Effect::execute`; listed so a new async variant fails to compile here
            Effect::Play(_) | Effect::SaveProfile => {}
        }
    }
    if let Err(error) = result {
        report_update_error(state, &error);
    }
}

/// Check if mini-game is in countdown state
fn is_minigame_countdown(state: &AppState) -> bool {
    state
        .minigame_session()
        .is_some_and(|s| s.state().is_countdown())
}

/// Check if mini-game is in playing state
fn is_minigame_playing(state: &AppState) -> bool {
    state
        .minigame_session()
        .is_some_and(|s| s.state().is_playing())
}

/// Check if current mini-game scenario has timed out
fn is_minigame_timed_out(state: &AppState) -> bool {
    state
        .minigame_session()
        .is_some_and(helix_trainer::minigame::MiniGameSession::is_timed_out)
}

/// Check if the mini-game's session-level time limit has elapsed (e.g. Arcade's 60 seconds)
///
/// Checked across both `Playing` and `Transition` (not just `Playing`, unlike
/// [`is_minigame_timed_out`]): the session clock keeps running during the brief
/// between-scenario transition, so gating this on `Playing` alone would let a
/// run overrun the limit by up to the transition delay before the check fires
/// again on the next scenario.
fn is_minigame_session_expired(state: &AppState) -> bool {
    state.minigame_session().is_some_and(|s| {
        let playing_or_transitioning = s.state().is_playing() || s.state().is_transition();
        playing_or_transitioning && s.is_session_expired()
    })
}

/// Check if mini-game should auto-advance to next scenario
fn should_minigame_advance(state: &AppState) -> bool {
    state
        .minigame_session()
        .is_some_and(helix_trainer::minigame::MiniGameSession::should_advance_to_next)
}

/// Async event loop using `tokio::select`!
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
    data_tx: &mpsc::Sender<DataLoadMessage>,
) -> Result<()> {
    // Create event stream from crossterm
    let mut event_stream = EventStream::new();

    // Tick interval for animations and mini-game
    let mut tick_interval = tokio::time::interval(ANIMATION_TICK_INTERVAL);

    // Countdown tick interval for mini-game
    let mut countdown_tick_interval = tokio::time::interval(COUNTDOWN_TICK_INTERVAL);

    let mut consecutive_input_errors = 0u32;

    let size = terminal.size()?;
    apply_update(
        state,
        Message::Resize(Viewport::new(size.width, size.height)),
        data_tx,
    );

    loop {
        // Render the current state
        terminal.draw(|f| ui::render(f, state))?;

        // Check if we should exit
        if !state.ui.running {
            break;
        }

        // Check if scenario completed and delay elapsed
        if state.screen.completed_task().is_some_and(|task| {
            state
                .progress
                .monotonic_now()
                .duration_since(task.completed_at)
                >= SUCCESS_SCREEN_DELAY
        }) {
            tracing::debug!("Success screen delay elapsed, transitioning to results");
            apply_update(state, Message::CompleteScenario, data_tx);
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
                        // Dispatch to screen-specific handlers
                        if let Some(msg) = handle_key_event(key, state) {
                            tracing::debug!("Message: {:?}", msg);
                            apply_update(state, msg, data_tx);
                        }
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        consecutive_input_errors = 0;
                        apply_update(state, Message::Resize(Viewport::new(cols, rows)), data_tx);
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
                handle_data_message(state, data_msg);
                state.normalize_view();
            }

            // Countdown tick for mini-game (1 second)
            _ = countdown_tick_interval.tick(), if is_minigame_countdown(state) => {
                apply_update(state, Message::MiniGameTick, data_tx);
            }

            // Fast tick for animations and mini-game timeout checking (100ms)
            _ = tick_interval.tick() => {
                // Check for mini-game session-level expiry (e.g. Arcade's 60s limit) first —
                // it takes priority over a per-scenario timeout landing on the same tick.
                // Not gated on is_minigame_playing: it must also fire during the
                // between-scenario Transition state (see is_minigame_session_expired).
                if is_minigame_session_expired(state) {
                    apply_update(state, Message::MiniGameSessionTimeout, data_tx);
                } else if is_minigame_playing(state) && is_minigame_timed_out(state) {
                    apply_update(state, Message::MiniGameTimeout, data_tx);
                }
                // Check for mini-game transition auto-advance
                if should_minigame_advance(state) {
                    apply_update(state, Message::MiniGameNextScenario, data_tx);
                }
                // Cleanup expired notifications
                apply_update(state, Message::CleanupNotifications, data_tx);
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
    use std::assert_matches;

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

        let (data_tx, _data_rx) = mpsc::channel(1);
        apply_update(&mut state, Message::RetryScenario, &data_tx);

        assert!(state.ui.running);
        assert_eq!(state.ui.notifications.count(), 0);
    }

    /// Regression test for #481: effects queued before a failing step are still executed.
    #[test]
    fn test_effects_run_even_when_the_update_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.json");
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::with_path(&path),
            PerformanceTracker::new(),
        );
        state.progress.mark_profile_loaded();
        let (data_tx, _data_rx) = mpsc::channel(1);

        settle_update(
            &mut state,
            Err(UserError::WrongScreen),
            vec![Effect::SaveProfile],
            &data_tx,
        );

        assert!(path.exists(), "the queued save must run despite the error");
    }

    /// #518: the start-fresh effect runs the load on the runtime and reports its result.
    #[tokio::test]
    async fn test_start_fresh_effect_runs_the_load_and_sends_the_result() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.json");
        std::fs::write(&path, "{broken").unwrap();
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::with_path(&path),
            PerformanceTracker::new(),
        );
        let (data_tx, mut data_rx) = mpsc::channel(1);

        settle_update(
            &mut state,
            Ok(()),
            vec![Effect::StartFreshProfile],
            &data_tx,
        );

        let msg = data_rx.recv().await.expect("a result message");
        assert_matches!(msg, DataLoadMessage::ProfileReady(p) if p.level == 1);
        assert!(std::fs::read_to_string(&path).unwrap().contains("profile"));
        let backups = backup_files(dir.path());
        assert_eq!(
            backups.len(),
            1,
            "the unreadable file must be kept as a backup"
        );
        assert_eq!(std::fs::read_to_string(&backups[0]).unwrap(), "{broken");
    }

    fn backup_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.to_string_lossy().contains(".corrupt-"))
            .collect()
    }

    /// #531, #533: the restore effect runs on the runtime, keeps the backup and reports the
    /// validated previous copy.
    #[tokio::test]
    async fn test_restore_effect_runs_the_restore_and_sends_the_previous_profile() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
        let mut earlier = UserProfile::new();
        earlier.add_xp(500);
        storage.save(&earlier).unwrap();
        storage.save(&UserProfile::new()).unwrap();
        std::fs::write(storage.path(), "{broken").unwrap();
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            storage.clone(),
            PerformanceTracker::new(),
        );
        let (data_tx, mut data_rx) = mpsc::channel(1);

        settle_update(&mut state, Ok(()), vec![Effect::RestoreProfile], &data_tx);

        let msg = data_rx.recv().await.expect("a result message");
        assert_matches!(msg, DataLoadMessage::ProfileReady(p) if p.total_xp == 500);
        assert_eq!(backup_files(dir.path()).len(), 1);
    }

    /// #531: a previous copy that went bad after the offer is refused and reported, nothing
    /// is rewritten.
    #[tokio::test]
    async fn test_restore_effect_refuses_an_invalid_previous_copy() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
        storage.save(&UserProfile::new()).unwrap();
        storage.save(&UserProfile::new()).unwrap();
        std::fs::write(storage.path(), "{broken").unwrap();
        let prev = dir.path().join("profile.json.prev");
        std::fs::write(&prev, "{also broken").unwrap();
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            storage.clone(),
            PerformanceTracker::new(),
        );
        let (data_tx, mut data_rx) = mpsc::channel(1);

        settle_update(&mut state, Ok(()), vec![Effect::RestoreProfile], &data_tx);

        let msg = data_rx.recv().await.expect("a result message");
        assert_matches!(msg, DataLoadMessage::ProfileError { .. }, "{msg:?}");
        assert_eq!(std::fs::read_to_string(storage.path()).unwrap(), "{broken");
        assert_eq!(std::fs::read_to_string(&prev).unwrap(), "{also broken");
    }

    /// Effects that need no runtime never spawn a load.
    #[tokio::test]
    async fn test_sync_effects_do_not_spawn_a_load() {
        let mut state = test_state();
        let (data_tx, mut data_rx) = mpsc::channel(1);

        settle_update(&mut state, Ok(()), vec![Effect::SaveProfile], &data_tx);
        drop(data_tx);

        assert!(data_rx.recv().await.is_none());
    }

    /// Regression test for #466: any other recoverable error becomes a
    /// notification and never stops the session.
    #[test]
    fn test_recoverable_error_is_reported_and_session_continues() {
        let mut state = test_state();

        report_update_error(&mut state, &UserError::operation_failed());

        assert!(state.ui.running);
        assert_eq!(state.ui.notifications.count(), 1);
    }

    /// FR-094: the update-error sink logs the whole cause chain of a storage error, while
    /// the notification stays free of paths and OS messages.
    #[test]
    fn test_storage_error_is_logged_with_its_cause_and_shown_sanitized() {
        use helix_trainer::config::json_store::{Document, StorageError};

        let mut state = test_state();
        let cause = serde_json::from_str::<u8>("x").unwrap_err();
        let cause_text = cause.to_string();
        let error = UserError::from(StorageError::Corrupt {
            doc: Document::Profile,
            path: "/secret/dir/profile.json".into(),
            source: cause,
        });

        let logs = crate::log_capture::capture_logs(|| report_update_error(&mut state, &error));

        assert!(logs.contains(&cause_text), "{logs}");
        let shown = state.ui.notifications.visible();
        let message = shown
            .map(|n| format!("{:?}", n.notification_type))
            .collect::<Vec<_>>()
            .concat();
        assert!(!message.contains("/secret"), "{message}");
    }
}
