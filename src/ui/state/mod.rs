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
//! - `TypedScreen`: The current screen being displayed, with its required data
//! - `Message`: User actions and events that trigger state changes
//! - `update()`: Pure function that transforms state based on messages
//!
//! This ensures:
//! - All state changes go through the update function
//! - No hidden side effects in state changes
//! - State transitions are testable and reproducible
//! - UI rendering is pure (no side effects)

use crate::async_state::SaveRequest;
use crate::config::Scenario;
use crate::gamification::{ProfileStorage, QuarantineOutcome, UserProfile};
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

mod effect;
pub use effect::Effect;

// Handler infrastructure for type-safe handlers
mod handler_context;
pub use handler_context::{HandlerContext, HandlerOutcome};

// Sub-structures for organizing AppState
mod substates;
pub use substates::{ConfigState, GameState, ProfilePersistence, ProgressState, UIState};

// Type-safe screen variants with required data
pub mod screen;
pub use screen::{
    AchievementsData, CategoryFiltersData, CategoryProgress, CompletedOrAbandoned,
    EndGameSummaryData, HintPanel, InputStateAccess, KeyHistory, MasteryInfo, MenuData,
    MiniGameData, MiniGameModeSelection, ModeSelectionData, NextStep, ProfileData, ResultsData,
    ReturnDestination, ReviewData, StatisticsData, TaskData, TopMode, TypedScreen,
};

/// Breakdown of XP earned from a scenario
#[derive(Debug, Clone)]
pub struct XPBreakdown {
    /// XP before any bonus or scaling.
    pub base_xp: u64,
    /// Bonus for a perfect score.
    pub perfect_bonus: u64,
    /// Bonus for the first completion of the day.
    pub first_today_bonus: u64,
    /// Combined XP scaling (mastery factor times repeat penalty).
    pub mastery_multiplier: f64,
    /// Scaling from the scenario's mastery level.
    pub mastery_factor: f64,
    /// Scaling from repeated attempts within the session.
    pub repeat_penalty: f64,
    /// XP bonuses granted by quests completed by this run.
    pub quest_bonuses: Vec<QuestBonus>,
    /// Total XP awarded after all bonuses and scaling.
    pub total_xp: u64,
}

/// XP bonus granted for completing a quest
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestBonus {
    /// Quest description shown to the player
    pub quest: String,
    /// Bonus XP awarded
    pub xp: u64,
}

/// Quest progress change (before → after)
#[derive(Debug, Clone)]
pub struct QuestProgressChange {
    /// Description of the affected quest.
    pub quest_description: String,
    /// Progress before the run.
    pub old_progress: u32,
    /// Progress after the run.
    pub new_progress: u32,
}

/// State for review session
#[derive(Debug, Clone)]
pub struct ReviewSessionState {
    /// Commands due for review, in review order.
    pub due_commands: Vec<String>,
    /// Index of the command being reviewed.
    pub current_index: usize,
    /// When the session started.
    pub session_started_at: Instant,
    /// Results recorded so far.
    pub completed_reviews: Vec<ReviewResult>,
}

impl ReviewSessionState {
    /// The command currently being reviewed, `None` once every command is done
    pub fn current_command(&self) -> Option<&str> {
        self.due_commands
            .get(self.current_index)
            .map(String::as_str)
    }
}

/// Whether the player recalled a reviewed command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewOutcome {
    /// The command was recalled correctly
    Recalled,
    /// The command was not recalled
    Forgot,
}

impl ReviewOutcome {
    /// Whether the command was recalled
    pub fn is_recalled(self) -> bool {
        self == Self::Recalled
    }
}

/// Result of a single review
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// Reviewed command.
    pub command: String,
    /// How the review ended.
    pub outcome: ReviewOutcome,
    /// Time the review took.
    pub duration: Duration,
}

/// Screens that can be entered without carrying any session or result data
///
/// Screens that need data (task, results, review, arcade, end-game summary) are
/// entered through their own messages, so they are not representable here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTarget {
    /// Mode selection screen (Training vs Arcade)
    ModeSelection,
    /// Main menu screen (Training Mode)
    MainMenu,
    /// Profile screen showing level, achievements, stats
    Profile,
    /// Statistics screen showing command mastery and analytics
    Statistics,
    /// Achievements screen showing unlocked/locked achievements
    Achievements,
    /// Category filters configuration screen
    CategoryFilters,
}

