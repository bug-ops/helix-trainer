//! UI state management using Elm Architecture
//!
//! This module implements the Elm Architecture pattern for the TUI application.
//! It provides a centralized AppState with pure update functions, enabling
//! predictable state transitions and easy testing.
//!
//! # Architecture
//!
//! The Elm Architecture pattern consists of:
//! - `AppState`: The complete application state
//! - `Screen`: The current screen being displayed
//! - `Message`: User actions and events that trigger state changes
//! - `update()`: Pure function that transforms state based on messages
//!
//! This ensures:
//! - All state changes go through the update function
//! - No hidden side effects in state changes
//! - State transitions are testable and reproducible
//! - UI rendering is pure (no side effects)

use crate::config::{Difficulty, Scenario, ScenarioCategory, SortMode};
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::PerformanceTracker;
use crate::security::UserError;
use std::fmt;
use std::time::{Duration, Instant};

// Message handlers in separate modules
mod handlers;

// Handler infrastructure for type-safe handlers
mod handler_context;
pub use handler_context::{HandlerContext, HandlerOutcome};

// Sub-structures for organizing AppState
mod substates;
pub use substates::{ConfigState, GameState, ProgressState, UIState};

// Type-safe screen variants with required data
pub mod screen;
pub use screen::{
    CommandBufferAccess, CompletedOrAbandoned, InputStateAccess, KeyHistory, MenuData,
    MiniGameData, ModeSelectionData, ProfileData, ResultsData, ReturnDestination, ReviewData,
    StatisticsData, TaskData, TypedScreen,
};

/// Breakdown of XP earned from a scenario
#[derive(Debug, Clone)]
pub struct XPBreakdown {
    pub base_xp: u64,
    pub perfect_bonus: u64,
    pub first_today_bonus: u64,
    pub mastery_multiplier: f64, // Combined XP scaling (mastery * repeat)
    pub mastery_factor: f64,     // Just mastery level factor
    pub repeat_penalty: f64,     // Just session repeat penalty
    pub quest_bonuses: Vec<(String, u64)>, // (quest description, bonus xp)
    pub total_xp: u64,
}

/// Quest progress change (before → after)
#[derive(Debug, Clone)]
pub struct QuestProgressChange {
    pub quest_description: String,
    pub old_progress: u32,
    pub new_progress: u32,
}

/// State for review session
#[derive(Debug, Clone)]
pub struct ReviewSessionState {
    pub due_commands: Vec<String>,
    pub current_index: usize,
    pub current_command: Option<String>,
    pub session_started_at: Instant,
    pub completed_reviews: Vec<ReviewResult>,
}

/// Result of a single review
#[derive(Debug, Clone)]
pub struct ReviewResult {
    pub command: String,
    pub success: bool,
    pub duration: Duration,
}

/// The current screen being displayed in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Mode selection screen (Training vs Arcade)
    ModeSelection,
    /// Main menu screen (Training Mode)
    MainMenu,
    /// Task/scenario gameplay screen
    Task,
    /// Results screen after scenario completion
    Results,
    /// Profile screen showing level, achievements, stats
    Profile,
    /// Statistics screen showing command mastery and analytics
    Statistics,
    /// Review session screen for spaced repetition
    Review,
    /// Mini-game mode (Arcade Mode)
    MiniGame,
}

