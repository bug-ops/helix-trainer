//! Type-safe screen variants with required data
//!
//! This module implements the TypedScreen pattern, where each screen variant
//! carries the data required for that screen. This makes invalid states
//! unrepresentable at compile time - you can't be on the Task screen without
//! an active GameSession.

use crate::config::ScenarioCategory;
use crate::game::{Abandoned, Active, Completed, Feedback, GameSession};
use crate::input::typestate::InputStateMachine;
use crate::learning::ScenarioMastery;
use crate::ui::state::{QuestProgressChange, ReviewSessionState, XPBreakdown};

/// Maximum number of keys to keep in history
const KEY_HISTORY_CAPACITY: usize = 5;

/// Shared key history for tracking recent keypresses
///
/// Used by both training mode (TaskData) and arcade mode (MiniGameData)
/// to display recent key presses in the UI. Keeps the last 5 keys
/// with most recent first.
#[derive(Debug, Clone, Default)]
pub struct KeyHistory {
    /// History of keypresses (most recent first)
    keys: Vec<String>,
}

impl KeyHistory {
    /// Create a new empty key history
    pub fn new() -> Self {
        Self {
            keys: Vec::with_capacity(KEY_HISTORY_CAPACITY),
        }
    }

    /// Add a key to the history (keeps last 5, most recent first)
    pub fn push(&mut self, key: String) {
        self.keys.insert(0, key);
        if self.keys.len() > KEY_HISTORY_CAPACITY {
            self.keys.truncate(KEY_HISTORY_CAPACITY);
        }
    }

    /// Clear all key history
    pub fn clear(&mut self) {
        self.keys.clear();
    }

    /// Get the keys as a slice (most recent first)
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Get the number of keys in history
    pub fn len(&self) -> usize {
        self.keys.len()
    }
}

/// Screen with associated required data - invalid states unrepresentable
///
/// Each variant contains the exact data needed for that screen, guaranteeing
/// at compile time that the required data exists.
#[derive(Debug)]
pub enum TypedScreen {
    /// Mode selection screen (Training vs Arcade)
    ModeSelection(ModeSelectionData),

    /// Main menu with selected item (Training Mode scenarios)
    Menu(MenuData),

    /// Active game session (guaranteed to have session)
    Task(TaskData),

    /// Results after completion/abandonment (guaranteed to have feedback)
    Results(ResultsData),

    /// User profile screen
    Profile(ProfileData),

    /// Statistics screen
    Statistics(StatisticsData),

    /// Achievements screen
    Achievements(AchievementsData),

    /// Category filters configuration screen
    CategoryFilters(CategoryFiltersData),

    /// Review session screen
    Review(ReviewData),

    /// Mini-game mode (arcade-style training)
    MiniGame(MiniGameData),

    /// Curriculum-completion summary, reached once every scenario has been
    /// completed at least once
    EndGameSummary(EndGameSummaryData),
}

impl TypedScreen {
    /// Get screen type for display/logging
    pub fn screen_type(&self) -> &'static str {
        match self {
            Self::ModeSelection(_) => "ModeSelection",
            Self::Menu(_) => "Menu",
            Self::Task(_) => "Task",
            Self::Results(_) => "Results",
            Self::Profile(_) => "Profile",
            Self::Statistics(_) => "Statistics",
            Self::Achievements(_) => "Achievements",
            Self::CategoryFilters(_) => "CategoryFilters",
            Self::Review(_) => "Review",
            Self::MiniGame(_) => "MiniGame",
            Self::EndGameSummary(_) => "EndGameSummary",
        }
    }

    /// Get the corresponding Screen enum value (for backward compatibility)
    pub fn to_screen_enum(&self) -> super::Screen {
        match self {
            Self::ModeSelection(_) => super::Screen::ModeSelection,
            Self::Menu(_) => super::Screen::MainMenu,
            Self::Task(_) => super::Screen::Task,
            Self::Results(_) => super::Screen::Results,
            Self::Profile(_) => super::Screen::Profile,
            Self::Statistics(_) => super::Screen::Statistics,
            Self::Achievements(_) => super::Screen::Achievements,
            Self::CategoryFilters(_) => super::Screen::CategoryFilters,
            Self::Review(_) => super::Screen::Review,
            Self::MiniGame(_) => super::Screen::MiniGame,
            Self::EndGameSummary(_) => super::Screen::EndGameSummary,
        }
    }
}

