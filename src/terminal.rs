//! Terminal setup and cleanup utilities
//!
//! Provides safe terminal initialization and restoration for TUI applications.

use anyhow::Result;
use crossterm::{
    cursor::Hide,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

/// Initialize terminal for TUI rendering
///
/// Enables raw mode, enters alternate screen, and hides cursor.
///
/// # Returns
///
/// A configured Terminal instance ready for rendering.
pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    tracing::debug!("Terminal initialized");

    Ok(terminal)
}

/// Restore terminal to normal state
///
/// Disables raw mode, leaves alternate screen, and shows cursor.
///
/// # Safety
///
/// Always call this before exiting, even on error paths.
/// Use defer pattern or ensure it's called in cleanup code.
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
