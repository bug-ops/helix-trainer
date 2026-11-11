//! Terminal setup and cleanup utilities
//!
//! Provides safe terminal initialization and restoration for TUI applications.

use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    ops::{Deref, DerefMut},
    panic,
};

/// Owns the initialized terminal and restores it when dropped.
///
/// Guarantees restoration on early `?` returns between setup and the normal
/// exit path. Call [`TerminalGuard::restore`] on the normal path to surface
/// restore errors; the `Drop` fallback only logs them.
#[derive(Debug)]
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    restored: bool,
}

impl TerminalGuard {
    /// Restore the terminal now and disarm the `Drop` fallback.
    pub fn restore(mut self) -> Result<()> {
        self.restored = true;
        restore_terminal_state()
    }
}

impl Deref for TerminalGuard {
    type Target = Terminal<CrosstermBackend<io::Stdout>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for TerminalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.restored
            && let Err(e) = restore_terminal_state()
        {
            tracing::error!("Failed to restore terminal on drop: {}", e);
        }
    }
}

/// Initialize terminal for TUI rendering
///
/// Enables raw mode, enters alternate screen, hides cursor, and
/// enables keyboard enhancement protocol for proper Alt key handling.
///
/// # Returns
///
/// A [`TerminalGuard`] that restores the terminal when dropped.
pub fn setup_terminal() -> Result<TerminalGuard> {
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

    Ok(TerminalGuard {
        terminal,
        restored: false,
    })
}

/// Undo everything [`setup_terminal`] did, on a fresh stdout handle.
///
/// The single restore implementation shared by the normal exit path, the
/// [`TerminalGuard`] drop and the panic hook. Idempotent: running it on an
/// already-restored terminal is harmless.
fn restore_terminal_state() -> Result<()> {
    let mut stdout = io::stdout();
    // Ignore errors if the keyboard enhancement protocol is not supported
    let _ = execute!(stdout, PopKeyboardEnhancementFlags);

    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, Show)?;

    Ok(())
}

/// Install a panic hook that restores the terminal before the previous hook
/// runs.
///
/// Needed because release builds use `panic = "abort"`: no destructor runs,
/// so without the hook a panic leaves the shell in raw mode on the alternate
/// screen. Call right after [`setup_terminal`].
pub fn install_panic_hook() {
    chain_panic_hook(|| {
        let _ = restore_terminal_state();
    });
}

fn chain_panic_hook(restore: impl Fn() + Send + Sync + 'static) {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use std::{
        sync::Mutex,
        thread::{self, ThreadId},
    };

    use super::*;

    static EVENTS: Mutex<Vec<(ThreadId, &'static str)>> = Mutex::new(Vec::new());

    fn record(event: &'static str) {
        EVENTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((thread::current().id(), event));
    }

    // Events are keyed by thread and the hooks never panic, so concurrent
    // tests panicking under  cannot break this one.
    #[test]
    fn panic_hook_restores_then_chains_to_previous_hook() {
        panic::set_hook(Box::new(|_| record("previous")));
        chain_panic_hook(|| record("restore"));

        let result = panic::catch_unwind(|| panic!("boom"));
        let _ = panic::take_hook();

        let mine: Vec<_> = EVENTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|(id, _)| *id == thread::current().id())
            .map(|(_, event)| *event)
            .collect();
        assert!(result.is_err());
        assert_eq!(mine, ["restore", "previous"]);
    }
}