/// Data required for main menu screen
#[derive(Debug, Clone, Default)]
pub struct MenuData {
    /// Index of selected menu item
    pub selected_item: usize,

    /// Scroll offset for menu list (top visible item index)
    pub scroll_offset: usize,

    /// Command buffer for multi-key navigation (e.g., "g" waiting for "g", count prefix "12")
    pub command_buffer: String,
}

impl CommandBufferAccess for MenuData {
    fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    fn command_buffer_mut(&mut self) -> &mut String {
        &mut self.command_buffer
    }
}

/// Data required for task screen (contains active session)
#[derive(Debug)]
pub struct TaskData {
    /// Active game session (guaranteed to exist)
    pub session: GameSession<Active>,

    /// Typestate-based input state machine for multi-key commands
    ///
    /// Replaces the old `command_buffer: String` with a compile-time safe
    /// state machine that tracks pending input states (goto, find, count, etc.)
    pub input_state: InputStateMachine,

    /// History of recent keypresses (shared KeyHistory struct)
    pub key_history: KeyHistory,

    /// Current hint being displayed
    pub current_hint: Option<String>,

    /// Whether to show hint panel
    pub show_hint_panel: bool,

    /// Last command executed (for display)
    pub last_command: Option<String>,

    /// Index of current scenario in filtered list (for next/prev navigation)
    /// None if scenario was started from outside the normal list flow
    pub scenario_index: Option<usize>,
}

impl TaskData {
    /// Create new TaskData with an active session
    ///
    /// # Arguments
    /// * `session` - Active game session
    pub fn new(session: GameSession<Active>) -> Self {
        Self {
            session,
            input_state: InputStateMachine::new(),
            key_history: KeyHistory::new(),
            current_hint: None,
            show_hint_panel: false,
            last_command: None,
            scenario_index: None,
        }
    }

    /// Create new TaskData with an active session and scenario index
    ///
    /// # Arguments
    /// * `session` - Active game session
    /// * `index` - Index of scenario in filtered list
    pub fn with_index(session: GameSession<Active>, index: usize) -> Self {
        Self {
            session,
            input_state: InputStateMachine::new(),
            key_history: KeyHistory::new(),
            current_hint: None,
            show_hint_panel: false,
            last_command: None,
            scenario_index: Some(index),
        }
    }

    /// Add a key to the history (keeps last 5)
    pub fn add_key_to_history(&mut self, key: String) {
        self.key_history.push(key);
    }

    /// Clear key history
    pub fn clear_key_history(&mut self) {
        self.key_history.clear();
    }
}

/// Data required for results screen
#[derive(Debug)]
pub struct ResultsData {
    /// Session that was completed or abandoned
    pub session: CompletedOrAbandoned,

    /// Feedback from the session (guaranteed to exist)
    pub feedback: Feedback,

    /// XP breakdown (optional, for gamification)
    pub xp_breakdown: Option<XPBreakdown>,

    /// Quest progress changes during session
    pub quest_changes: Vec<QuestProgressChange>,

    /// Scenario mastery info (mastery level, multiplier)
    pub scenario_mastery: Option<(ScenarioMastery, f64)>,

    /// Index of current scenario in filtered list (for next/prev navigation)
    /// None if scenario was started from outside the normal list flow
    pub scenario_index: Option<usize>,
}

