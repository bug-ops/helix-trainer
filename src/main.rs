//! Main entry point for the Helix Keybindings Trainer
//!
//! This is the application's entry point. It initializes the terminal UI,
//! loads scenarios asynchronously, and runs the async event loop.

use anyhow::Result;
use helix_trainer::{
    async_state::SaveWriterOutcome,
    config::{AppConfig, ConfigStorage, data_dir::DataDir},
    data_loader::{spawn_data_loaders, spawn_save_writer},
    gamification::ProfileStorage,
    input::keymap::resolve_startup_overlay,
    learning::PerformanceTracker,
    security::error_chain,
    ui::{self, AppState, Notification, NotificationType, keymap_status_notification},
};
use std::io;
use tokio::sync::mpsc;

mod cli;
mod data_handling;
mod event_loop;
#[cfg(test)]
#[path = "testing/log_capture.rs"]
mod log_capture;
mod logging;
mod terminal;

use event_loop::run_async_event_loop;
use logging::init_secure_logging;
use terminal::{install_panic_hook, setup_terminal};

/// Exit status when no data directory can be determined.
const NO_DATA_DIR_EXIT_CODE: i32 = 1;

/// Error text shown when no data directory can be determined.
const NO_DATA_DIR_MESSAGE: &str = "cannot determine the configuration directory; \
     set HOME or XDG_CONFIG_HOME to an absolute path";

/// Turns an unresolved data directory into the startup error message.
fn require_data_dir(resolved: Option<DataDir>) -> Result<DataDir, &'static str> {
    resolved.ok_or(NO_DATA_DIR_MESSAGE)
}

/// Loads the user configuration; on failure falls back to the defaults and returns the
/// notification text to show at startup.
fn load_config_or_default(storage: &ConfigStorage) -> (AppConfig, Option<NotificationType>) {
    match storage.load() {
        Ok(config) => (config, None),
        Err(e) => {
            tracing::warn!(error = %error_chain(&e), "Failed to load config, using defaults");
            (
                AppConfig::default(),
                Some(NotificationType::ConfigLoadFailed),
            )
        }
    }
}

