//! Sub-structures for organizing AppState
//!
//! This module defines focused sub-structures that group related fields
//! from AppState, improving code organization and maintainability.

use crate::config::{Difficulty, ScenarioCategory, ScenarioCollection, SortMode};
use crate::game::GameSession;
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::{PerformanceTracker, ScenarioMastery, Scheduler};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::{QuestProgressChange, ReviewSessionState, XPBreakdown};

/// UI rendering and display state
///
/// NOTE: After Phase 3 refactoring, most screen-specific data has been moved
/// into TypedScreen variants. This struct now only contains global UI state
/// that persists across screen transitions.
#[derive(Debug)]
pub struct UIState {
    /// Whether the application is running
    pub running: bool,

    /// Whether to show key history popup (global setting)
    pub show_key_history: bool,

    /// Time when scenario was completed (for success animation)
    pub completion_time: Option<Instant>,

    /// TEMPORARY (Phase 3): Feedback storage for transition to results screen
    /// Will be moved into TypedScreen::Results in future refactoring
    pub last_feedback: Option<crate::game::Feedback>,

    /// TEMPORARY (Phase 3): XP breakdown for results display
    /// Will be moved into ResultsData in future refactoring
    pub xp_breakdown: Option<XPBreakdown>,

    /// TEMPORARY (Phase 3): Quest progress changes
    /// Will be moved into ResultsData in future refactoring
    pub quest_progress_changes: Vec<QuestProgressChange>,

    /// TEMPORARY (Phase 3): Scenario mastery info
    /// Will be moved into ResultsData in future refactoring
    pub scenario_mastery: Option<(ScenarioMastery, f64)>,
}

impl UIState {
    /// Create new UIState with default values
    pub fn new() -> Self {
        Self {
            running: true,
            show_key_history: true,
            completion_time: None,
            last_feedback: None,
            xp_breakdown: None,
            quest_progress_changes: Vec::new(),
            scenario_mastery: None,
        }
    }

    /// Clear temporary results data (Phase 3 compatibility)
    pub fn clear_temp_results(&mut self) {
        self.last_feedback = None;
        self.xp_breakdown = None;
        self.quest_progress_changes.clear();
        self.scenario_mastery = None;
    }
}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

/// Active game state (session, scenarios)
pub struct GameState {
    /// All available scenarios with filtering and sorting
    pub scenario_collection: ScenarioCollection,

    /// DEPRECATED: Kept for backward compatibility during Phase 3 migration
    /// Will be removed in Phase 4. Session is now stored in TypedScreen::Task.
    pub session: Option<GameSession<crate::game::session::Active>>,

    /// Active review session (Some if reviewing)
    pub review_session: Option<ReviewSessionState>,

    /// Temporary storage for completed session (Phase 3 migration)
    /// Used to pass session from gameplay handler to CompleteScenario handler
    pub pending_completed_session: Option<GameSession<crate::game::session::Completed>>,

    /// Active mini-game session (Arcade Mode)
    pub minigame_session: Option<crate::minigame::MiniGameSession>,
}

impl GameState {
    /// Create new GameState with scenarios
    pub fn new(scenarios: Vec<crate::config::Scenario>) -> Self {
        Self {
            scenario_collection: ScenarioCollection::new(scenarios),
            session: None,
            review_session: None,
            pending_completed_session: None,
            minigame_session: None,
        }
    }

    /// Check if actively playing a scenario
    pub fn is_playing(&self) -> bool {
        self.session.is_some()
    }

    /// Check if in review session
    pub fn is_reviewing(&self) -> bool {
        self.review_session.is_some()
    }

    /// Check if playing mini-game
    pub fn is_playing_minigame(&self) -> bool {
        self.minigame_session.is_some()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            scenario_collection: ScenarioCollection::new(vec![]),
            session: None,
            review_session: None,
            pending_completed_session: None,
            minigame_session: None,
        }
    }
}

impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameState")
            .field("scenario_count", &self.scenario_collection.count())
            .field("session", &self.session.is_some())
            .field("review_session", &self.review_session.is_some())
            .field("minigame_session", &self.minigame_session.is_some())
            .finish()
    }
}

/// User progress (profile, learning, achievements)
pub struct ProgressState {
    /// User profile with XP, level, achievements
    pub profile: Rc<RefCell<UserProfile>>,

    /// Performance tracker for spaced repetition
    pub performance_tracker: Rc<RefCell<PerformanceTracker>>,

    /// Scheduler for review sessions
    pub scheduler: Scheduler,

    /// Profile storage handler
    pub storage: ProfileStorage,

    /// Scenarios completed today
    pub scenarios_completed_today: usize,

    /// Commands used today (for exploration quests)
    pub commands_used_today: HashSet<String>,

    /// Previously completed quest IDs (to detect new completions)
    pub previously_completed_quests: HashSet<String>,