impl ResultsData {
    /// Create new ResultsData from a completed session
    ///
    /// # Arguments
    /// * `session` - Completed game session
    /// * `feedback` - Performance feedback
    /// * `scenario_index` - Index of scenario in filtered list (for navigation)
    pub fn from_completed(
        session: GameSession<Completed>,
        feedback: Feedback,
        scenario_index: Option<usize>,
    ) -> Result<Self, crate::security::SecurityError> {
        Ok(Self {
            session: CompletedOrAbandoned::Completed(session),
            feedback,
            xp_breakdown: None,
            quest_changes: Vec::new(),
            scenario_mastery: None,
            scenario_index,
        })
    }

    /// Create new ResultsData from an abandoned session
    ///
    /// # Arguments
    /// * `session` - Abandoned game session
    /// * `feedback` - Performance feedback
    /// * `scenario_index` - Index of scenario in filtered list (for navigation)
    pub fn from_abandoned(
        session: GameSession<Abandoned>,
        feedback: Feedback,
        scenario_index: Option<usize>,
    ) -> Self {
        Self {
            session: CompletedOrAbandoned::Abandoned(session),
            feedback,
            xp_breakdown: None,
            quest_changes: Vec::new(),
            scenario_mastery: None,
            scenario_index,
        }
    }
}

/// Either a completed or abandoned session
#[derive(Debug)]
pub enum CompletedOrAbandoned {
    /// Session that was successfully completed
    Completed(GameSession<Completed>),

    /// Session that was abandoned
    Abandoned(GameSession<Abandoned>),
}

impl CompletedOrAbandoned {
    /// Check if this is a completed session
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }

    /// Check if this is an abandoned session
    pub fn is_abandoned(&self) -> bool {
        matches!(self, Self::Abandoned(_))
    }

    /// Get the scenario ID from either variant
    pub fn scenario_id(&self) -> &str {
        match self {
            Self::Completed(session) => &session.scenario().id,
            Self::Abandoned(session) => &session.scenario().id,
        }
    }
}

/// Return destination when navigating back from profile/statistics screens
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReturnDestination {
    /// Return to main menu (default)
    #[default]
    Menu,
    /// Return to paused mini-game
    PausedMiniGame,
}

/// Data required for profile screen
#[derive(Debug, Clone, Default)]
pub struct ProfileData {
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,
}

/// Data required for statistics screen
#[derive(Debug, Clone, Default)]
pub struct StatisticsData {
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,
}

/// Data required for achievements screen
#[derive(Debug, Clone, Default)]
pub struct AchievementsData {
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,

    /// Scroll offset for the achievement list (top visible row index).
    /// Clamped to the valid range on render, the same way `MenuData::scroll_offset` is.
    pub scroll_offset: usize,
}

/// Data required for category filters screen
#[derive(Debug, Clone, Default)]
pub struct CategoryFiltersData {
    /// Index of currently selected category (0-indexed)
    pub selected_index: usize,
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,
}

/// Data required for review session screen
#[derive(Debug)]
pub struct ReviewData {
    /// The active review session
    pub session: ReviewSessionState,
}

impl ReviewData {
    /// Create new ReviewData with a review session
    pub fn new(session: ReviewSessionState) -> Self {
        Self { session }
    }
}

/// Immutable snapshot of curriculum-completion stats for the end-game summary screen.
///
/// Snapshotted at transition time rather than read live from `AppState` (as
/// `ProfileData` does) because `render()` runs on every animation tick and the
/// numbers must not drift while the user reads the screen.
#[derive(Debug, Clone)]
pub struct EndGameSummaryData {
    /// Size of the unfiltered scenario set — the "all N" in the headline.
    pub scenarios_total: usize,
    /// Unique scenarios with at least one 100% completion.
    pub perfected: usize,
    /// `scenarios_total - perfected`. Reaches 0 at full mastery.
    pub imperfect: usize,
    /// Lifetime completion events, replays included. Must be labelled as such
    /// in copy — it is not the same axis as `scenarios_total`.
    pub total_completions: u32,
    /// Total accumulated XP.
    pub total_xp: u64,
    /// Current account level.
    pub level: u32,
    /// Mean per-command success rate, `0.0..=1.0`. Not scenario accuracy.
    pub command_success_rate: f64,
    /// Span from earliest to latest recorded scenario attempt, in days. 0 means
    /// every attempt happened on the same day.
    pub journey_days: i64,
    /// Number of commands at `MasteryLevel::Master`.
    pub commands_mastered: usize,
    /// `(category, perfected_in_category, total_in_category)`, sorted by
    /// category enum order (matches the filter screen and menu).
    pub category_breakdown: Vec<(ScenarioCategory, usize, usize)>,
    /// Suggested next actions, in display order.
    pub next_steps: Vec<NextStep>,
}

