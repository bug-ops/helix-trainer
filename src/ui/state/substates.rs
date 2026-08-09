//! Sub-structures for organizing AppState
//!
//! This module defines focused sub-structures that group related fields
//! from AppState, improving code organization and maintainability.

use crate::config::{AppConfig, Difficulty, ScenarioCategory, ScenarioCollection, SortMode};
use crate::constants::PROFILE_SAVE_DEBOUNCE;
use crate::game::GameSession;
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::{PerformanceTracker, ScenarioMastery, Scheduler};
use crate::sound::SoundManager;
use std::collections::HashSet;
use std::time::Instant;

use super::{QuestProgressChange, ReviewSessionState, XPBreakdown};
use crate::ui::notification::NotificationQueue;

/// UI rendering and display state
///
/// Contains global UI state that persists across screen transitions.
/// Screen-specific data is stored in TypedScreen variants.
#[derive(Debug)]
pub struct UIState {
    /// Whether the application is running
    pub running: bool,

    /// Whether to show key history popup (global setting)
    pub show_key_history: bool,

    /// Time when scenario was completed (for success animation)
    pub completion_time: Option<Instant>,

    /// Feedback storage for transition to results screen
    pub last_feedback: Option<crate::game::Feedback>,

    /// XP breakdown for results display
    pub xp_breakdown: Option<XPBreakdown>,

    /// Quest progress changes for results display
    pub quest_progress_changes: Vec<QuestProgressChange>,

    /// Scenario mastery info for results display
    pub scenario_mastery: Option<(ScenarioMastery, f64)>,

    /// Notification queue for transient messages (level-up, achievements, etc.)
    pub notifications: NotificationQueue,

    /// Last selected scenario index in menu (session-only, for position persistence)
    pub last_menu_selected: usize,

    /// Last scroll offset in menu (session-only, for position persistence)
    pub last_menu_scroll: usize,
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
            notifications: NotificationQueue::new(),
            last_menu_selected: 0,
            last_menu_scroll: 0,
        }
    }

    /// Clear results data after transitioning away from results screen
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

    /// Active review session (Some if reviewing)
    pub review_session: Option<ReviewSessionState>,

    /// Completed session pending transition to results screen
    pub pending_completed_session: Option<GameSession<crate::game::session::Completed>>,

    /// Active mini-game session (Arcade Mode)
    pub minigame_session: Option<crate::minigame::MiniGameSession>,
}

impl GameState {
    /// Create new GameState with scenarios
    pub fn new(scenarios: Vec<crate::config::Scenario>) -> Self {
        Self {
            scenario_collection: ScenarioCollection::new(scenarios),
            review_session: None,
            pending_completed_session: None,
            minigame_session: None,
        }
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
            .field("review_session", &self.review_session.is_some())
            .field(
                "pending_completed_session",
                &self.pending_completed_session.is_some(),
            )
            .field("minigame_session", &self.minigame_session.is_some())
            .finish()
    }
}

/// User progress (profile, learning, achievements)
pub struct ProgressState {
    /// User profile with XP, level, achievements
    pub profile: UserProfile,

    /// Performance tracker for spaced repetition
    pub performance_tracker: PerformanceTracker,

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

    /// Sound manager for audio feedback
    pub sound_manager: SoundManager,
}

impl ProgressState {
    /// Create new ProgressState
    pub fn new(
        profile: UserProfile,
        performance_tracker: PerformanceTracker,
        storage: ProfileStorage,
    ) -> Self {
        // Initialize sound manager from profile config
        let mut sound_manager = SoundManager::new(profile.sound_config.clone());
        // Try to initialize audio (graceful failure)
        let _ = sound_manager.try_init();

        Self {
            profile,
            performance_tracker,
            scheduler: Scheduler::new(),
            storage,
            scenarios_completed_today: 0,
            commands_used_today: HashSet::new(),
            previously_completed_quests: HashSet::new(),
            session_start_time: Instant::now(),
            last_save_time: None,
            sound_manager,
        }
    }

    /// Check if enough time has passed since last save
    pub fn should_save(&self) -> bool {
        match self.last_save_time {
            Some(last) => last.elapsed() >= PROFILE_SAVE_DEBOUNCE,
            None => true,
        }
    }

    /// Mark save as completed
    pub fn mark_saved(&mut self) {
        self.last_save_time = Some(Instant::now());
    }

    /// Sync FSRS performance data from the live tracker and persist the profile immediately.
    ///
    /// This is the single source of truth for saving progress: every call site that
    /// persists the profile mid-session must go through this method (or
    /// [`ProgressState::save_debounced`]) rather than calling `storage.save` directly,
    /// otherwise `profile.performance_data` silently goes stale.
    ///
    /// # Errors
    ///
    /// Returns an error if the save operation fails.
    pub fn save_immediate(&mut self) -> Result<(), crate::gamification::GamificationError> {
        self.profile.performance_data = self.performance_tracker.get_stats_clone();
        self.storage.save(&self.profile)?;
        self.mark_saved();
        Ok(())
    }

