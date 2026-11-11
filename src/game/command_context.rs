//! Shared command execution context for different gameplay modes
//!
//! This module provides a trait-based abstraction for command execution
//! that unifies logic between training mode and arcade mode, reducing
//! code duplication while maintaining type safety.
//!
//! # Key Traits
//!
//! - [`CommandExecutor`]: Execute commands with count prefix support

use crate::helix::commands::*;
use crate::security::UserError;

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings.
/// This is used by both training and arcade modes for consistent
/// key history display.
///
/// # Examples
///
/// ```
/// use helix_trainer::game::format_key_for_display;
///
/// assert_eq!(format_key_for_display("Left"), "←");
/// assert_eq!(format_key_for_display("Backspace"), "⌫");
/// assert_eq!(format_key_for_display("x"), "x");
/// ```
pub fn format_key_for_display(command: &str) -> String {
    match command {
        CMD_ARROW_LEFT => "←".to_string(),
        CMD_ARROW_RIGHT => "→".to_string(),
        CMD_ARROW_UP => "↑".to_string(),
        CMD_ARROW_DOWN => "↓".to_string(),
        CMD_BACKSPACE => "⌫".to_string(),
        CMD_ESCAPE => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
    }
}

/// Trait for executing commands, including count-prefixed ones
///
/// Provides unified command execution for both training and arcade modes.
/// Implementations forward the command string to their simulator, which is the
/// only place a count prefix (`3d`, `2G`, `3fa`) is interpreted.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::CommandExecutor;
///
/// fn run<E: CommandExecutor>(executor: &mut E) -> Result<(), UserError> {
///     // "3d" is one action; the simulator applies the count
///     executor.execute_single("3d")
/// }
/// ```
pub trait CommandExecutor {
    /// Execute one command string, possibly with a count prefix
    ///
    /// The whole string (e.g. `"3d"`) is passed to the simulator unchanged, so
    /// the count is applied according to the command's `CountBehavior`.
    /// This counts as ONE action for scoring purposes.
    fn execute_single(&mut self, command: &str) -> Result<(), UserError>;

    /// Check if scenario is completed after command execution
    fn check_completion(&self) -> bool;
}