/// A suggested next action offered on the end-game summary screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextStep {
    /// N commands are due for spaced-repetition review.
    DueReviews(usize),
    /// N daily quests are still open.
    PendingQuests(usize),
    /// N scenarios have not yet been perfected.
    ImperfectScenarios(usize),
    /// Try Arcade mode — always offered, unconditional fallback.
    ArcadeMode,
}

/// Data required for mode selection screen
#[derive(Debug, Clone, Default)]
pub struct ModeSelectionData {
    /// Index of selected mode (0 = Training, 1 = Arcade)
    pub selected_mode: usize,

    /// Mini-game mode selection state (when arcade is chosen)
    pub minigame_mode_selection: Option<MiniGameModeSelection>,
}

/// Selection state for mini-game mode menu
///
/// Used when the player selects Arcade from the main mode selection
/// and needs to choose between Arcade, Survival, and Challenge modes.
///
/// Note: Visibility is controlled by the `Option` wrapper in `ModeSelectionData.minigame_mode_selection`
/// - `Some(MiniGameModeSelection)` = menu is visible
/// - `None` = menu is hidden
#[derive(Debug, Clone, Default)]
pub struct MiniGameModeSelection {
    /// Currently highlighted mode (0 = Arcade, 1 = Survival, 2 = Challenge)
    pub selected_index: usize,
}

impl MiniGameModeSelection {
    /// Available mode count
    pub const MODE_COUNT: usize = 3;

    /// Create a new mode selection starting at Arcade (index 0)
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    /// Move selection up (wraps around)
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = Self::MODE_COUNT - 1;
        }
    }

    /// Move selection down (wraps around)
    pub fn select_next(&mut self) {
        self.selected_index = (self.selected_index + 1) % Self::MODE_COUNT;
    }

    /// All selectable modes, in menu order.
    ///
    /// Single source of truth for index <-> mode mapping, shared by
    /// `selected_mode`, `mode_name`, `mode_description`, and the mode-selection
    /// submenu renderer so none of them can drift out of sync with each other.
    /// Name/description text itself comes from `MiniGameMode`'s own exhaustive
    /// match, so a new mode variant only needs adding here to appear correctly
    /// everywhere.
    pub(crate) fn all_modes(
        today: chrono::NaiveDate,
    ) -> [crate::minigame::MiniGameMode; Self::MODE_COUNT] {
        use crate::minigame::{ArcadeConfig, ChallengeConfig, MiniGameMode, SurvivalConfig};

        [
            MiniGameMode::Arcade(ArcadeConfig::default()),
            MiniGameMode::Survival(SurvivalConfig::default()),
            MiniGameMode::Challenge(ChallengeConfig::for_date(today)),
        ]
    }

    /// Get the selected mode configuration
    pub fn selected_mode(&self, today: chrono::NaiveDate) -> crate::minigame::MiniGameMode {
        Self::all_modes(today)
            .get(self.selected_index)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the name for each mode by index
    pub fn mode_name(index: usize) -> &'static str {
        // Name text does not depend on the date, so any placeholder date works.
        Self::all_modes(chrono::NaiveDate::default())
            .get(index)
            .map(crate::minigame::MiniGameMode::name)
            .unwrap_or("Unknown")
    }

    /// Get the description for each mode by index
    pub fn mode_description(index: usize) -> &'static str {
        Self::all_modes(chrono::NaiveDate::default())
            .get(index)
            .map(crate::minigame::MiniGameMode::description)
            .unwrap_or("")
    }
}