/// Messages that trigger state updates
///
/// Each message represents a user action or system event that should
/// change the application state. The `update()` function handles all
/// messages in a pure, side-effect-free manner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Navigate to a screen that needs no data
    NavigateTo(NavTarget),

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

    /// Select Arcade Mode (mini-games)
    SelectArcadeMode,

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
        /// Canonical command to dispatch.
        keys: crate::input::keymap::CanonicalKeys,
        /// Physically pressed key, for key-history display.
        typed: std::borrow::Cow<'static, str>,
    },

    /// Timeout on current mini-game scenario
    MiniGameTimeout,

    /// The mode's session-level time limit elapsed (e.g. Arcade's 60 seconds)
    MiniGameSessionTimeout,

    /// Advance to next mini-game scenario (after transition delay)
    MiniGameNextScenario,

    /// Return to mode selection from mini-game
    MiniGameBackToMenu,

    /// Menu navigation: move selection up
    MenuUp,

    /// Menu navigation: move selection down
    MenuDown,

    /// Menu navigation: a key for the multi-key parser (counts, `gg`, `G`, `j`, `k`)
    MenuKey(crossterm::event::KeyEvent),

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
        /// Canonical command to dispatch.
        keys: crate::input::keymap::CanonicalKeys,
        /// Physically pressed key, for key-history display.
        typed: std::borrow::Cow<'static, str>,
    },

    /// Retry the current scenario
    RetryScenario,

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
        /// Command executed, when the progress update follows one.
        command: Option<String>,
        /// Whether the run completed its scenario.
        scenario_completed: bool,
        /// Scenario the update refers to.
        scenario_id: Option<String>,
        /// Time the run took.
        duration: Duration,
    },

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
    CompleteReviewCommand {
        /// Whether the player succeeded or failed.
        outcome: ReviewOutcome,
    },

    /// Abandon the review session
    AbandonReviewSession,

    /// Remove expired notifications from the queue
    CleanupNotifications,

    /// Toggle sound on/off
    ToggleSound,

    /// Replace the quarantined profile with a fresh one (only offered after a backup was made)
    StartFreshProfile,
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
    /// The active arcade session, if one exists.
    ///
    /// Single read path for gameplay code that only needs to inspect the session,
    /// so callers do not reach through `state.game` field by field.
    #[must_use]
    pub fn minigame_session(&self) -> Option<&crate::minigame::MiniGameSession> {
        self.screen.minigame_data().map(|data| &*data.session)
    }

    /// Mutable counterpart of [`AppState::minigame_session`].
    pub fn minigame_session_mut(&mut self) -> Option<&mut crate::minigame::MiniGameSession> {
        self.screen
            .minigame_data_mut()
            .map(|data| &mut *data.session)
    }

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
    /// ```
    /// use helix_trainer::gamification::{ProfileStorage, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::ui::state::TypedScreen;
    ///
    /// let storage = ProfileStorage::with_path(std::env::temp_dir().join("doc-profile.json"));
    /// let state = AppState::new(vec![], UserProfile::new(), storage, PerformanceTracker::new());
    /// assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
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
    /// ```
    /// use helix_trainer::gamification::{ProfileStorage, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::ui::state::ConfigState;
    ///
    /// let mut config = ConfigState::default();
    /// config.persistent.enable_arrow_keys_in_normal_mode = true;
    /// let storage = ProfileStorage::with_path(std::env::temp_dir().join("doc-profile.json"));
    /// let state = AppState::with_config(
    ///     vec![],
    ///     UserProfile::new(),
    ///     storage,
    ///     PerformanceTracker::new(),
    ///     config,
    /// );
    /// assert!(state.config.persistent.enable_arrow_keys_in_normal_mode);
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
    pub fn scenario(&self, index: usize) -> Option<&Scenario> {
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
    /// guarantee this depends on the caller upholding. `None` means the
    /// profile must not be written (still loading, or its load failed).
    pub fn prepare_final_save_request(&mut self) -> Option<SaveRequest> {
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
            Err(UserError::WrongScreen)
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
            Err(UserError::WrongScreen)
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
            Err(UserError::WrongScreen)
        }
    };
}

