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

use crate::async_state::SaveRequest;
use crate::config::{Difficulty, Scenario, ScenarioCategory, SortMode};
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::PerformanceTracker;
use crate::security::UserError;
use crate::sound::SoundEffect;
use crate::time::Clock;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

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
    AchievementsData, CategoryFiltersData, CommandBufferAccess, CompletedOrAbandoned,
    EndGameSummaryData, InputStateAccess, KeyHistory, MenuData, MiniGameData,
    MiniGameModeSelection, ModeSelectionData, NextStep, ProfileData, ResultsData,
    ReturnDestination, ReviewData, StatisticsData, TaskData, TypedScreen,
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
    /// Achievements screen showing unlocked/locked achievements
    Achievements,
    /// Category filters configuration screen
    CategoryFilters,
    /// Review session screen for spaced repetition
    Review,
    /// Mini-game mode (Arcade Mode)
    MiniGame,
    /// Curriculum-completion summary screen
    EndGameSummary,
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

    /// Mode selection: go back (close submenu)
    ModeSelectionBack,

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
    ///
    /// `keys` is the canonical command to dispatch (post keymap-overlay
    /// translation); `typed` is the physically-pressed key, for
    /// `KeyHistory` display. Equal on every keystroke when the overlay is
    /// disabled or misses.
    MiniGameCommand {
        keys: crate::input::keymap::CanonicalKeys,
        typed: std::borrow::Cow<'static, str>,
    },

    /// Timeout on current mini-game scenario
    MiniGameTimeout,

    /// The mode's session-level time limit elapsed (e.g. Arcade's 60 seconds)
    MiniGameSessionTimeout,

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
    ///
    /// `keys` is the canonical command to dispatch (post keymap-overlay
    /// translation); `typed` is the physically-pressed key, for
    /// `KeyHistory` display. Equal on every keystroke when the overlay is
    /// disabled or misses.
    ExecuteCommand {
        keys: crate::input::keymap::CanonicalKeys,
        typed: std::borrow::Cow<'static, str>,
    },

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

    /// Navigate to achievements screen
    ShowAchievements,

    /// Scroll the achievements list (positive = down, negative = up)
    ScrollAchievements(i32),

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

    /// Navigate to category filters screen
    ShowCategoryFilters,

    /// Move selection up in category filters screen
    CategoryFilterUp,

    /// Move selection down in category filters screen
    CategoryFilterDown,

    /// Toggle selected category filter on/off
    CategoryFilterToggle,

    /// Reset category filters to show all categories
    CategoryFilterSelectAll,

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

    /// Toggle sound on/off
    ToggleSound,
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
        Self::with_config(
            scenarios,
            profile,
            profile_storage,
            performance_tracker,
            ConfigState::default(),
        )
    }

    /// Create a new application state with custom configuration
    ///
    /// # Arguments
    ///
    /// * `scenarios` - The list of available scenarios to play
    /// * `profile` - User profile with XP, level, achievements
    /// * `profile_storage` - Storage for saving/loading profile
    /// * `performance_tracker` - Tracker for command performance and spaced repetition
    /// * `config` - Custom configuration state (e.g., for enabling arrow keys in normal mode)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::config::Scenario;
    /// use helix_trainer::ui::state::ConfigState;
    ///
    /// let scenarios = vec![/* ... */];
    /// let mut config = ConfigState::default();
    /// config.persistent.enable_arrow_keys_in_normal_mode = true;
    /// let state = AppState::with_config(scenarios, profile, storage, tracker, config);
    /// ```
    pub fn with_config(
        scenarios: Vec<Scenario>,
        profile: UserProfile,
        profile_storage: ProfileStorage,
        performance_tracker: PerformanceTracker,
        config: ConfigState,
    ) -> Self {
        Self::with_clock(
            scenarios,
            profile,
            profile_storage,
            performance_tracker,
            config,
            Arc::new(crate::time::SystemClock),
        )
    }

    /// Create a new application state with custom configuration and an explicit clock
    ///
    /// Primarily useful for tests that need deterministic control over day-boundary
    /// and scheduling behavior; see [`crate::time::FakeClock`].
    pub fn with_clock(
        scenarios: Vec<Scenario>,
        profile: UserProfile,
        profile_storage: ProfileStorage,
        performance_tracker: PerformanceTracker,
        config: ConfigState,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::with_clock(
                profile,
                performance_tracker,
                profile_storage,
                clock,
            ),
            config,
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

    /// Wire up the channel used to dispatch mid-session profile saves to
    /// the serialized save writer, off the event-loop thread. See
    /// [`ProgressState::set_save_channel`].
    pub fn set_save_channel(&mut self, tx: mpsc::Sender<SaveRequest>) {
        self.progress.set_save_channel(tx);
    }

    /// Drop this instance's clone of the save channel. See
    /// [`ProgressState::close_save_channel`] — call this on the exit path
    /// before awaiting the save writer's drain.
    pub fn close_save_channel(&mut self) {
        self.progress.close_save_channel();
    }

    /// Prepare the application's exit-time save. See
    /// [`ProgressState::prepare_final_save_request`] for the ordering
    /// guarantee this depends on the caller upholding.
    pub fn prepare_final_save_request(&mut self) -> SaveRequest {
        self.progress.prepare_final_save_request()
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

/// Clamps `MenuData::selected_item` after a filter change may have shrunk the
/// filtered scenario list.
///
/// `ToggleCategoryFilter`/`ToggleDifficultyFilter`/`ToggleCompletedFilter` are dispatched
/// screen-agnostically via `HandlerContext`, which structurally excludes `state.screen`, so
/// the filter handlers themselves cannot perform this clamp. This is a no-op unless the menu
/// screen is currently active. `ui::render::menu::render_main_menu` performs an equivalent
/// clamp on every render as a backstop for paths (e.g. the `CategoryFilterToggle` filter
/// popup) that mutate the filter while `MenuData` isn't the active screen.
fn clamp_menu_selection_to_filtered_count(state: &mut AppState) {
    if let TypedScreen::Menu(ref mut menu_data) = state.screen {
        let max_index =
            handlers::menu::total_menu_items_for_count(state.game.scenario_collection.count())
                .saturating_sub(1);
        menu_data.selected_item = menu_data.selected_item.min(max_index);
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
            let outcome = handlers::handle_quit_app(state)?;
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
            extract_screen!(state, ModeSelection, mut data, ctx => handlers::handle_mode_selection_select(data, &mut ctx))
        }
        Message::ModeSelectionBack => {
            extract_screen!(state, ModeSelection, data => handlers::handle_mode_selection_back(data))
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
        Message::MiniGameCommand { keys, typed } => {
            handlers::handle_minigame_command(state, keys, typed)
        }
        Message::MiniGameTimeout => handlers::handle_minigame_timeout(state),
        Message::MiniGameSessionTimeout => handlers::handle_minigame_session_timeout(state),
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
            let outcome = handlers::handle_minigame_next_scenario(state)?;
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
            // Play success sound for training mode completion
            state
                .progress
                .sound_manager
                .play(SoundEffect::ScenarioComplete);
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::AbandonScenario => {
            // Extract TaskData by replacing screen temporarily
            if let TypedScreen::Task(task_data) =
                std::mem::replace(&mut state.screen, TypedScreen::Menu(MenuData::default()))
            {
                let new_screen = handlers::handle_abandon_scenario(task_data)?;
                // Play failure sound for abandoned scenario
                state
                    .progress
                    .sound_manager
                    .play(SoundEffect::ScenarioFailed);
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
                // Curriculum-completion fanfare - not the routine ScenarioComplete chime,
                // since this fires on every scenario. See `Message::CompleteScenario` above
                // for that call.
                if matches!(&outcome, HandlerOutcome::Transition(s) if matches!(**s, TypedScreen::EndGameSummary(_)))
                {
                    state.progress.sound_manager.play(SoundEffect::LevelUp);
                }
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
        Message::ExecuteCommand { keys, typed } => {
            let outcome = handlers::handle_execute_command(state, keys, typed)?;
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
        Message::ShowAchievements => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_show_achievements(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ScrollAchievements(delta) => {
            extract_screen!(state, Achievements, data => handlers::handle_scroll_achievements(data, delta))
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
            clamp_menu_selection_to_filtered_count(state);
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
            clamp_menu_selection_to_filtered_count(state);
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
            clamp_menu_selection_to_filtered_count(state);
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

        // Category filters screen messages
        Message::ShowCategoryFilters => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_show_category_filters(&mut ctx)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::CategoryFilterUp => {
            extract_screen!(state, CategoryFilters, mut data, ctx => handlers::handle_category_filter_up(data, &ctx))
        }
        Message::CategoryFilterDown => {
            extract_screen!(state, CategoryFilters, mut data, ctx => handlers::handle_category_filter_down(data, &ctx))
        }
        Message::CategoryFilterToggle => {
            extract_screen!(state, CategoryFilters, data, ctx => handlers::handle_category_filter_toggle(data, &mut ctx))
        }
        Message::CategoryFilterSelectAll => {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_category_filter_select_all(&mut ctx)?;
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
        Message::ToggleSound => {
            let enabled = state.progress.sound_manager.toggle();
            tracing::info!("Sound toggled: {}", if enabled { "on" } else { "off" });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
