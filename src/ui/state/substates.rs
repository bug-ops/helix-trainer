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

use super::{QuestProgressChange, ReviewSessionState, Screen, XPBreakdown};

/// UI rendering and display state
#[derive(Debug)]
pub struct UIState {
    /// The current screen being displayed
    pub screen: Screen,

    /// Whether the application is running
    pub running: bool,

    /// Index of selected menu item
    pub selected_menu_item: usize,

    /// Scroll offset for menu list (top visible item index)
    pub menu_scroll_offset: usize,

    /// Current hint being displayed
    pub current_hint: Option<String>,

    /// Last command executed (for display)
    pub last_command: Option<String>,

    /// History of last 5 keypresses (most recent first)
    pub key_history: Vec<String>,

    /// Command buffer for multi-key commands (e.g., "d" waiting for "d")
    pub command_buffer: String,

    /// Whether to show hint panel
    pub show_hint_panel: bool,

    /// Whether to show key history popup
    pub show_key_history: bool,

    /// Time when scenario was completed (for success animation)
    pub completion_time: Option<Instant>,

    /// XP breakdown from last scenario (for results display)
    pub xp_breakdown: Option<XPBreakdown>,

    /// Quest progress changes during last scenario
    pub quest_progress_changes: Vec<QuestProgressChange>,

    /// Scenario mastery info (for results display)
    pub scenario_mastery: Option<(ScenarioMastery, f64)>,
}

impl UIState {
    /// Create new UIState with default values
    pub fn new() -> Self {
        Self {
            screen: Screen::MainMenu,
            running: true,
            selected_menu_item: 0,
            menu_scroll_offset: 0,
            current_hint: None,
            last_command: None,
            key_history: Vec::with_capacity(5),
            command_buffer: String::new(),
            show_hint_panel: false,
            show_key_history: true,
            completion_time: None,
            xp_breakdown: None,
            quest_progress_changes: Vec::new(),
            scenario_mastery: None,
        }
    }

    /// Clear results-related state
    pub fn clear_results(&mut self) {
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

    /// Active game session (Some if playing)
    pub session: Option<GameSession>,

    /// Active review session (Some if reviewing)
    pub review_session: Option<ReviewSessionState>,
}

impl GameState {
    /// Create new GameState with scenarios
    pub fn new(scenarios: Vec<crate::config::Scenario>) -> Self {
        Self {
            scenario_collection: ScenarioCollection::new(scenarios),
            session: None,
            review_session: None,
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
}

impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameState")
            .field("scenario_count", &self.scenario_collection.count())
            .field("session", &self.session.is_some())
            .field("review_session", &self.review_session.is_some())
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
        assert_eq!(ui.screen, Screen::MainMenu);
        assert!(ui.running);
        assert_eq!(ui.selected_menu_item, 0);
        assert!(ui.key_history.is_empty());
    }

    #[test]
    fn test_ui_state_clear_results() {
        let mut ui = UIState::new();
        ui.xp_breakdown = Some(XPBreakdown {
            base_xp: 100,
            perfect_bonus: 20,
            first_today_bonus: 10,
            mastery_multiplier: 1.0,
            quest_bonuses: vec![],
            total_xp: 130,
        });
        ui.quest_progress_changes.push(QuestProgressChange {
            quest_description: "test".to_string(),
            old_progress: 0,
            new_progress: 1,
        });

        ui.clear_results();

        assert!(ui.xp_breakdown.is_none());
        assert!(ui.quest_progress_changes.is_empty());
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
}
