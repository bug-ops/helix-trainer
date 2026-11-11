//! Test `AppState` builder for creating test fixtures
//!
//! Provides utilities for creating `AppState` instances in tests.

use crate::config::Scenario;
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::PerformanceTracker;
use crate::minigame::MiniGameSession;
use crate::security::UserError;
use crate::testing::default_test_scenario;
use crate::time::{Clock, MonotonicClock, SystemClock, SystemMonotonicClock};
use crate::ui::state::{
    AppState, ConfigState, MenuData, Message, MiniGameData, ProgressState, ReturnDestination,
    TypedScreen, UIState, update,
};
use std::sync::Arc;

/// Create a test `AppState` with no scenarios
///
/// Uses default profile, storage, and tracker.
pub fn empty_test_app_state() -> AppState {
    test_app_state_with_scenarios(vec![])
}

/// Create a test `AppState` with the provided scenarios
///
/// Uses default profile, storage, and tracker.
pub fn test_app_state_with_scenarios(scenarios: Vec<Scenario>) -> AppState {
    let profile = UserProfile::new();
    let storage = ProfileStorage::for_test();
    let tracker = PerformanceTracker::new();
    let mut state = AppState::new(scenarios, profile, storage, tracker);
    state.progress.mark_profile_loaded();
    state
}

/// Builder for creating test `AppState` with custom configuration
pub struct TestAppStateBuilder {
    scenarios: Vec<Scenario>,
    profile: Option<UserProfile>,
    storage: Option<ProfileStorage>,
    tracker: Option<PerformanceTracker>,
    clock: Option<Arc<dyn Clock>>,
    monotonic: Option<Arc<dyn MonotonicClock>>,
}

impl Default for TestAppStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAppStateBuilder {
    /// Create a new builder with empty defaults
    pub fn new() -> Self {
        Self {
            scenarios: vec![],
            profile: None,
            storage: None,
            tracker: None,
            clock: None,
            monotonic: None,
        }
    }

    /// Add a scenario
    #[must_use]
    pub fn scenario(mut self, scenario: Scenario) -> Self {
        self.scenarios.push(scenario);
        self
    }

    /// Set all scenarios
    #[must_use]
    pub fn scenarios(mut self, scenarios: Vec<Scenario>) -> Self {
        self.scenarios = scenarios;
        self
    }

    /// Set a custom user profile
    #[must_use]
    pub fn profile(mut self, profile: UserProfile) -> Self {
        self.profile = Some(profile);
        self
    }

    /// Set a custom profile storage
    #[must_use]
    pub fn storage(mut self, storage: ProfileStorage) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set a custom performance tracker
    #[must_use]
    pub fn tracker(mut self, tracker: PerformanceTracker) -> Self {
        self.tracker = Some(tracker);
        self
    }

    /// Set a custom clock (e.g. [`crate::time::FakeClock`]) for deterministic time-based tests
    #[must_use]
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Set a custom monotonic clock (e.g. [`crate::time::FakeMonotonicClock`]) for
    /// deterministic timer tests
    #[must_use]
    pub fn with_monotonic_clock(mut self, monotonic: Arc<dyn MonotonicClock>) -> Self {
        self.monotonic = Some(monotonic);
        self
    }

    /// Build the `AppState`
    pub fn build(self) -> AppState {
        let profile = self.profile.unwrap_or_default();
        let storage = self.storage.unwrap_or_else(ProfileStorage::for_test);
        let tracker = self.tracker.unwrap_or_default();
        let mut state = AppState::with_clocks(
            self.scenarios,
            profile,
            storage,
            tracker,
            ConfigState::default(),
            self.clock.unwrap_or_else(|| Arc::new(SystemClock)),
            self.monotonic
                .unwrap_or_else(|| Arc::new(SystemMonotonicClock)),
        );
        state.progress.mark_profile_loaded();
        state
    }
}

/// Start the scenario at `index` the way the UI does: by selecting it in the scenario list.
///
/// Puts the state on the Menu screen with `index` selected and dispatches
/// [`Message::MenuSelect`]. An out-of-range `index` selects one of the fixed trailing
/// menu entries (or nothing), exactly as it would for a user.
///
/// # Errors
///
/// Propagates the [`UserError`] returned by [`update`].
pub fn start_scenario_from_menu(state: &mut AppState, index: usize) -> Result<(), UserError> {
    state.screen = TypedScreen::Menu(MenuData {
        selected_item: index,
        ..MenuData::default()
    });
    update_executing_effects(state, Message::MenuSelect)
}

/// Execute the effects handlers queued on `ui`, for tests that call a handler directly.
pub fn run_queued_effects(ui: &mut UIState, progress: &mut ProgressState) {
    for effect in ui.effects.drain(..) {
        drop(effect.execute(progress));
    }
}

/// Run [`update`] and execute the effects it requests against the in-memory state, so tests see
/// sounds and saves happen as they do in the event loop.
///
/// # Errors
///
/// Propagates the [`UserError`] returned by [`update`]; effects are executed either way.
pub fn update_executing_effects(state: &mut AppState, msg: Message) -> Result<(), UserError> {
    let mut effects = Vec::new();
    let result = update(state, msg, &mut effects);
    for effect in effects {
        drop(effect.execute(&mut state.progress));
    }
    result
}

/// The review session currently on screen, if the state is on the review screen.
pub fn review_session(state: &AppState) -> Option<&crate::learning::ReviewSession> {
    match &state.screen {
        TypedScreen::Review(data) => Some(&data.session),
        _ => None,
    }
}

/// A mini-game screen payload around a freshly started session over the default test scenario.
pub fn started_minigame_data() -> MiniGameData {
    let mut session = MiniGameSession::new(Arc::new(vec![default_test_scenario()]), None);
    session.start();
    MiniGameData::new(session)
}

/// Return destination of a detour screen opened from a paused arcade run.
pub fn paused_minigame_return() -> ReturnDestination {
    let mut data = started_minigame_data();
    for _ in 0..3 {
        data.session.tick_countdown();
    }
    data.session.pause();
    ReturnDestination::PausedMiniGame(Box::new(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::scenario::default_test_scenario;

    #[test]
    fn test_empty_state() {
        let state = empty_test_app_state();
        assert_eq!(state.game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_state_with_scenarios() {
        let scenarios = vec![default_test_scenario()];
        let state = test_app_state_with_scenarios(scenarios);
        assert_eq!(state.game.scenario_collection.count(), 1);
    }

    #[test]
    fn test_builder() {
        let state = TestAppStateBuilder::new()
            .scenario(default_test_scenario())
            .scenario(default_test_scenario())
            .build();
        assert_eq!(state.game.scenario_collection.count(), 2);
    }
}