/// Data required for mini-game screen
#[derive(Debug, Clone, Default)]
pub struct MiniGameData {
    /// Typestate-based input state machine for multi-key commands
    ///
    /// Replaces the old `command_buffer: String` with a compile-time safe
    /// state machine that tracks pending input states (goto, find, count, etc.)
    pub input_state: InputStateMachine,
    /// Last XP earned (for popup display during transition)
    pub last_xp_earned: Option<u64>,
    /// History of recent keypresses (shared KeyHistory struct)
    pub key_history: KeyHistory,
    /// Current game mode (for display purposes)
    pub mode: Option<crate::minigame::MiniGameMode>,
    /// Challenge progress for tracking attempts (for Challenge mode)
    pub challenge_progress: Option<crate::minigame::ChallengeProgress>,
}

impl MiniGameData {
    /// Add a key to the history (keeps last 5)
    pub fn add_key_to_history(&mut self, key: String) {
        self.key_history.push(key);
    }

    /// Clear key history (called when starting new scenario)
    pub fn clear_key_history(&mut self) {
        self.key_history.clear();
    }
}

/// Trait for types that manage a command buffer for multi-key commands
///
/// Used by MenuData for menu navigation commands like `gg`, `5j`, `G`.
/// For gameplay screens (TaskData, MiniGameData), use [`InputStateAccess`] instead.
pub trait CommandBufferAccess {
    /// Get reference to the command buffer
    fn command_buffer(&self) -> &str;

    /// Get mutable reference to the command buffer
    fn command_buffer_mut(&mut self) -> &mut String;

    /// Push a command string to the buffer
    fn push_command(&mut self, cmd: &str) {
        self.command_buffer_mut().push_str(cmd);
    }

    /// Clear the command buffer
    fn clear_buffer(&mut self) {
        self.command_buffer_mut().clear();
    }
}

/// Trait for types that use typestate-based input state machine
///
/// TaskData and MiniGameData use `InputStateMachine` for handling multi-key
/// Helix commands with compile-time safety. This trait provides uniform
/// access to the input state machine.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::ui::state::InputStateAccess;
///
/// fn check_waiting_for_char<T: InputStateAccess>(data: &T) -> bool {
///     data.input_state().is_waiting_for_char()
/// }
/// ```
pub trait InputStateAccess {
    /// Get reference to the input state machine
    fn input_state(&self) -> &InputStateMachine;

    /// Get mutable reference to the input state machine
    fn input_state_mut(&mut self) -> &mut InputStateMachine;

    /// Reset the input state machine to base state
    fn reset_input_state(&mut self) {
        self.input_state_mut().reset();
    }

    /// Check if waiting for character input (e.g., after 'f', 'r')
    fn is_waiting_for_char(&self) -> bool {
        self.input_state().is_waiting_for_char()
    }

    /// Check if in a prefix state (building multi-key command)
    fn is_prefix_state(&self) -> bool {
        self.input_state().is_prefix_state()
    }
}

impl InputStateAccess for TaskData {
    fn input_state(&self) -> &InputStateMachine {
        &self.input_state
    }

    fn input_state_mut(&mut self) -> &mut InputStateMachine {
        &mut self.input_state
    }
}

impl InputStateAccess for MiniGameData {
    fn input_state(&self) -> &InputStateMachine {
        &self.input_state
    }

    fn input_state_mut(&mut self) -> &mut InputStateMachine {
        &mut self.input_state
    }
}

// Implement game::CommandBuffer for MenuData only
// (TaskData and MiniGameData now use InputStateMachine)
impl crate::game::CommandBuffer for MenuData {
    fn buffer(&self) -> &str {
        self.command_buffer()
    }

    fn push(&mut self, input: &str) {
        self.push_command(input);
    }