/// Messages that trigger state updates
///
/// Each message represents a user action or system event that should
/// change the application state. The `update()` function handles all
/// messages in a pure, side-effect-free manner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Navigate to a specific screen
    NavigateTo(Screen),

    /// Quit the application
    QuitApp,

    /// Mode selection: move selection up
    ModeSelectionUp,

    /// Mode selection: move selection down
    ModeSelectionDown,

    /// Mode selection: select current mode
    ModeSelectionSelect,

    /// Select Training Mode (manual scenario selection)
    SelectTrainingMode,

    /// Select Arcade Mode (mini-games)
    SelectArcadeMode,

    /// Start mini-game session
    StartMiniGame,

    /// Pause mini-game
    PauseMiniGame,

    /// Resume mini-game from paused state
    ResumeMiniGame,

    /// Mini-game timer tick (100ms interval)
    MiniGameTick,

    /// Execute Helix command during mini-game
    MiniGameCommand(std::borrow::Cow<'static, str>),

    /// Timeout on current mini-game scenario
    MiniGameTimeout,

    /// Current mini-game scenario completed
    MiniGameScenarioComplete,

    /// Advance to next mini-game scenario (after transition delay)
    MiniGameNextScenario,

    /// Return to mode selection from mini-game
    MiniGameBackToMenu,

    /// Menu navigation: move selection up
    MenuUp,

    /// Menu navigation: move selection down
    MenuDown,

    /// Menu navigation: move up by N items
    MenuUpBy(usize),

    /// Menu navigation: move down by N items
    MenuDownBy(usize),

    /// Menu navigation: jump to first item (gg)
    MenuJumpToFirst,

    /// Menu navigation: jump to last item (G)
    MenuJumpToLast,

    /// Menu navigation: jump to item N (1-indexed, like Helix)
    MenuJumpTo(usize),

    /// Menu action: select current menu item
    MenuSelect,

    /// Start a scenario at the given index
    StartScenario(usize),

    /// Mark the current scenario as complete
    CompleteScenario,

    /// Abandon the current scenario
    AbandonScenario,

    /// Request to show the next hint
    ShowHint,

    /// Execute a Helix command during gameplay
    ExecuteCommand(std::borrow::Cow<'static, str>),

    /// Retry the current scenario
    RetryScenario,

    /// Move to next scenario
    NextScenario,

    /// Navigate to next lesson from results screen (sequential navigation)
    NextLesson,

    /// Navigate to scenario list (Menu screen) from results screen
    GoToScenarioList,

    /// Return to main menu
    BackToMenu,

    /// Navigate to profile screen
    ShowProfile,

    /// Navigate to statistics screen
    ShowStatistics,

    /// Award XP to the user
    AwardXP { amount: u64 },

    /// Update quest progress based on gameplay
    UpdateQuestProgress {
        command: Option<String>,
        scenario_completed: bool,
        scenario_id: Option<String>,
        duration: Duration,
    },

    /// Set sort mode for scenario list
    SetSortMode(SortMode),

    /// Toggle category filter (add/remove from active filters)
    ToggleCategoryFilter(ScenarioCategory),

    /// Toggle difficulty filter (add/remove from active filters)
    ToggleDifficultyFilter(Difficulty),

    /// Toggle completed scenarios filter
    ToggleCompletedFilter,

    /// Reset all filters to default
    ResetFilters,

    /// Start a review session
    StartReviewSession,

    /// Complete current review command (mark as success or failure)
    CompleteReviewCommand { success: bool },

    /// Move to next review command
    NextReviewCommand,

    /// Abandon the review session
    AbandonReviewSession,

    /// Show a notification (level-up, achievement, quest complete, streak)
    ShowNotification(crate::ui::notification::Notification),

    /// Remove expired notifications from the queue
    CleanupNotifications,
}

/// Main application state
///
/// Contains all the data needed to render the UI and handle user interactions.
/// This is the single source of truth for the application.
///
/// Structure:
/// - `screen`: Type-safe screen with required data (TypedScreen)
/// - `ui`: Global UI rendering state (running, completion_time)
/// - `game`: Game scenarios collection
/// - `progress`: User progress (profile, learning, achievements)
/// - `config`: Application configuration (filters, settings)
pub struct AppState {
    /// Current screen with type-safe data
    pub screen: TypedScreen,

    /// Global UI rendering and display state
    pub ui: UIState,

    /// Game scenarios collection
    pub game: GameState,

    /// User progress (profile, learning, achievements)
    pub progress: ProgressState,

    /// Application configuration (filters, settings)
    pub config: ConfigState,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("screen", &self.screen.screen_type())
            .field("ui", &self.ui)
            .field("game", &self.game)
            .field("progress", &self.progress)
            .field("config", &self.config)
            .finish()
    }
}

