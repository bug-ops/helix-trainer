//! Game engine and session management.
//!
//! This module contains the core game logic, including scenario management,
//! user action tracking, and scoring.

use std::time::Duration;

use crate::helix::SelectionBounds;

pub mod command_context;
pub mod editor_state;
pub mod scenario_state;
pub mod scorer;
pub mod services;
pub mod session;

pub use command_context::{
    CommandBuffer, CommandContext, CommandExecutor, CommandInputResult, ParsedCommand,
    extract_count_and_command, format_key_for_display, parse_command_buffer, process_command_input,
};
pub use editor_state::EditorState;
pub use scenario_state::ScenarioState;
pub use scorer::{PerformanceRating, Scorer};
pub use session::{
    Abandoned, Active, Completed, Feedback, GameSession, SessionAfterAction, SessionState,
    UserAction,
};

/// Trait for types that represent a playable scenario session.
///
/// Both training mode (`GameSession<Active>`) and arcade mode (`ActiveMiniScenario`)
/// implement this trait, enabling shared rendering and state inspection logic.
///
/// This trait uses primitive types (strings, tuples) instead of EditorState
/// to minimize coupling with legacy types. Implementations typically use
/// the simulator's EditorDisplay facade internally.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::PlayableScenario;
///
/// fn render_editor<S: PlayableScenario>(session: &S) {
///     let current_content = session.current_content();
///     let (cursor_row, cursor_col) = session.current_cursor();
///     // ... render editor
/// }
/// ```
pub trait PlayableScenario {
    /// Get current editor content as string
    fn current_content(&self) -> String;

    /// Get target editor content as string
    fn target_content(&self) -> String;

    /// Get current cursor position as (row, col)
    fn current_cursor(&self) -> (usize, usize);

    /// Get target cursor position as (row, col)
    fn target_cursor(&self) -> (usize, usize);

    /// Get current selection bounds (if any)
    fn current_selection(&self) -> Option<SelectionBounds>;

    /// Get target selection bounds (if any)
    fn target_selection(&self) -> Option<SelectionBounds>;

    /// Get the number of actions taken so far
    fn action_count(&self) -> usize;

    /// Check if the session is in Insert mode
    fn is_insert_mode(&self) -> bool;

    /// Get elapsed time since scenario start
    fn elapsed(&self) -> Duration;

    /// Get the scenario's effective content language as a file-extension-style token
    /// (e.g. `"rs"`, `"md"`), used to select syntax highlighting for the target panel.
    ///
    /// Default implementation returns [`crate::config::DEFAULT_LANGUAGE`], matching the
    /// highlighter's historical hardcoded behavior; implementors backed by a `Scenario`
    /// should resolve this from `Setup.language`.
    fn language(&self) -> &str {
        crate::config::DEFAULT_LANGUAGE
    }

    /// Get current editor mode as string for UI display
    fn mode_name(&self) -> &'static str {
        if self.is_insert_mode() {
            "INSERT"
        } else {
            "NORMAL"
        }
    }

    /// Get all cursor positions for multi-cursor scenarios.
    ///
    /// Returns a vector of (row, col) pairs. The first cursor is the primary cursor.
    /// Default implementation returns only the primary cursor.
    fn all_cursors(&self) -> Vec<(usize, usize)> {
        vec![self.current_cursor()]
    }

    /// Get all selection bounds for multi-selection scenarios.
    ///
    /// Returns a vector of SelectionBounds. Default implementation
    /// returns the primary selection if any.
    fn all_selections(&self) -> Vec<SelectionBounds> {
        self.current_selection().into_iter().collect()
    }

    /// Get all target cursor positions for multi-cursor scenarios.
    ///
    /// Returns a vector of (row, col) pairs. The first cursor is the primary cursor.
    /// Default implementation returns only the primary target cursor.
    fn all_target_cursors(&self) -> Vec<(usize, usize)> {
        vec![self.target_cursor()]
    }

    /// Get all target selection bounds for multi-selection scenarios.
    ///
    /// Returns a vector of SelectionBounds. Default implementation
    /// returns the primary target selection if any.
    fn all_target_selections(&self) -> Vec<SelectionBounds> {
        self.target_selection().into_iter().collect()
    }
}
