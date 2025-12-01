//! Type-safe handler infrastructure
//!
//! Provides compile-time guarantees that handlers receive the correct
//! screen data type, eliminating runtime type checks.

use crate::ui::state::{ConfigState, GameState, ProgressState, TypedScreen, UIState};

/// Shared context for handlers that need access to non-screen state
///
/// This context provides handlers with access to the broader application state
/// when they need to perform transitions or access shared state like profile,
/// scenarios, or UI state.
///
/// # Usage
///
/// Simple handlers that only modify screen data don't need HandlerContext:
///
/// ```ignore
/// pub fn handle_menu_up(data: &mut MenuData) -> Result<HandlerOutcome, UserError> {
///     if data.selected_item > 0 {
///         data.selected_item -= 1;
///     }
///     Ok(HandlerOutcome::Stay)
/// }
/// ```
///
/// Complex handlers that need transitions or shared state use HandlerContext:
///
/// ```ignore
/// pub fn handle_start_scenario(
///     index: usize,
///     ctx: &mut HandlerContext,
/// ) -> Result<HandlerOutcome, UserError> {
///     let scenario = ctx.game.scenario_collection.get_filtered_by_index(index).cloned();
///     // ... create session and transition to Task screen
///     Ok(HandlerOutcome::Transition(TypedScreen::Task(task_data)))
/// }
/// ```
pub struct HandlerContext<'a> {
    /// Global UI rendering and display state
    pub ui: &'a mut UIState,
    /// Game scenarios collection and sessions
    pub game: &'a mut GameState,
    /// User progress (profile, learning, achievements)
    pub progress: &'a mut ProgressState,
    /// Application configuration (filters, settings)
    pub config: &'a ConfigState,
}

impl<'a> HandlerContext<'a> {
    /// Create a HandlerContext from mutable references
    ///
    /// This is called by the update() function to provide context to handlers.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let ctx = HandlerContext {
    ///     ui: &mut state.ui,
    ///     game: &mut state.game,
    ///     progress: &mut state.progress,
    ///     config: &state.config,
    /// };
    /// ```
    pub fn new(
        ui: &'a mut UIState,
        game: &'a mut GameState,
        progress: &'a mut ProgressState,
        config: &'a ConfigState,
    ) -> Self {
        Self {
            ui,
            game,
            progress,
            config,
        }
    }
}

/// Result of handler execution indicating screen transition
///
/// Handlers return this to indicate whether to stay on the current screen
/// or transition to a new screen. This makes screen transitions explicit
/// and trackable.
///
/// TypedScreen is boxed to reduce size difference between variants.
///
/// # Examples
///
/// ```ignore
/// // Handler that stays on current screen
/// pub fn handle_menu_up(data: &mut MenuData) -> Result<HandlerOutcome, UserError> {
///     if data.selected_item > 0 {
///         data.selected_item -= 1;
///     }
///     Ok(HandlerOutcome::Stay)
/// }
///
/// // Handler that transitions to a new screen
/// pub fn handle_menu_select(data: &MenuData, ctx: &mut HandlerContext) -> Result<HandlerOutcome, UserError> {
///     match data.selected_item {
///         0 => Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Task(task_data)))),
///         1 => Ok(HandlerOutcome::Stay),
///         _ => Ok(HandlerOutcome::Stay),
///     }
/// }
/// ```
#[derive(Debug, Default)]
pub enum HandlerOutcome {
    /// Remain on current screen (no transition)
    #[default]
    Stay,
    /// Transition to a new screen (boxed to reduce size)
    Transition(Box<TypedScreen>),
}

impl HandlerOutcome {
    /// Check if this is a stay outcome
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let outcome = HandlerOutcome::Stay;
    /// assert!(outcome.is_stay());
    /// ```
    pub fn is_stay(&self) -> bool {
        matches!(self, Self::Stay)
    }

    /// Check if this is a transition
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let outcome = HandlerOutcome::Transition(TypedScreen::Menu(MenuData::default()));
    /// assert!(outcome.is_transition());
    /// ```
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::Transition(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_outcome_is_stay() {
        let outcome = HandlerOutcome::Stay;
        assert!(outcome.is_stay());
        assert!(!outcome.is_transition());
    }

    #[test]
    fn test_handler_outcome_is_transition() {
        use crate::ui::state::MenuData;
        let outcome = HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())));
        assert!(!outcome.is_stay());
        assert!(outcome.is_transition());
    }

    #[test]
    fn test_handler_outcome_default() {
        let outcome = HandlerOutcome::default();
        assert!(outcome.is_stay());
    }
}
