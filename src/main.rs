#![forbid(unsafe_code)]

//! Main entry point for the Helix Keybindings Trainer
//!
//! This is the application's entry point. It initializes the terminal UI,
//! loads scenarios asynchronously, and runs the async event loop.

use anyhow::Result;
use helix_trainer::{
    async_state::SaveWriterOutcome,
    config::{AppConfig, ConfigStorage},
    data_loader::{spawn_data_loaders, spawn_save_writer},
    gamification::ProfileStorage,
    learning::PerformanceTracker,
    ui::{self, AppState},
};
use std::io;
use tokio::sync::mpsc;

mod data_handling;
mod event_loop;
mod logging;
mod terminal;

use event_loop::run_async_event_loop;
use logging::init_secure_logging;
use terminal::{restore_terminal, setup_terminal};

/// Main entry point (async)
///
/// Uses 2 worker threads - sufficient for our async workload:
/// - Terminal event handling
/// - Background data loading (scenarios, profile)
/// - Tick interval for animations
#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
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
    let config_storage = ConfigStorage::new();
    let app_config = config_storage.load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });

    // Initialize app state (empty, will be populated by async loaders)
    let profile_storage = ProfileStorage::new();
    let tracker = PerformanceTracker::new();

    // Create ConfigState from loaded AppConfig
    let config_state = ui::state::ConfigState {
        persistent: app_config,
        ..ui::state::ConfigState::default()
    };

    let mut app_state = AppState::with_config(
        vec![],
        helix_trainer::gamification::UserProfile::new(),
        profile_storage,
        tracker,
        config_state,
    );

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

    tracing::debug!("Terminal initialized");

    // Spawn background data loaders
    spawn_data_loaders(data_tx);

    // Force full redraw on first render by clearing the terminal
    // This ensures the UI is visible immediately on startup
    terminal.clear()?;
    terminal.draw(|f| ui::render(f, &mut app_state))?;
    // Flush to ensure immediate display
    io::Write::flush(terminal.backend_mut())?;

    // Run the async event loop
    let result = run_async_event_loop(&mut terminal, &mut app_state, &mut data_rx).await;

    // Snapshot the exit-time save before anything else touches `progress`.
    let final_save_request = app_state.prepare_final_save_request();

    // Save configuration before exit (only if modified)
    if app_state.config.config_modified {
        if let Err(e) = config_storage.save(&app_state.config.persistent) {
            tracing::error!("Failed to save config on exit: {:?}", e);
        } else {
            tracing::info!("Config saved successfully");
        }
    }

    // Enqueue the exit save as the very last item, then drop every sender
    // clone and wait for the writer to fully drain before the runtime
    // shuts down. Since the event loop has already stopped, nothing else
    // enqueues a save after this point, so this snapshot — the most up to
    // date one there is — is guaranteed to be written last and win over
    // any mid-session save that was still queued or in flight.
    if let Err(e) = save_request_tx.send(final_save_request).await {
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
    // `restore_terminal` below is left stranded on raw mode.
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
            tracing::error!("Failed to save profile on exit: {}", err);
        }
        Ok(SaveWriterOutcome::NoRequestsProcessed) => {
            tracing::error!("Profile save writer exited without processing the exit-time save");
        }
        Err(e) => tracing::error!("Profile save writer task panicked: {}", e),
    }

    // Restore terminal
    restore_terminal(&mut terminal)?;

    tracing::info!("Exiting Helix Keybindings Trainer");

    result
}
