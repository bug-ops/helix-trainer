//! Type-safe handler infrastructure
//!
//! Provides compile-time guarantees that handlers receive the correct
//! screen data type, eliminating runtime type checks.

use crate::error::UserError;
use crate::ui::state::{ConfigState, GameState, ProgressState, TypedScreen, UIState};

/// Shared context for handlers that need access to non-screen state
///
/// This context provides handlers with access to the broader application state
/// when they need to perform transitions or access shared state like profile,
/// scenarios, or UI state.
///
/// # Usage
///
/// Simple handlers that only modify screen data don't need `HandlerContext`:
///
/// ```
/// use helix_trainer::ui::state::{HandlerOutcome, MenuData};
///
/// fn handle_menu_up(data: &mut MenuData) -> HandlerOutcome {
///     data.selected_item = data.selected_item.saturating_sub(1);
///     HandlerOutcome::Stay
/// }
///
/// let mut data = MenuData { selected_item: 1, ..MenuData::default() };
/// assert!(handle_menu_up(&mut data).is_stay());
/// assert_eq!(data.selected_item, 0);
/// ```
///
/// Handlers with a real failure path return `Result<HandlerOutcome, UserError>` instead;
/// the dispatcher accepts both.
///
/// Complex handlers that need transitions or shared state take a `HandlerContext`
/// and may return [`HandlerOutcome::Transition`] with the next screen.
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
    /// Create a `HandlerContext` from mutable references
    ///
    /// This is called by the `update()` function to provide context to handlers.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{ProfileStorage, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::ui::state::HandlerContext;
    ///
    /// let storage = ProfileStorage::with_path(std::env::temp_dir().join("doc-profile.json"));
    /// let mut state = AppState::new(vec![], UserProfile::new(), storage, PerformanceTracker::new());
    /// let ctx = HandlerContext::new(
    ///     &mut state.ui,
    ///     &mut state.game,
    ///     &mut state.progress,
    ///     &state.config,
    /// );
    /// assert!(ctx.ui.running);
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
/// `TypedScreen` is boxed to reduce size difference between variants.
///
/// # Examples
///
/// ```
/// use helix_trainer::ui::state::{HandlerOutcome, MenuData, TypedScreen};
///
/// let stay = HandlerOutcome::Stay;
/// assert!(stay.is_stay());
///
/// let go = HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())));
/// assert!(go.is_transition());
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
    /// ```
    /// use helix_trainer::ui::state::HandlerOutcome;
    ///
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
    /// ```
    /// use helix_trainer::ui::state::{HandlerOutcome, MenuData, TypedScreen};
    ///
    /// let outcome = HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())));
    /// assert!(outcome.is_transition());
    /// ```
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::Transition(_))
    }
}

/// The single boundary between handler signatures and the dispatcher
///
/// Handlers that cannot fail return a plain [`HandlerOutcome`]; only handlers with a real
/// failure path return `Result<HandlerOutcome, UserError>`. `update()` accepts both through
/// this trait, so the error policy lives in one place.
pub(super) trait IntoHandlerResult {
    /// Convert the handler's return value into the uniform fallible form
    fn into_outcome(self) -> Result<HandlerOutcome, UserError>;
}

impl IntoHandlerResult for HandlerOutcome {
    fn into_outcome(self) -> Result<HandlerOutcome, UserError> {
        Ok(self)
    }
}

impl IntoHandlerResult for Result<HandlerOutcome, UserError> {
    fn into_outcome(self) -> Result<HandlerOutcome, UserError> {
        self
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
    fn test_into_outcome_passes_plain_and_fallible_handlers_through() {
        assert!(HandlerOutcome::Stay.into_outcome().unwrap().is_stay());
        let ok: Result<HandlerOutcome, UserError> = Ok(HandlerOutcome::Stay);
        assert!(ok.into_outcome().unwrap().is_stay());
        let err: Result<HandlerOutcome, UserError> = Err(UserError::WrongScreen);
        assert!(err.into_outcome().is_err());
    }

    #[test]
    fn test_handler_outcome_default() {
        let outcome = HandlerOutcome::default();
        assert!(outcome.is_stay());
    }
}