impl AppState {
    /// Create a new application state with the given scenarios
    ///
    /// # Arguments
    ///
    /// * `scenarios` - The list of available scenarios to play
    /// * `profile` - User profile with XP, level, achievements
    /// * `profile_storage` - Storage for saving/loading profile
    /// * `performance_tracker` - Tracker for command performance and spaced repetition
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::config::Scenario;
    ///
    /// let scenarios = vec![/* ... */];
    /// let state = AppState::new(scenarios, profile, storage, tracker);
    /// assert!(matches!(state.screen, TypedScreen::Menu(_)));
    /// ```
    pub fn new(
        scenarios: Vec<Scenario>,
        profile: UserProfile,
        profile_storage: ProfileStorage,
        performance_tracker: PerformanceTracker,
    ) -> Self {
        Self {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(profile, performance_tracker, profile_storage),
            config: ConfigState::default(),
        }
    }

    /// Get the number of available scenarios (filtered count)
    pub fn scenario_count(&self) -> usize {
        self.game.scenario_collection.count()
    }

    /// Get a scenario by filtered index
    pub fn get_scenario(&self, index: usize) -> Option<&Scenario> {
        self.game.scenario_collection.get_filtered_by_index(index)
    }

    /// Save profile with debouncing (only if enough time has passed)
    ///
    /// # Errors
    ///
    /// Returns error if save operation fails
    // NOTE: Debounce saves to reduce I/O overhead (5-second delay)
    // OPTIMIZE: Performance audit suggested this optimization (50-80% I/O reduction)
    pub fn save_profile_debounced(&mut self) -> Result<(), crate::gamification::GamificationError> {
        if !self.progress.should_save() {
            return Ok(());
        }

        self.progress.storage.save(&self.progress.profile)?;
        self.progress.mark_saved();

        Ok(())
    }

    /// Force immediate save (for level-up, achievements, exit)
    ///
    /// # Errors
    ///
    /// Returns error if save operation fails
    pub fn save_profile_immediate(&mut self) -> Result<(), crate::gamification::GamificationError> {
        self.progress.storage.save(&self.progress.profile)?;
        self.progress.mark_saved();
        Ok(())
    }
}

/// Helper macro to extract screen data and call type-safe handler
///
/// This macro handles the pattern of:
/// 1. Extract screen data from TypedScreen variant
/// 2. Call handler with extracted data
/// 3. Apply HandlerOutcome (transition or stay)
macro_rules! extract_screen {
    // Data-only handler (no context)
    ($state:expr, $variant:ident, $data:ident => $handler:expr) => {
        if let TypedScreen::$variant(ref mut $data) = $state.screen {
            let outcome = $handler?;
            apply_outcome($state, outcome);
            Ok(())
        } else {
            Err(UserError::invalid_state(concat!(
                "Expected ",
                stringify!($variant),
                " screen"
            )))
        }
    };
    // Handler with context (immutable data)
    ($state:expr, $variant:ident, $data:ident, $ctx:ident => $handler:expr) => {
        if let TypedScreen::$variant(ref $data) = $state.screen {
            #[allow(unused_mut)]
            let mut $ctx = HandlerContext::new(
                &mut $state.ui,
                &mut $state.game,
                &mut $state.progress,
                &$state.config,
            );
            let outcome = $handler?;
            apply_outcome($state, outcome);
            Ok(())
        } else {
            Err(UserError::invalid_state(concat!(
                "Expected ",
                stringify!($variant),
                " screen"
            )))
        }
    };
    // Handler with context (mutable data)
    ($state:expr, $variant:ident, mut $data:ident, $ctx:ident => $handler:expr) => {
        if let TypedScreen::$variant(ref mut $data) = $state.screen {
            #[allow(unused_mut)]
            let mut $ctx = HandlerContext::new(
                &mut $state.ui,
                &mut $state.game,
                &mut $state.progress,
                &$state.config,
            );
            let outcome = $handler?;
            apply_outcome($state, outcome);
            Ok(())
        } else {
            Err(UserError::invalid_state(concat!(
                "Expected ",
                stringify!($variant),
                " screen"
            )))
        }
    };
}

/// Apply a HandlerOutcome to the state
///
/// If the outcome is a Transition, update the screen.
/// If it's Stay, do nothing.
fn apply_outcome(state: &mut AppState, outcome: HandlerOutcome) {
    if let HandlerOutcome::Transition(boxed_screen) = outcome {
        state.screen = *boxed_screen;
    }
}