    /// Save the profile only if the debounce interval has elapsed since the last save.
    ///
    /// Syncs FSRS performance data from the live tracker before writing, same as
    /// [`ProgressState::save_immediate`].
    ///
    /// # Errors
    ///
    /// Returns an error if the save operation fails.
    pub fn save_debounced(&mut self) -> Result<(), crate::gamification::GamificationError> {
        if !self.should_save() {
            return Ok(());
        }
        self.save_immediate()
    }
}

impl std::fmt::Debug for ProgressState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressState")
            .field("profile", &self.profile)
            .field("performance_tracker", &"<PerformanceTracker>")
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

    /// Persistent configuration (loaded from/saved to disk)
    pub persistent: AppConfig,

    /// Whether persistent config was modified during this session
    pub config_modified: bool,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            persistent: AppConfig::default(),
            sort_mode: SortMode::ByName,
            category_filters: HashSet::new(),
            difficulty_filters: HashSet::new(),
            show_completed: true,
            config_modified: false,
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

    /// Mark that persistent config was modified
    pub fn mark_config_modified(&mut self) {
        self.config_modified = true;
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
        assert_eq!(ui.last_menu_selected, 0);
        assert_eq!(ui.last_menu_scroll, 0);
    }

    #[test]
    fn test_config_state_default() {
        let config = ConfigState::default();
        assert_eq!(config.sort_mode, SortMode::ByName);
        assert!(config.category_filters.is_empty());
        assert!(config.show_completed);
        assert!(!config.config_modified);
        assert!(!config.persistent.enable_arrow_keys_in_normal_mode);
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
        assert!(game.review_session.is_none());
        assert!(game.pending_completed_session.is_none());
        assert!(game.minigame_session.is_none());
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
            ProfileStorage::for_test(),
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
            ProfileStorage::for_test(),
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
            ProfileStorage::for_test(),
        );

        // First save should be allowed
        assert!(progress.should_save());
        progress.mark_saved();

        // Immediate second save should be blocked (within 5 second window)
        assert!(!progress.should_save());
    }

    /// Compares the fields that `save_immediate`/`save_debounced` are responsible for
    /// syncing. `CommandPerformance` has no `PartialEq` impl, so this checks the subset
    /// of fields that prove the persisted copy reflects the tracker's state.
    fn stats_match(
        a: &std::collections::HashMap<String, crate::learning::CommandPerformance>,
        b: &std::collections::HashMap<String, crate::learning::CommandPerformance>,
    ) -> bool {
        a.len() == b.len()
            && a.iter().all(|(k, v)| {
                b.get(k).is_some_and(|other| {
                    other.attempts == v.attempts
                        && other.successes == v.successes
                        && other.reps == v.reps
                        && other.command == v.command
                })
            })
    }

    #[test]
    fn test_save_immediate_syncs_tracker_stats_into_persisted_profile() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        );

        progress.performance_tracker.record_attempt(
            "x",
            Duration::from_millis(500),
            true,
            Duration::from_millis(500),
        );

        progress.save_immediate().unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(!persisted.performance_data.is_empty());
        assert!(stats_match(
            &persisted.performance_data,
            &progress.performance_tracker.get_stats_clone()
        ));
    }

    #[test]
    fn test_save_debounced_does_not_write_within_debounce_window() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        );

        // Establish a recent save so the debounce window is active.
        progress.save_immediate().unwrap();
        let after_first_save = std::fs::read_to_string(&profile_path).unwrap();

        // Mutate the tracker after the save; a debounced save within the window
        // must not pick this up.
        progress.performance_tracker.record_attempt(
            "d",
            Duration::from_millis(500),
            true,
            Duration::from_millis(500),
        );
        progress.save_debounced().unwrap();

        let after_debounced_call = std::fs::read_to_string(&profile_path).unwrap();
        assert_eq!(
            after_first_save, after_debounced_call,
            "debounced save must not have rewritten the file within the debounce window"
        );
    }

    #[test]
    fn test_save_debounced_writes_once_debounce_elapses() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use std::time::{Duration, Instant};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        );

        // Simulate a save that happened long enough ago for the debounce to elapse,
        // without needing to sleep in the test.
        progress.last_save_time =
            Some(Instant::now() - PROFILE_SAVE_DEBOUNCE - Duration::from_secs(1));

        progress.performance_tracker.record_attempt(
            "j",
            Duration::from_millis(500),
            true,
            Duration::from_millis(500),
        );

        assert!(progress.should_save());
        progress.save_debounced().unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(stats_match(
            &persisted.performance_data,
            &progress.performance_tracker.get_stats_clone()
        ));
        assert!(!persisted.performance_data.is_empty());
    }
}
