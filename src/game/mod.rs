//! Game engine and session management.
//!
//! This module contains the core game logic, including scenario management,
//! user action tracking, and scoring.

use std::time::Duration;

pub mod command_context;
pub mod editor_state;
pub mod scorer;
pub mod session;

pub use command_context::{
    CommandBuffer, CommandContext, CommandExecutor, CommandInputResult, ParsedCommand,
    extract_count_and_command, parse_command_buffer, process_command_input,
};
pub use editor_state::{CursorPosition, EditorState, Selection};
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
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::PlayableScenario;
///
/// fn render_editor<S: PlayableScenario>(session: &S) {
///     let current = session.current_state();
///     let target = session.target_state();
///     // ... render both states
/// }
/// ```
pub trait PlayableScenario {
    /// Get reference to the current editor state
    fn current_state(&self) -> &EditorState;

    /// Get reference to the target editor state
    fn target_state(&self) -> &EditorState;

    /// Get the number of actions taken so far
    fn action_count(&self) -> usize;

    /// Check if the session is in Insert mode
    fn is_insert_mode(&self) -> bool;

    /// Get elapsed time since scenario start
    fn elapsed(&self) -> Duration;

    /// Check if current state matches target state
    fn is_completed(&self) -> bool {
        self.current_state() == self.target_state()
    }

    /// Get current editor mode as string for UI display
    fn mode_name(&self) -> &'static str {
        if self.is_insert_mode() {
            "INSERT"
        } else {
            "NORMAL"
        }
    }
}
