//! Game engine and session management.
//!
//! This module contains the core game logic, including scenario management,
//! user action tracking, and scoring.

use std::time::Duration;

use crate::helix::{ModeKind, Pos, SelectionBounds};

pub mod command_context;
mod error;
mod live_scenario;
mod recorded_action;
pub mod scenario_collection;
pub mod scenario_state;
pub mod scorer;
pub mod services;
pub mod session;

pub use command_context::format_key_for_display;
pub use error::GameError;
pub use live_scenario::LiveScenario;
pub use recorded_action::{RecordedAction, UserAction};
pub use scenario_collection::{
    CompletionFilter, Criterion, CurriculumStats, EmptyCriterion, NonEmptySet, PrerequisiteFilter,
    ScenarioCollection, ScenarioFilter, SortMode,
};
pub use scenario_state::ScenarioState;
pub use scorer::{PerformanceRating, Scorer};
pub use session::{
    Abandoned, Active, Completed, Feedback, FeedbackOutcome, GameSession, SessionAfterAction,
    SessionState,
};

/// Trait for types that represent a playable scenario session.
///
/// Both training mode (`GameSession<Active>`) and arcade mode (`ActiveMiniScenario`)
/// implement this trait, enabling shared rendering and state inspection logic.
/// An implementor names its [`LiveScenario`] and its own clock; every query
/// is answered from the live scenario, so the two modes cannot drift apart.
///
/// # Examples
///
/// ```
/// use helix_trainer::game::PlayableScenario;
///
/// fn render_editor<S: PlayableScenario>(session: &S) {
///     let current_content = session.current_content();
///     let cursor = session.current_cursor();
///     // ... render editor
/// }
/// ```
pub trait PlayableScenario {
    /// The live scenario behind this session.
    fn live(&self) -> &LiveScenario;

    /// Get elapsed time since scenario start
    fn elapsed(&self) -> Duration;

    /// Get current editor content as string
    fn current_content(&self) -> String {
        self.live().current_content()
    }

    /// Get target editor content as string
    fn target_content(&self) -> String {
        self.live().target_content()
    }

    /// Get current cursor position as a [`Pos`]
    fn current_cursor(&self) -> Pos {
        self.live().current_cursor()
    }

    /// Get target cursor position as a [`Pos`]
    fn target_cursor(&self) -> Pos {
        self.live().target_cursor()
    }

    /// Get current selection bounds (if any)
    fn current_selection(&self) -> Option<SelectionBounds> {
        self.live().current_selection()
    }

    /// Get target selection bounds (if any)
    fn target_selection(&self) -> Option<SelectionBounds> {
        self.live().target_selection()
    }

    /// Get the number of actions taken so far
    fn action_count(&self) -> usize {
        self.live().action_count()
    }

    /// Check if the session is in Insert mode
    fn is_insert_mode(&self) -> bool {
        self.live().is_insert_mode()
    }

    /// Get the scenario's effective content language, used to select syntax
    /// highlighting for the target panel.
    fn language(&self) -> crate::helix::Language {
        self.live().language()
    }

    /// Get the current editor mode for UI display
    fn mode_kind(&self) -> ModeKind {
        self.live().mode_kind()
    }

    /// Get all cursor positions for multi-cursor scenarios; the first is the primary cursor.
    fn all_cursors(&self) -> Vec<Pos> {
        self.live().all_cursors()
    }

    /// Get all selection bounds for multi-selection scenarios.
    fn all_selections(&self) -> Vec<SelectionBounds> {
        self.live().all_selections()
    }

    /// Get all target cursor positions for multi-cursor scenarios; the first is the primary cursor.
    fn all_target_cursors(&self) -> Vec<Pos> {
        self.live().all_target_cursors()
    }

    /// Get all target selection bounds for multi-selection scenarios.
    fn all_target_selections(&self) -> Vec<SelectionBounds> {
        self.live().all_target_selections()
    }
}