/// Pure update function for state transitions
///
/// This function is the heart of the Elm Architecture pattern.
/// It takes the current state and a message, and returns the new state.
/// It has no side effects - all effects are handled elsewhere.
///
/// # Arguments
///
/// * `state` - The current application state (will be modified)
/// * `msg` - The message/action that triggered the update
///
/// # Errors
///
/// Returns `UserError` if state validation fails (e.g., invalid scenario)
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::ui::{AppState, Screen, Message, update};
///
/// let mut state = AppState::new(vec![]);
/// update(&mut state, Message::QuitApp)?;
/// assert!(!state.ui.running);
/// # Ok::<(), helix_trainer::security::UserError>(())
/// ```
pub fn update(state: &mut AppState, msg: Message) -> Result<(), UserError> {
    match msg {
        // Navigation messages (don't require specific screen)
        Message::QuitApp => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_quit_app(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::NavigateTo(screen) => {
            let outcome = handlers::handle_navigate_to(screen)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::BackToMenu => {
            // Need to access current screen before creating context
            let current_screen_ref = &state.screen;
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_back_to_menu(current_screen_ref, &mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Mode selection messages
        Message::ModeSelectionUp => {
            extract_screen!(state, ModeSelection, data => handlers::handle_mode_selection_up(data))
        }
        Message::ModeSelectionDown => {
            extract_screen!(state, ModeSelection, data => handlers::handle_mode_selection_down(data))
        }
        Message::ModeSelectionSelect => {
            extract_screen!(state, ModeSelection, data, ctx => handlers::handle_mode_selection_select(data, &mut ctx))
        }
        Message::SelectTrainingMode => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_select_training_mode(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::SelectArcadeMode => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_select_arcade_mode(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::StartMiniGame => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_start_minigame(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Mini-game messages
        Message::PauseMiniGame => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_pause_minigame(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ResumeMiniGame => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_resume_minigame(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::MiniGameTick => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_minigame_tick(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::MiniGameCommand(command) => handlers::handle_minigame_command(state, command),
        Message::MiniGameTimeout => handlers::handle_minigame_timeout(state),
        Message::MiniGameScenarioComplete => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_minigame_scenario_complete(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::MiniGameNextScenario => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_minigame_next_scenario(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::MiniGameBackToMenu => {
            let outcome = handlers::handle_minigame_back_to_menu(state)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Menu messages
        Message::MenuUp => {
            extract_screen!(state, Menu, data => handlers::handle_menu_up(data))
        }
        Message::MenuDown => {
            extract_screen!(state, Menu, mut data, ctx => handlers::handle_menu_down(data, &ctx))
        }
        Message::MenuUpBy(count) => {
            extract_screen!(state, Menu, data => handlers::menu::handle_menu_up_by(data, count))
        }
        Message::MenuDownBy(count) => {
            extract_screen!(state, Menu, mut data, ctx => handlers::menu::handle_menu_down_by(data, &ctx, count))
        }
        Message::MenuJumpToFirst => {
            extract_screen!(state, Menu, data => handlers::menu::handle_menu_jump_to_first(data))
        }
        Message::MenuJumpToLast => {
            extract_screen!(state, Menu, mut data, ctx => handlers::menu::handle_menu_jump_to_last(data, &ctx))
        }
        Message::MenuJumpTo(line) => {
            extract_screen!(state, Menu, mut data, ctx => handlers::menu::handle_menu_jump_to(data, &ctx, line))
        }
        Message::MenuSelect => {
            let (selected_item, scroll_offset, command_buffer) =
                if let TypedScreen::Menu(ref data) = state.screen {
                    (
                        data.selected_item,
                        data.scroll_offset,
                        data.command_buffer.clone(),
                    )
                } else {
                    return Err(UserError::invalid_state("Expected Menu screen"));
                };

            let menu_data = MenuData {
                selected_item,
                scroll_offset,
                command_buffer,
            };
            let outcome = handlers::menu::handle_menu_select(&menu_data, state)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Scenario lifecycle messages
        Message::StartScenario(index) => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_start_scenario(&mut ctx, index)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::CompleteScenario => {
            // This handler needs full AppState access to call update() for quest progress
            let outcome = handlers::handle_complete_scenario(state)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::AbandonScenario => {
            // Extract TaskData by replacing screen temporarily
            if let TypedScreen::Task(task_data) =
                std::mem::replace(&mut state.screen, TypedScreen::Menu(MenuData::default()))
            {
                let new_screen = handlers::handle_abandon_scenario(task_data)?;
                state.screen = new_screen;
                Ok(())
            } else {
                Err(UserError::invalid_state(
                    "Expected Task screen for AbandonScenario",
                ))
            }
        }
        Message::RetryScenario => {
            // Extract ResultsData by replacing screen temporarily
            if let TypedScreen::Results(results_data) =
                std::mem::replace(&mut state.screen, TypedScreen::Menu(MenuData::default()))
            {
                let mut ctx = HandlerContext::new(
                    &mut state.ui,
                    &mut state.game,
                    &mut state.progress,
                    &state.config,
                );
                let new_screen = handlers::handle_retry_scenario(results_data, &mut ctx)?;
                state.screen = new_screen;
                Ok(())
            } else {
                Err(UserError::invalid_state(
                    "Expected Results screen for RetryScenario",
                ))
            }
        }
        Message::NextScenario => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_next_scenario(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::NextLesson => {
            if let TypedScreen::Results(ref results_data) = state.screen {
                let mut ctx = HandlerContext::new(
                    &mut state.ui,
                    &mut state.game,
                    &mut state.progress,
                    &state.config,
                );
                let outcome = handlers::handle_next_lesson(results_data, &mut ctx)?;
                apply_outcome(state, outcome);
                Ok(())
            } else {
                Err(UserError::invalid_state(
                    "Expected Results screen for NextLesson",
                ))
            }
        }
        Message::GoToScenarioList => {
            if let TypedScreen::Results(ref results_data) = state.screen {
                let mut ctx = HandlerContext::new(
                    &mut state.ui,
                    &mut state.game,
                    &mut state.progress,
                    &state.config,
                );
                let outcome = handlers::handle_go_to_scenario_list(results_data, &mut ctx)?;
                apply_outcome(state, outcome);
                Ok(())
            } else {
                Err(UserError::invalid_state(
                    "Expected Results screen for GoToScenarioList",
                ))
            }
        }

        // Gameplay messages
        Message::ShowHint => {
            let outcome = handlers::handle_show_hint(state)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ExecuteCommand(command) => {
            let outcome = handlers::handle_execute_command(state, command)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Profile messages
        Message::ShowProfile => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_show_profile(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ShowStatistics => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_show_statistics(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::AwardXP { amount } => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handlers::handle_award_xp(&mut ctx, amount)
        }

        // Quest messages
        Message::UpdateQuestProgress {
            command,
            scenario_completed,
            scenario_id,
            duration,
        } => handlers::handle_update_quest_progress(
            state,
            command,
            scenario_completed,
            scenario_id,
            duration,
        ),

        // Filter messages
        Message::SetSortMode(mode) => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_set_sort_mode(&mut ctx, mode)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ToggleCategoryFilter(category) => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_toggle_category_filter(&mut ctx, category)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ToggleDifficultyFilter(difficulty) => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_toggle_difficulty_filter(&mut ctx, difficulty)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ToggleCompletedFilter => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_toggle_completed_filter(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ResetFilters => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_reset_filters(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Review session messages
        Message::StartReviewSession => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_start_review_session(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::CompleteReviewCommand { success } => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_complete_review_command(&mut ctx, success)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::NextReviewCommand => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_next_review_command(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::AbandonReviewSession => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_abandon_review_session(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Notification messages
        Message::ShowNotification(notification) => {
            let outcome = handlers::handle_show_notification(&mut state.ui, notification)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::CleanupNotifications => {
            let outcome = handlers::handle_cleanup_notifications(&mut state.ui)?;
            apply_outcome(state, outcome);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
