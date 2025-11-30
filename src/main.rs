//! Main entry point for the Helix Keybindings Trainer
//!
//! This is the application's entry point. It initializes the terminal UI,
//! loads scenarios asynchronously, and runs the async event loop.

use anyhow::Result;
use helix_trainer::{
    data_loader::spawn_data_loaders,
    gamification::ProfileStorage,
    learning::PerformanceTracker,
    ui::{self, AppState},
};
use std::io;
use tokio::sync::mpsc;

mod data_handling;
mod event_loop;
mod input;
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

    // Initialize app state (empty, will be populated by async loaders)
    let profile_storage = ProfileStorage::new();
    let tracker = PerformanceTracker::new();
    let mut app_state = AppState::new(
        vec![],
        helix_trainer::gamification::UserProfile::new(),
        profile_storage,
        tracker,
    );

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

    // Save profile before exit
    if let Err(e) = app_state.save_profile_immediate() {
        tracing::error!("Failed to save profile on exit: {}", e);
    } else {
        tracing::info!("Profile saved successfully");
    }

    // Restore terminal
    restore_terminal(&mut terminal)?;

    tracing::info!("Exiting Helix Keybindings Trainer");

    result
}
