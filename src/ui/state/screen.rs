//! Type-safe screen variants with required data
//!
//! This module implements the TypedScreen pattern, where each screen variant
//! carries the data required for that screen. This makes invalid states
//! unrepresentable at compile time - you can't be on the Task screen without
//! an active GameSession.

use crate::game::{Abandoned, Active, Completed, Feedback, GameSession};
use crate::learning::ScenarioMastery;
use crate::ui::state::{QuestProgressChange, ReviewSessionState, XPBreakdown};

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

    /// Review session screen
    Review(ReviewData),

    /// Mini-game mode (arcade-style training)
    MiniGame(MiniGameData),
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
            Self::Review(_) => "Review",
            Self::MiniGame(_) => "MiniGame",
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
            Self::Review(_) => super::Screen::Review,
            Self::MiniGame(_) => super::Screen::MiniGame,
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
}

/// Data required for task screen (contains active session)
#[derive(Debug)]
pub struct TaskData {
    /// Active game session (guaranteed to exist)
    pub session: GameSession<Active>,

    /// History of last 5 keypresses (most recent first)
    pub key_history: Vec<String>,

    /// Current hint being displayed
    pub current_hint: Option<String>,

    /// Whether to show hint panel
    pub show_hint_panel: bool,

    /// Command buffer for multi-key commands (e.g., "d" waiting for "d")
    pub command_buffer: String,

    /// Last command executed (for display)
    pub last_command: Option<String>,
}

impl TaskData {
    /// Create new TaskData with an active session
    pub fn new(session: GameSession<Active>) -> Self {
        Self {
            session,
            key_history: Vec::with_capacity(5),
            current_hint: None,
            show_hint_panel: false,
            command_buffer: String::new(),
            last_command: None,
        }
    }

    /// Add a key to the history (keeps last 5)
    pub fn add_key_to_history(&mut self, key: String) {
        // Insert at the beginning (most recent first)
        self.key_history.insert(0, key);

        // Keep only last 5 keys
        if self.key_history.len() > 5 {
            self.key_history.truncate(5);
        }
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
}

impl ResultsData {
    /// Create new ResultsData from a completed session
    pub fn from_completed(
        session: GameSession<Completed>,
        feedback: Feedback,
    ) -> Result<Self, crate::security::SecurityError> {
        Ok(Self {
            session: CompletedOrAbandoned::Completed(session),
            feedback,
            xp_breakdown: None,
            quest_changes: Vec::new(),
            scenario_mastery: None,
        })
    }

    /// Create new ResultsData from an abandoned session
    pub fn from_abandoned(session: GameSession<Abandoned>, feedback: Feedback) -> Self {
        Self {
            session: CompletedOrAbandoned::Abandoned(session),
            feedback,
            xp_breakdown: None,
            quest_changes: Vec::new(),
            scenario_mastery: None,
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

/// Data required for mode selection screen
#[derive(Debug, Clone, Default)]
pub struct ModeSelectionData {
    /// Index of selected mode (0 = Training, 1 = Arcade)
    pub selected_mode: usize,
}

/// Data required for mini-game screen
#[derive(Debug, Clone, Default)]
pub struct MiniGameData {
    /// Command buffer for multi-key commands (e.g., "g" waiting for "g")
    pub command_buffer: String,
}

/// Trait for types that manage a command buffer for multi-key commands
///
/// Both training mode (TaskData) and arcade mode (MiniGameData) use command
/// buffers to handle multi-key sequences like `dd`, `gg`, and `rx`.
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

impl CommandBufferAccess for TaskData {
    fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    fn command_buffer_mut(&mut self) -> &mut String {
        &mut self.command_buffer
    }
}

impl CommandBufferAccess for MiniGameData {
    fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    fn command_buffer_mut(&mut self) -> &mut String {
        &mut self.command_buffer
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
        use crate::config::{Scenario, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Test".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
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
        assert_eq!(task_data.key_history[0], "j");

        task_data.add_key_to_history("k".to_string());
        assert_eq!(task_data.key_history.len(), 2);
        assert_eq!(task_data.key_history[0], "k"); // Most recent first

        task_data.clear_key_history();
        assert!(task_data.key_history.is_empty());
    }

    #[test]
    fn test_task_data_key_history_max_5() {
        use crate::config::{Scenario, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Test".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
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
        assert_eq!(task_data.key_history[0], "key6"); // Most recent
        assert_eq!(task_data.key_history[4], "key2");
    }

    #[test]
    fn test_completed_or_abandoned_helpers() {
        use crate::config::{Scenario, ScoringConfig, Setup, Solution, TargetState};

        let scenario = Scenario {
            id: "test_123".to_string(),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Test".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
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
}