    fn clear(&mut self) {
        self.clear_buffer();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_data_default() {
        let data = MenuData::default();
        assert_eq!(data.selected_item, 0);
        assert_eq!(data.scroll_offset, 0);
    }

    #[test]
    fn test_typed_screen_screen_type() {
        let menu = TypedScreen::Menu(MenuData::default());
        assert_eq!(menu.screen_type(), "Menu");

        let profile = TypedScreen::Profile(ProfileData::default());
        assert_eq!(profile.screen_type(), "Profile");
    }

    #[test]
    fn test_typed_screen_to_screen_enum() {
        let menu = TypedScreen::Menu(MenuData::default());
        assert_eq!(menu.to_screen_enum(), super::super::Screen::MainMenu);

        let stats = TypedScreen::Statistics(StatisticsData::default());
        assert_eq!(stats.to_screen_enum(), super::super::Screen::Statistics);
    }

    #[test]
    fn test_task_data_key_history() {
        use crate::config::{CursorSpec, Scenario, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Test".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: None,
        };

        let session = GameSession::new(scenario).unwrap();
        let mut task_data = TaskData::new(session);

        assert!(task_data.key_history.is_empty());

        task_data.add_key_to_history("j".to_string());
        assert_eq!(task_data.key_history.len(), 1);
        assert_eq!(task_data.key_history.keys()[0], "j");

        task_data.add_key_to_history("k".to_string());
        assert_eq!(task_data.key_history.len(), 2);
        assert_eq!(task_data.key_history.keys()[0], "k"); // Most recent first

        task_data.clear_key_history();
        assert!(task_data.key_history.is_empty());
    }

    #[test]
    fn test_task_data_key_history_max_5() {
        use crate::config::{CursorSpec, Scenario, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Test".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: None,
        };

        let session = GameSession::new(scenario).unwrap();
        let mut task_data = TaskData::new(session);

        for i in 0..7 {
            task_data.add_key_to_history(format!("key{}", i));
        }

        // Should only keep last 5
        assert_eq!(task_data.key_history.len(), 5);
        assert_eq!(task_data.key_history.keys()[0], "key6"); // Most recent
        assert_eq!(task_data.key_history.keys()[4], "key2");
    }

    #[test]
    fn test_completed_or_abandoned_helpers() {
        use crate::config::{CursorSpec, Scenario, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test_123".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Test".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: None,
        };

        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();

        let coa = CompletedOrAbandoned::Abandoned(abandoned);
        assert!(coa.is_abandoned());
        assert!(!coa.is_completed());
        assert_eq!(coa.scenario_id(), "test_123");
    }

    // CR-007: Test MiniGameModeSelection invalid index handling
    #[test]
    fn test_mode_name_invalid_index() {
        assert_eq!(MiniGameModeSelection::mode_name(3), "Unknown");
        assert_eq!(MiniGameModeSelection::mode_name(100), "Unknown");
        assert_eq!(MiniGameModeSelection::mode_name(usize::MAX), "Unknown");
    }

    #[test]
    fn test_mode_description_invalid_index() {
        assert_eq!(MiniGameModeSelection::mode_description(3), "");
        assert_eq!(MiniGameModeSelection::mode_description(100), "");
        assert_eq!(MiniGameModeSelection::mode_description(usize::MAX), "");
    }

    #[test]
    fn test_selected_mode_invalid_index() {
        let mut selection = MiniGameModeSelection::new();
        selection.selected_index = 99;
        let mode = selection.selected_mode(chrono::Utc::now().date_naive());
        // Should fall back to default Arcade mode
        assert!(mode.is_arcade());
    }

    #[test]
    fn test_minigame_mode_selection_valid_indices() {
        // Test valid mode names
        assert_eq!(MiniGameModeSelection::mode_name(0), "Arcade");
        assert_eq!(MiniGameModeSelection::mode_name(1), "Survival");
        assert_eq!(MiniGameModeSelection::mode_name(2), "Daily Challenge");

        // Test valid mode descriptions
        assert!(!MiniGameModeSelection::mode_description(0).is_empty());
        assert!(!MiniGameModeSelection::mode_description(1).is_empty());
        assert!(!MiniGameModeSelection::mode_description(2).is_empty());

        // Test valid selected modes
        let today = chrono::Utc::now().date_naive();
        let mut selection = MiniGameModeSelection::new();
        selection.selected_index = 0;
        assert!(selection.selected_mode(today).is_arcade());

        selection.selected_index = 1;
        assert!(selection.selected_mode(today).is_survival());

        selection.selected_index = 2;
        assert!(selection.selected_mode(today).is_challenge());
    }

    /// Regression test for #328: `selected_mode`, `mode_name`, and
    /// `mode_description` must never disagree with each other, since all
    /// three are now derived from the single `all_modes` array. If a future
    /// mode variant is added to `all_modes` this loop automatically covers
    /// it too.
    #[test]
    fn test_mode_selection_helpers_agree_with_each_other() {
        let today = chrono::Utc::now().date_naive();
        let mut selection = MiniGameModeSelection::new();

        for index in 0..MiniGameModeSelection::MODE_COUNT {
            selection.selected_index = index;
            let mode = selection.selected_mode(today);

            assert_eq!(MiniGameModeSelection::mode_name(index), mode.name());
            assert_eq!(
                MiniGameModeSelection::mode_description(index),
                mode.description()
            );
        }
    }

    /// Structural backstop for #328: every currently-defined `MiniGameMode`
    /// variant must appear exactly once in `all_modes`. This does not force a
    /// compile error if a future variant is forgotten, but it does fail this
    /// test rather than silently leaving the new mode unselectable.
    #[test]
    fn test_all_modes_covers_every_known_variant_exactly_once() {
        let today = chrono::Utc::now().date_naive();
        let modes = MiniGameModeSelection::all_modes(today);
        assert_eq!(modes.len(), MiniGameModeSelection::MODE_COUNT);

        let arcade_count = modes.iter().filter(|m| m.is_arcade()).count();
        let survival_count = modes.iter().filter(|m| m.is_survival()).count();
        let challenge_count = modes.iter().filter(|m| m.is_challenge()).count();

        assert_eq!(arcade_count, 1);
        assert_eq!(survival_count, 1);
        assert_eq!(challenge_count, 1);
        assert_eq!(arcade_count + survival_count + challenge_count, modes.len());
    }

    #[test]
    fn test_category_filters_data_default() {
        let data = CategoryFiltersData::default();
        assert_eq!(data.selected_index, 0);
        assert_eq!(data.return_to, ReturnDestination::Menu);
    }

    #[test]
    fn test_category_filters_data_with_values() {
        let data = CategoryFiltersData {
            selected_index: 5,
            return_to: ReturnDestination::PausedMiniGame,
        };
        assert_eq!(data.selected_index, 5);
        assert_eq!(data.return_to, ReturnDestination::PausedMiniGame);
    }

    #[test]
    fn test_typed_screen_end_game_summary() {
        let data = EndGameSummaryData {
            scenarios_total: 1,
            perfected: 1,
            imperfect: 0,
            total_completions: 1,
            total_xp: 0,
            level: 1,
            command_success_rate: 1.0,
            journey_days: 0,
            commands_mastered: 0,
            category_breakdown: vec![],
            next_steps: vec![NextStep::ArcadeMode],
        };
        let screen = TypedScreen::EndGameSummary(data);
        assert_eq!(screen.screen_type(), "EndGameSummary");
        assert_eq!(
            screen.to_screen_enum(),
            super::super::Screen::EndGameSummary
        );
    }

    #[test]
    fn test_typed_screen_category_filters() {
        let screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());
        assert_eq!(screen.screen_type(), "CategoryFilters");
        assert_eq!(
            screen.to_screen_enum(),
            super::super::Screen::CategoryFilters
        );
    }
}