/// Run a handler that needs only the shared (non-screen) state, then apply its outcome
fn run_with_context(
    state: &mut AppState,
    handler: impl FnOnce(&mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError>,
) -> Result<(), UserError> {
    let mut ctx = HandlerContext::new(
        &mut state.ui,
        &mut state.game,
        &mut state.progress,
        &state.config,
    );
    let outcome = handler(&mut ctx)?;
    apply_outcome(state, outcome);
    Ok(())
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
/// It performs no I/O itself - side effects are returned through `effects`.
///
/// # Arguments
///
/// * `state` - The current application state (will be modified)
/// * `msg` - The message/action that triggered the update
/// * `effects` - Receives the side effects (sound, saves) the update requests; filled even when
///   the update returns an error
///
/// # Errors
///
/// Returns `UserError` if state validation fails (e.g., invalid scenario)
///
/// # Examples
///
/// ```
/// use helix_trainer::gamification::{ProfileStorage, UserProfile};
/// use helix_trainer::learning::PerformanceTracker;
/// use helix_trainer::ui::{AppState, Message, update};
///
/// let storage = ProfileStorage::with_path(std::env::temp_dir().join("doc-profile.json"));
/// let mut state = AppState::new(vec![], UserProfile::new(), storage, PerformanceTracker::new());
/// let mut effects = Vec::new();
/// update(&mut state, Message::QuitApp, &mut effects)?;
/// assert!(!state.ui.running);
/// # Ok::<(), helix_trainer::security::UserError>(())
/// ```
pub fn update(
    state: &mut AppState,
    msg: Message,
    effects: &mut Vec<Effect>,
) -> Result<(), UserError> {
    let result = dispatch(state, msg);
    // Delivered on the error path too: effects pushed before a failing step must not be lost
    effects.append(&mut state.ui.effects);
    result
}

/// Handle `msg` without draining the effect queue, for handlers that dispatch follow-up messages
pub(crate) fn dispatch(state: &mut AppState, msg: Message) -> Result<(), UserError> {
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
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let outcome = handlers::handle_back_to_menu(&mut state.screen, &mut ctx)?;
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
        Message::SelectArcadeMode => run_with_context(state, handlers::handle_select_arcade_mode),

        // Mini-game messages
        Message::PauseMiniGame => {
            extract_screen!(state, MiniGame, data => handlers::handle_pause_minigame(data))
        }
        Message::ResumeMiniGame => {
            extract_screen!(state, MiniGame, data => handlers::handle_resume_minigame(data))
        }
        Message::MiniGameTick => {
            extract_screen!(state, MiniGame, mut data, ctx => handlers::handle_minigame_tick(data, &mut ctx))
        }
        Message::MiniGameCommand { keys, typed } => {
            handlers::handle_minigame_command(state, keys, typed)
        }
        Message::MiniGameTimeout => handlers::handle_minigame_timeout(state),
        Message::MiniGameSessionTimeout => handlers::handle_minigame_session_timeout(state),
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
        Message::MenuKey(key) => {
            extract_screen!(state, Menu, mut data, ctx => handlers::menu::handle_menu_key(data, &ctx, key))
        }
        Message::MenuSelect => {
            let TypedScreen::Menu(ref data) = state.screen else {
                return Err(UserError::WrongScreen);
            };
            let outcome = handlers::menu::handle_menu_select(data.selected_item, state)?;
            apply_outcome(state, outcome);
            Ok(())
        }

        // Scenario lifecycle messages
        Message::CompleteScenario => {
            // This handler needs full AppState access to call update() for quest progress
            let outcome = handlers::handle_complete_scenario(state)?;
            // Play success sound for training mode completion
            state
                .ui
                .effects
                .push(Effect::Play(SoundEffect::ScenarioComplete));
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
                    .ui
                    .effects
                    .push(Effect::Play(SoundEffect::ScenarioFailed));
                state.screen = new_screen;
                Ok(())
            } else {
                Err(UserError::WrongScreen)
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
                Err(UserError::WrongScreen)
            }
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
                    state.ui.effects.push(Effect::Play(SoundEffect::LevelUp));
                }
                apply_outcome(state, outcome);
                Ok(())
            } else {
                Err(UserError::WrongScreen)
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
                Err(UserError::WrongScreen)
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
            let outcome = handlers::handle_show_profile(&mut state.screen)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ShowStatistics => {
            let outcome = handlers::handle_show_statistics(&mut state.screen)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::ShowAchievements => {
            let outcome = handlers::handle_show_achievements(&mut state.screen)?;
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

        // Category filters screen messages
        Message::ShowCategoryFilters => {
            let outcome = handlers::handle_show_category_filters(&mut state.screen)?;
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
            run_with_context(state, handlers::handle_category_filter_select_all)
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
            if outcome.is_transition() && state.minigame_session().is_some() {
                // Starting a review leaves a paused arcade run behind for good
                handlers::handle_minigame_game_over(state)?;
            }
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::CompleteReviewCommand { outcome } => {
            extract_screen!(state, Review, mut data, ctx => handlers::handle_complete_review_command(data, &mut ctx, outcome))
        }
        Message::AbandonReviewSession => {
            run_with_context(state, handlers::handle_abandon_review_session)
        }

        // Notification messages
        Message::CleanupNotifications => {
            let outcome = handlers::handle_cleanup_notifications(&mut state.ui)?;
            apply_outcome(state, outcome);
            Ok(())
        }
        Message::StartFreshProfile => {
            if matches!(
                state.progress.persistence(),
                ProfilePersistence::Quarantined(QuarantineOutcome::BackedUp(_))
            ) {
                state.ui.effects.push(Effect::StartFreshProfile);
            }
            Ok(())
        }
        Message::ToggleSound => {
            let enabled = state.progress.toggle_sound();
            tracing::info!("Sound toggled: {}", if enabled { "on" } else { "off" });
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
