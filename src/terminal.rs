//! Terminal setup and cleanup utilities
//!
//! Provides safe terminal initialization and restoration for TUI applications.

use anyhow::Result;
use crossterm::{
    cursor::Hide,
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

/// Initialize terminal for TUI rendering
///
/// Enables raw mode, enters alternate screen, hides cursor, and
/// enables keyboard enhancement protocol for proper Alt key handling.
///
/// # Returns
///
/// A configured Terminal instance ready for rendering.
pub fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)?;

    // Try to enable keyboard enhancement protocol (like Helix does)
    // This makes terminals send Alt modifier instead of composed Unicode chars
    // Ignore errors for terminals that don't support it
    match execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        )
    ) {
        Ok(()) => tracing::debug!("Keyboard enhancement protocol enabled"),
        Err(e) => tracing::debug!("Keyboard enhancement not available: {}", e),
    }

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    tracing::debug!("Terminal initialized");

    Ok(terminal)
}

/// Restore terminal to normal state
///
/// Disables raw mode, leaves alternate screen, shows cursor, and
/// pops keyboard enhancement flags if they were enabled.
///
/// # Safety
///
/// Always call this before exiting, even on error paths.
/// Use defer pattern or ensure it's called in cleanup code.
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    // Pop keyboard enhancement flags (ignore errors if not supported)
    let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