/// Main entry point (async)
///
/// Uses 2 worker threads - sufficient for our async workload:
/// - Terminal event handling
/// - Background data loading (scenarios, profile)
/// - Tick interval for animations
#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    match cli::parse_args(std::env::args_os().skip(1)) {
        cli::CliAction::Run => {}
        cli::CliAction::Version => {
            println!("{}", cli::version_text());
            return Ok(());
        }
        cli::CliAction::Help => {
            println!("{}", cli::help_text());
            return Ok(());
        }
        cli::CliAction::Error(e) => {
            eprintln!("error: {e}");
            std::process::exit(cli::USAGE_EXIT_CODE);
        }
    }

    // Resolved before logging and the terminal exist so the message is readable.
    let data_dir = match require_data_dir(DataDir::resolve()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(NO_DATA_DIR_EXIT_CODE);
        }
    };

    // Warn if running debug build
    #[cfg(debug_assertions)]
    {
        eprintln!("WARNING: Running debug build. Not for production use!");
        eprintln!("Build with: cargo build --release");
    }

    // Initialize secure logging
    init_secure_logging()?;

    tracing::info!("Starting Helix Keybindings Trainer (async mode)");

    // Create channel for data loading messages
    let (data_tx, mut data_rx) = mpsc::channel(32);

    // Load application configuration
    let config_storage = ConfigStorage::in_data_dir(&data_dir);
    let (app_config, config_startup_message) = load_config_or_default(&config_storage);

    // Resolve the gameplay keymap overlay from the user's Helix config, if
    // `use_helix_keymap` is enabled. Degrades to the stock keymap
    // (identity overlay) on any failure - never blocks startup.
    let startup_keymap = resolve_startup_overlay(&app_config);
    let keymap_startup_notice = keymap_status_notification(&startup_keymap);

    // Initialize app state (empty, will be populated by async loaders)
    let profile_storage = ProfileStorage::in_data_dir(&data_dir);
    let tracker = PerformanceTracker::new();

    // Create ConfigState from loaded AppConfig
    let config_state = ui::state::ConfigState {
        persistent: app_config,
        keymap: startup_keymap.overlay,
        keymap_report: startup_keymap.report,
    };

    // The loader shares the app's handle (clones share the last known-valid profile text), so the
    // first save after startup keeps the profile that just loaded as `profile.json.prev`.
    let loader_storage = profile_storage.clone();

    let mut app_state = AppState::with_config(
        vec![],
        helix_trainer::gamification::UserProfile::new(),
        profile_storage,
        tracker,
        config_state,
    );

    for notice in config_startup_message
        .into_iter()
        .chain(keymap_startup_notice)
    {
        app_state.ui.notifications.push(Notification::new(notice));
    }

    // Every profile save (mid-session and exit-time) is funneled through
    // this single serialized writer rather than writing directly, so
    // writes apply strictly in the order they were requested — see
    // `spawn_save_writer` for why an unordered fire-and-forget task per
    // save is unsafe. Outcomes come back over `data_tx`/`data_rx` as
    // `DataLoadMessage::ProfileSaved`/`ProfileSaveError`.
    let (save_request_tx, save_writer_handle) = spawn_save_writer(data_tx.clone());
    app_state.set_save_channel(save_request_tx.clone());

    // Setup terminal
    let mut terminal = setup_terminal()?;
    install_panic_hook();

    tracing::debug!("Terminal initialized");

    app_state.progress.begin_profile_load();

    // Spawn background data loaders
    spawn_data_loaders(data_tx.clone(), loader_storage);

    // Force full redraw on first render by clearing the terminal
    // This ensures the UI is visible immediately on startup
    terminal.clear()?;
    terminal.draw(|f| ui::render(f, &app_state))?;
    // Flush to ensure immediate display
    io::Write::flush(terminal.backend_mut())?;

    if let Err(e) = ui::render::spawn_highlight_prewarm() {
        tracing::debug!("syntect pre-warm thread not started: {e}");
    }

    // Run the async event loop
    let result = run_async_event_loop(&mut terminal, &mut app_state, &mut data_rx, &data_tx).await;

    // Snapshot the exit-time save before anything else touches `progress`.
    let final_save_request = app_state.prepare_final_save_request();

    // Enqueue the exit save as the very last item, then drop every sender
    // clone and wait for the writer to fully drain before the runtime
    // shuts down. Since the event loop has already stopped, nothing else
    // enqueues a save after this point, so this snapshot — the most up to
    // date one there is — is guaranteed to be written last and win over
    // any mid-session save that was still queued or in flight.
    let exit_save_requested = final_save_request.is_some();
    if let Some(request) = final_save_request
        && let Err(e) = save_request_tx.send(request).await
    {
        tracing::error!("Failed to enqueue exit-time profile save: {}", e);
    }
    drop(save_request_tx);
    // Closing `app_state`'s clone explicitly (rather than relying only on
    // dropping `app_state` itself, which happens to also be unused past
    // this point today) keeps the writer-drain invariant local to the
    // channel regardless of how a future refactor reshuffles this
    // function.
    app_state.close_save_channel();
    // Nothing reads `data_rx` past this point (the event loop that drove it
    // has already returned); drop it so the writer's `result_tx.send` calls
    // fail fast on a full channel instead of risking a wait here while
    // `terminal.restore()` below is left stranded on raw mode.
    drop(data_rx);
    // `JoinHandle::await` resolving to `Ok` only means the writer task
    // didn't panic, not that the save it processed actually succeeded —
    // `SaveWriterOutcome` carries that real answer for the last request it
    // processed, which is this exit-time save (nothing else enqueues after
    // it).
    match save_writer_handle.await {
        Ok(SaveWriterOutcome::LastSaveSucceeded) => {
            tracing::info!("Profile saved successfully");
        }
        Ok(SaveWriterOutcome::LastSaveFailed(err)) => {
            tracing::error!(error = %error_chain(&*err), "Failed to save profile on exit");
        }
        Ok(SaveWriterOutcome::NoRequestsProcessed) if exit_save_requested => {
            tracing::error!("Profile save writer exited without processing the exit-time save");
        }
        Ok(SaveWriterOutcome::NoRequestsProcessed) => {
            tracing::info!("Exit-time profile save skipped (profile not loaded or load failed)");
        }
        Err(e) => {
            tracing::error!(error = %error_chain(&e), "Profile save writer task panicked");
        }
    }

    // Restore terminal
    terminal.restore()?;

    tracing::info!("Exiting Helix Keybindings Trainer");

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_data_dir_is_an_error_naming_both_variables() {
        let message = require_data_dir(None).unwrap_err();
        assert!(message.contains("HOME"), "{message}");
        assert!(message.contains("XDG_CONFIG_HOME"), "{message}");
        assert_eq!(NO_DATA_DIR_EXIT_CODE, 1);
    }

    #[test]
    fn relative_bases_do_not_resolve_a_data_dir() {
        let resolved = DataDir::resolve_for(
            false,
            None,
            Some(std::path::PathBuf::from("relative/.config")),
        );
        assert!(require_data_dir(resolved).is_err());
    }

    #[test]
    fn resolved_data_dir_passes_through() {
        let base = std::env::temp_dir();
        let resolved = DataDir::resolve_for(false, None, Some(base.clone()));
        assert_eq!(
            require_data_dir(resolved).unwrap().dir(),
            base.join("helix-trainer")
        );
    }

    #[test]
    fn corrupt_config_falls_back_to_defaults_with_a_notification() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{ not json").unwrap();

        let (config, message) = load_config_or_default(&ConfigStorage::with_path(&path));
        assert_eq!(message, Some(NotificationType::ConfigLoadFailed));
        assert_eq!(
            serde_json::to_value(&config).unwrap(),
            serde_json::to_value(AppConfig::default()).unwrap()
        );
    }

    #[test]
    fn missing_config_yields_defaults_without_a_notification() {
        let dir = tempfile::tempdir().unwrap();
        let (_, message) =
            load_config_or_default(&ConfigStorage::with_path(dir.path().join("config.json")));
        assert_eq!(message, None);
    }
}