    /// Session start time
    pub session_start_time: Instant,

    /// Last save time (for debouncing)
    pub last_save_time: Option<Instant>,
}

impl ProgressState {
    /// Create new ProgressState
    pub fn new(
        profile: UserProfile,
        performance_tracker: PerformanceTracker,
        storage: ProfileStorage,
    ) -> Self {
        let profile = Rc::new(RefCell::new(profile));
        let tracker_rc = Rc::new(RefCell::new(performance_tracker));
        let scheduler = Scheduler::new(Rc::clone(&tracker_rc));

        Self {
            profile,
            performance_tracker: tracker_rc,
            scheduler,
            storage,
            scenarios_completed_today: 0,
            commands_used_today: HashSet::new(),
            previously_completed_quests: HashSet::new(),
            session_start_time: Instant::now(),
            last_save_time: None,
        }
    }

    /// Check if enough time has passed since last save (5 seconds)
    pub fn should_save(&self) -> bool {
        match self.last_save_time {
            Some(last) => last.elapsed() >= Duration::from_secs(5),
            None => true,
        }
    }

    /// Mark save as completed
    pub fn mark_saved(&mut self) {
        self.last_save_time = Some(Instant::now());
    }
}

impl std::fmt::Debug for ProgressState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressState")
            .field("profile", &"<Rc<RefCell<UserProfile>>>")
            .field("performance_tracker", &"<Rc<RefCell<PerformanceTracker>>>")
            .field("scenarios_completed_today", &self.scenarios_completed_today)
            .field("commands_used_today", &self.commands_used_today.len())
            .finish()
    }
}

/// Application configuration (filters, settings)
#[derive(Debug, Clone)]
pub struct ConfigState {
    /// Current sort mode for scenarios
    pub sort_mode: SortMode,

    /// Active category filters
    pub category_filters: HashSet<ScenarioCategory>,

    /// Active difficulty filters
    pub difficulty_filters: HashSet<Difficulty>,

    /// Whether to show completed scenarios
    pub show_completed: bool,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            sort_mode: SortMode::ByName,
            category_filters: HashSet::new(),
            difficulty_filters: HashSet::new(),
            show_completed: true,
        }
    }
}

impl ConfigState {
    /// Reset all filters to default
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Check if any filters are active
    pub fn has_active_filters(&self) -> bool {
        !self.category_filters.is_empty()
            || !self.difficulty_filters.is_empty()
            || !self.show_completed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_state_new() {
        let ui = UIState::new();
        assert!(ui.running);
        assert!(ui.show_key_history);
        assert!(ui.completion_time.is_none());
    }

    #[test]
    fn test_config_state_default() {
        let config = ConfigState::default();
        assert_eq!(config.sort_mode, SortMode::ByName);
        assert!(config.category_filters.is_empty());
        assert!(config.show_completed);
    }

    #[test]
    fn test_config_state_reset() {
        let mut config = ConfigState {
            sort_mode: SortMode::ByDifficulty,
            category_filters: {
                let mut set = std::collections::HashSet::new();
                set.insert(ScenarioCategory::Movement);
                set
            },
            ..Default::default()
        };

        config.reset();

        assert_eq!(config.sort_mode, SortMode::ByName);
        assert!(config.category_filters.is_empty());
    }

    #[test]
    fn test_config_state_has_active_filters() {
        let mut config = ConfigState::default();
        assert!(!config.has_active_filters());

        config.category_filters.insert(ScenarioCategory::Movement);
        assert!(config.has_active_filters());
    }

    #[test]
    fn test_game_state_default() {
        let game = GameState::default();
        assert_eq!(game.scenario_collection.count(), 0);
        assert!(game.session.is_none());
        assert!(game.review_session.is_none());
    }

    #[test]
    fn test_game_state_is_playing() {
        let game = GameState::default();
        assert!(!game.is_playing());
        // Note: Testing with actual session requires GameSession setup
    }

    #[test]
    fn test_game_state_is_reviewing() {
        let game = GameState::default();
        assert!(!game.is_reviewing());
        // Note: Testing with actual review session requires ReviewSessionState setup
    }

    #[test]
    fn test_progress_state_should_save_initially_true() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;

        let progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::new(),
        );

        // Should save immediately when no previous save
        assert!(progress.should_save());
    }

    #[test]
    fn test_progress_state_mark_saved_updates_time() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::new(),
        );

        assert!(progress.last_save_time.is_none());
        progress.mark_saved();
        assert!(progress.last_save_time.is_some());
    }

    #[test]
    fn test_progress_state_debounce_prevents_immediate_resave() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::new(),
        );

        // First save should be allowed
        assert!(progress.should_save());
        progress.mark_saved();

        // Immediate second save should be blocked (within 5 second window)
        assert!(!progress.should_save());
    }
}
