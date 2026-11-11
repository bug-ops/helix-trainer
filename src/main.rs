//! Main entry point for the Helix Keybindings Trainer
//!
//! This is the application's entry point. It initializes the terminal UI,
//! loads scenarios asynchronously, and runs the async event loop.

use anyhow::Result;
use helix_trainer::{
    async_state::SaveWriterOutcome,
    config::{
        AppConfig, ConfigStorage, app_config::config_load_failed_message, data_dir::DataDir,
        keymap::resolve_startup_overlay,
    },
    data_loader::{spawn_data_loaders, spawn_save_writer},
    gamification::ProfileStorage,
    learning::PerformanceTracker,
    security::error_chain,
    ui::{self, AppState, Notification, NotificationType},
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
    let Some(data_dir) = DataDir::resolve() else {
        eprintln!(
            "error: cannot determine the configuration directory; \
             set HOME or XDG_CONFIG_HOME to an absolute path"
        );
        std::process::exit(1);
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
    let (app_config, config_startup_message) = match config_storage.load() {
        Ok(config) => (config, None),
        Err(e) => {
            tracing::warn!(error = %error_chain(&e), "Failed to load config, using defaults");
            (AppConfig::default(), Some(config_load_failed_message()))
        }
    };

    // Resolve the gameplay keymap overlay from the user's Helix config, if
    // `use_helix_keymap` is enabled. Degrades to the stock keymap
    // (identity overlay) on any failure - never blocks startup.
    let (keymap, keymap_report, keymap_startup_message) = resolve_startup_overlay(&app_config);

    // Initialize app state (empty, will be populated by async loaders)
    let profile_storage = ProfileStorage::in_data_dir(&data_dir);
    let tracker = PerformanceTracker::new();

    // Create ConfigState from loaded AppConfig
    let config_state = ui::state::ConfigState {
        persistent: app_config,
        keymap,
        keymap_report,
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

    for message in config_startup_message
        .into_iter()
        .chain(keymap_startup_message)
    {
        app_state
            .ui
            .notifications
            .push(Notification::new(NotificationType::Info { message }));
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
