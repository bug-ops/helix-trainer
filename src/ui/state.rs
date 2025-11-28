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

use crate::config::Scenario;
use crate::game::GameSession;
use crate::gamification::{ProfileStorage, QuestType, UserProfile};
use crate::learning::{PerformanceTracker, ScenarioMastery, Scheduler};
use crate::security::UserError;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Breakdown of XP earned from a scenario
#[derive(Debug, Clone)]
pub struct XPBreakdown {
    pub base_xp: u64,
    pub perfect_bonus: u64,
    pub first_today_bonus: u64,
    pub mastery_multiplier: f64,           // Mastery-based XP scaling
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

/// The current screen being displayed in the UI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Main menu screen
    MainMenu,
    /// Task/scenario gameplay screen
    Task,
    /// Results screen after scenario completion
    Results,
    /// Profile screen showing level, achievements, stats
    Profile,
    /// Statistics screen showing command mastery and analytics
    Statistics,
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

    /// Menu navigation: move selection up
    MenuUp,

    /// Menu navigation: move selection down
    MenuDown,

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
        duration: Duration,
    },
}

/// Main application state
///
/// Contains all the data needed to render the UI and handle user interactions.
/// This is the single source of truth for the application.
///
/// Note: This doesn't derive Clone because GameSession doesn't implement Clone.
/// Instead, we implement Debug manually.
///
/// # Memory Layout
///
/// Fields are ordered for optimal memory layout and cache efficiency:
/// 1. Large allocations first (Vec, Option<GameSession>)
/// 2. Frequently accessed fields next (screen, session)
/// 3. Medium-sized types (Option<String>, String)
/// 4. Small types last (usize, bool, enum)
pub struct AppState {
    /// All available scenarios
    /// Size: 24 bytes (Vec) - placed first for alignment
    pub scenarios: Vec<Scenario>,

    /// Active game session (Some if on Task screen)
    /// Size: ~200+ bytes - large type, placed early
    pub session: Option<GameSession>,

    /// The current hint being displayed (if any)
    /// Size: 24-32 bytes (Option<String>)
    pub current_hint: Option<String>,

    /// Last command executed (for display purposes)
    /// Size: 24-32 bytes (Option<String>)
    pub last_command: Option<String>,

    /// History of last 5 key presses (most recent first)
    /// Size: 24 bytes (Vec)
    pub key_history: Vec<String>,

    /// Command buffer for accumulating multi-key commands (e.g., "d" waiting for "d")
    /// Size: 24 bytes (String)
    pub command_buffer: String,

    /// Time when scenario was completed (for showing success screen before results)
    /// Size: 16 bytes (Option<Instant>)
    pub completion_time: Option<std::time::Instant>,

    /// Index of the currently selected menu item
    /// Size: 8 bytes (usize)
    pub selected_menu_item: usize,

    /// Scroll offset for menu list (top visible item index)
    /// Size: 8 bytes (usize)
    pub menu_scroll_offset: usize,

    /// The screen currently being displayed
    /// Size: 1 byte (enum)
    pub screen: Screen,

    /// Whether the application is running
    /// Size: 1 byte (bool)
    pub running: bool,

    /// Whether to show hint on task screen
    /// Size: 1 byte (bool)
    pub show_hint_panel: bool,

    /// Whether to show key history popup
    /// Size: 1 byte (bool)
    pub show_key_history: bool,

    // NEW: Gamification fields
    /// User profile with XP, level, achievements, quests
    /// Size: 8 bytes (Rc pointer)
    pub profile: Rc<RefCell<UserProfile>>,

    /// Profile storage for saving/loading
    /// Size: ~24 bytes (PathBuf inside)
    pub profile_storage: ProfileStorage,

    // NEW: Learning fields
    /// Performance tracker for spaced repetition
    /// Size: 8 bytes (Rc pointer)
    pub performance_tracker: Rc<RefCell<PerformanceTracker>>,

    /// Scheduler for review sessions
    /// Size: 8 bytes (Rc pointer)
    pub scheduler: Scheduler,

    // NEW: UI state for progression
    /// Number of scenarios completed today (for first-today bonus)
    /// Size: 8 bytes (usize)
    pub scenarios_completed_today: usize,

    /// Last save time for debounced saves
    /// Size: 16 bytes (Option<Instant>)
    pub last_save_time: Option<Instant>,

    /// Session start time for tracking playtime
    /// Size: 16 bytes (Instant)
    pub session_start_time: Instant,

    /// Unique commands used today for exploration quests
    /// Size: 24+ bytes (HashSet)
    pub commands_used_today: HashSet<String>,

    /// XP breakdown from last scenario (for results display)
    /// Size: ~40+ bytes (Option<XPBreakdown>)
    pub xp_breakdown: Option<XPBreakdown>,

    /// Quest progress changes during last scenario
    /// Size: 24 bytes (Vec)
    pub quest_progress_changes: Vec<QuestProgressChange>,

    /// Previously completed quest IDs (to detect new completions)
    /// Size: 24+ bytes (HashSet)
    pub previously_completed_quests: HashSet<String>,

    /// Scenario mastery info for last completion (for results display)
    /// Size: 16 bytes (Option<(enum, f64)>)
    pub scenario_mastery: Option<(ScenarioMastery, f64)>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("screen", &self.screen)
            .field("session", &"<GameSession>")
            .field("scenarios", &self.scenarios.len())
            .field("selected_menu_item", &self.selected_menu_item)
            .field("running", &self.running)
            .field("current_hint", &self.current_hint.is_some())
            .field("show_hint_panel", &self.show_hint_panel)
            .field("show_key_history", &self.show_key_history)
            .field("last_command", &self.last_command)
            .field("completion_time", &self.completion_time.is_some())
            .field("key_history", &self.key_history.len())
            .field("command_buffer", &self.command_buffer)
            .field("profile", &"<Rc<RefCell<UserProfile>>>")
            .field("performance_tracker", &"<Rc<RefCell<PerformanceTracker>>>")
            .field("scenarios_completed_today", &self.scenarios_completed_today)
            .field("last_save_time", &self.last_save_time.is_some())
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
    /// * `scheduler` - Scheduler for review sessions
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::ui::AppState;
    /// use helix_trainer::config::Scenario;
    ///
    /// let scenarios = vec![/* ... */];
    /// let state = AppState::new(scenarios, profile, storage, tracker, scheduler);
    /// assert_eq!(state.screen, Screen::MainMenu);
    /// ```
    pub fn new(
        scenarios: Vec<Scenario>,
        profile: Rc<RefCell<UserProfile>>,
        profile_storage: ProfileStorage,
        performance_tracker: Rc<RefCell<PerformanceTracker>>,
        scheduler: Scheduler,
    ) -> Self {
        Self {
            scenarios,
            session: None,
            current_hint: None,
            last_command: None,
            key_history: Vec::new(),
            command_buffer: String::new(),
            completion_time: None,
            selected_menu_item: 0,
            menu_scroll_offset: 0,
            screen: Screen::MainMenu,
            running: true,
            show_hint_panel: false,
            show_key_history: false,
            profile,
            profile_storage,
            performance_tracker,
            scheduler,
            scenarios_completed_today: 0,
            last_save_time: None,
            session_start_time: Instant::now(),
            commands_used_today: HashSet::new(),
            xp_breakdown: None,
            quest_progress_changes: Vec::new(),
            previously_completed_quests: HashSet::new(),
            scenario_mastery: None,
        }
    }

    /// Get reference to the current session
    pub fn session(&self) -> Option<&GameSession> {
        self.session.as_ref()
    }

    /// Get mutable reference to the current session
    pub fn session_mut(&mut self) -> Option<&mut GameSession> {
        self.session.as_mut()
    }

    /// Get the menu items for the main menu
    // TODO: Iteration 4 - Add "View Profile" and "Statistics" menu items
    pub fn menu_items() -> Vec<&'static str> {
        vec!["Start Training", "Quit"]
    }

    /// Get the number of available scenarios
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Get a scenario by index
    pub fn get_scenario(&self, index: usize) -> Option<&Scenario> {
        self.scenarios.get(index)
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

    /// Save profile with debouncing (only if enough time has passed)
    ///
    /// # Errors
    ///
    /// Returns error if save operation fails
    // NOTE: Debounce saves to reduce I/O overhead (5-second delay)
    // OPTIMIZE: Performance audit suggested this optimization (50-80% I/O reduction)
    pub fn save_profile_debounced(&mut self) -> Result<(), crate::gamification::GamificationError> {
        let now = Instant::now();

        // Debounce: only save if 5+ seconds since last save
        if let Some(last_save) = self.last_save_time
            && now.duration_since(last_save) < std::time::Duration::from_secs(5)
        {
            return Ok(());
        }

        let profile = self.profile.borrow();
        self.profile_storage.save(&profile)?;
        self.last_save_time = Some(now);

        Ok(())
    }

    /// Force immediate save (for level-up, achievements, exit)
    ///
    /// # Errors
    ///
    /// Returns error if save operation fails
    pub fn save_profile_immediate(&mut self) -> Result<(), crate::gamification::GamificationError> {
        let profile = self.profile.borrow();
        self.profile_storage.save(&profile)?;
        self.last_save_time = Some(Instant::now());
        Ok(())
    }
}

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings
fn format_key_for_display(command: &str) -> String {
    match command {
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        "Backspace" => "⌫".to_string(),
        "Escape" => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
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
/// assert!(!state.running);
/// # Ok::<(), helix_trainer::security::UserError>(())
/// ```
pub fn update(state: &mut AppState, msg: Message) -> Result<(), UserError> {
    match msg {
        Message::QuitApp => {
            state.running = false;
            Ok(())
        }

        Message::NavigateTo(screen) => {
            state.screen = screen;
            Ok(())
        }

        Message::MenuUp => {
            if state.selected_menu_item > 0 {
                state.selected_menu_item -= 1;
            }
            Ok(())
        }

        Message::MenuDown => {
            // Total menu items = scenarios + Quit option
            let max_items = state.scenarios.len() + 1;
            if state.selected_menu_item < max_items - 1 {
                state.selected_menu_item += 1;
            }
            Ok(())
        }

        Message::MenuSelect => {
            let scenario_count = state.scenarios.len();
            let selected = state.selected_menu_item;

            if selected < scenario_count {
                // Start selected scenario
                update(state, Message::StartScenario(selected))?;
            } else if selected == scenario_count {
                // Quit option (last item)
                update(state, Message::QuitApp)?;
            }
            Ok(())
        }

        Message::StartScenario(index) => {
            if let Some(scenario) = state.scenarios.get(index).cloned() {
                let session = GameSession::new(scenario)?;
                state.session = Some(session);
                state.screen = Screen::Task;
                state.show_hint_panel = false;
                state.show_key_history = false;
                state.current_hint = None;
                state.last_command = None;
                state.completion_time = None;
                state.clear_key_history();
                state.command_buffer.clear();
            }
            Ok(())
        }

        Message::CompleteScenario => {
            // Update quest progress BEFORE awarding XP
            // Extract data we need first to avoid borrow issues
            let (duration, feedback, scenario_id) = if let Some(session) = &state.session {
                let duration = session.elapsed();
                let feedback = session
                    .get_feedback()
                    .map_err(|_| UserError::OperationFailed)?;
                let scenario_id = session.scenario().id.clone();
                (duration, feedback, scenario_id)
            } else {
                state.screen = Screen::Results;
                return Ok(());
            };

            // Update quest progress
            update(
                state,
                Message::UpdateQuestProgress {
                    command: None,
                    scenario_completed: true,
                    duration,
                },
            )?;

            // Calculate base XP (before mastery scaling)
            let score = feedback.score;
            let is_perfect = feedback.score == feedback.max_points;
            let is_first_today = state.scenarios_completed_today == 0;

            // Base XP from score (50 XP per 100 points)
            let base_xp = (score as u64 * 50) / 100;

            // Perfect bonus (+20%)
            let perfect_bonus = if is_perfect { base_xp / 5 } else { 0 };

            // First today bonus (+10 XP)
            let first_today_bonus = if is_first_today { 10 } else { 0 };

            let total_base_xp = base_xp + perfect_bonus + first_today_bonus;

            // Apply mastery scaling and record completion
            let (actual_xp, mastery_level, mastery_multiplier) = {
                let mut profile = state.profile.borrow_mut();
                let actual_xp =
                    profile
                        .scenario_history
                        .record_completion(&scenario_id, score, total_base_xp);

                // Get mastery info for UI display
                let completion = profile.scenario_history.get(&scenario_id).unwrap();
                let mastery_level = completion.mastery_level;
                let mastery_multiplier = completion.xp_multiplier();

                (actual_xp, mastery_level, mastery_multiplier)
            };

            // Store mastery info for results display
            state.scenario_mastery = Some((mastery_level, mastery_multiplier));

            // Quest bonuses (collect newly completed quests)
            let mut quest_bonuses = Vec::new();
            let newly_completed_quest_ids: Vec<String> = {
                let profile = state.profile.borrow();
                profile
                    .daily_quests
                    .iter()
                    .filter(|q| q.completed && !state.previously_completed_quests.contains(&q.id))
                    .map(|q| q.id.clone())
                    .collect()
            };

            // Collect bonuses and mark as processed
            for quest_id in newly_completed_quest_ids {
                let profile = state.profile.borrow();
                if let Some(quest) = profile.daily_quests.iter().find(|q| q.id == quest_id) {
                    let description = format_quest_description(&quest.quest_type);
                    let xp = quest.xp_reward as u64;
                    drop(profile);
                    quest_bonuses.push((description, xp));
                    state.previously_completed_quests.insert(quest_id);
                }
            }

            let quest_xp = quest_bonuses.iter().map(|(_, xp)| xp).sum::<u64>();
            let total_xp = actual_xp + quest_xp;

            // Store breakdown for results display
            state.xp_breakdown = Some(XPBreakdown {
                base_xp,
                perfect_bonus,
                first_today_bonus,
                mastery_multiplier,
                quest_bonuses,
                total_xp,
            });

            // Award XP to profile
            {
                let mut profile = state.profile.borrow_mut();
                let leveled_up = profile.add_xp(total_xp);

                // Update counters
                profile.scenarios_completed += 1;
                if is_perfect {
                    profile.perfect_scenarios += 1;
                }

                if leveled_up {
                    drop(profile);
                    state
                        .save_profile_immediate()
                        .map_err(|_| UserError::OperationFailed)?;
                }
            }

            state.scenarios_completed_today += 1;
            state
                .save_profile_debounced()
                .map_err(|_| UserError::OperationFailed)?;
            state.screen = Screen::Results;
            Ok(())
        }

        Message::AbandonScenario => {
            if let Some(session) = &mut state.session {
                session.abandon();
            }
            state.screen = Screen::Results;
            Ok(())
        }

        Message::ShowHint => {
            // If hint panel is already visible, close it (toggle behavior)
            if state.show_hint_panel {
                state.show_hint_panel = false;
                state.current_hint = None;
                return Ok(());
            }

            // Otherwise, try to show next hint
            if let Some(session) = &mut state.session
                && let Some(hint) = session.get_hint()
            {
                state.current_hint = Some(hint.clone());
                state.show_hint_panel = true;
            }
            Ok(())
        }

        Message::ExecuteCommand(command) => {
            // Add key to history for display (format for readability)
            let display_key = format_key_for_display(command.as_ref());
            state.add_key_to_history(display_key);

            // Show key history popup after first keypress
            state.show_key_history = true;

            // Track command for quest progress (only execute once per complete command)
            let mut executed_command: Option<String> = None;

            if let Some(session) = &mut state.session {
                // In Insert mode, execute commands directly
                if session.is_insert_mode() {
                    // Store last command for display (skip special commands and single chars)
                    if command.as_ref() == "Escape" {
                        state.last_command = Some(command.to_string());
                    }

                    // Execute command through session
                    session.record_action(command.to_string())?;
                } else {
                    // Normal mode: handle command buffer for multi-key commands
                    state.command_buffer.push_str(&command);

                    // Try to match a complete command
                    let final_command = match state.command_buffer.as_str() {
                        // Multi-key commands
                        "dd" => Some("dd"),
                        "gg" => Some("gg"),

                        // Replace character command: r + any char
                        cmd if cmd.starts_with('r') && cmd.len() == 2 => {
                            Some(state.command_buffer.as_str())
                        }

                        // Partial commands - wait for more input
                        "d" | "g" | "r" => None,

                        // Single-key commands (clear buffer and execute)
                        _ if state.command_buffer.len() == 1 => Some(state.command_buffer.as_str()),

                        // Invalid sequence - clear buffer
                        _ => {
                            state.command_buffer.clear();
                            return Ok(());
                        }
                    };

                    if let Some(cmd) = final_command {
                        // We have a complete command
                        let cmd_string = cmd.to_string();
                        state.command_buffer.clear();

                        // Store for display
                        state.last_command = Some(cmd_string.clone());

                        // Track for quest progress
                        executed_command = Some(cmd_string.clone());

                        // Execute command through session
                        session.record_action(cmd_string)?;
                    }
                    // If None, we're waiting for more keys (buffer not cleared)
                }
            }

            // Update quest progress for executed command (after releasing session borrow)
            if let Some(cmd) = executed_command {
                update(
                    state,
                    Message::UpdateQuestProgress {
                        command: Some(cmd),
                        scenario_completed: false,
                        duration: Duration::from_secs(0),
                    },
                )?;
            }

            // Check if scenario is complete
            if let Some(session) = &state.session
                && session.is_completed()
            {
                // Mark completion time instead of immediately going to results
                // This allows showing the success state before transition
                state.completion_time = Some(std::time::Instant::now());
            }

            Ok(())
        }

        Message::RetryScenario => {
            if let Some(session) = &mut state.session {
                session.reset()?;
                state.screen = Screen::Task;
                state.show_hint_panel = false;
                state.show_key_history = false;
                state.current_hint = None;
                state.last_command = None;
                state.completion_time = None;
                state.clear_key_history();
                state.command_buffer.clear();
            }
            Ok(())
        }

        Message::NextScenario => {
            state.screen = Screen::MainMenu;
            state.session = None;
            state.show_hint_panel = false;
            state.current_hint = None;
            Ok(())
        }

        Message::BackToMenu => {
            state.screen = Screen::MainMenu;
            state.session = None;
            state.show_hint_panel = false;
            state.current_hint = None;
            Ok(())
        }

        Message::ShowProfile => {
            state.screen = Screen::Profile;
            Ok(())
        }

        Message::ShowStatistics => {
            state.screen = Screen::Statistics;
            Ok(())
        }

        Message::AwardXP { amount } => {
            let mut profile = state.profile.borrow_mut();
            let leveled_up = profile.add_xp(amount);

            if leveled_up {
                drop(profile); // Release borrow before save
                state
                    .save_profile_immediate()
                    .map_err(|_| UserError::OperationFailed)?;
            }
            Ok(())
        }

        Message::UpdateQuestProgress {
            command,
            scenario_completed,
            duration,
        } => {
            use crate::gamification::QuestTracker;

            // Clear previous progress changes
            state.quest_progress_changes.clear();

            // Snapshot progress BEFORE updates
            let progress_before: HashMap<String, u32> = {
                let profile = state.profile.borrow();
                profile
                    .daily_quests
                    .iter()
                    .map(|q| (q.id.clone(), get_quest_current_progress(&q.quest_type)))
                    .collect()
            };

            // Track which quests were already completed before this update
            let was_completed: Vec<bool> = {
                let profile = state.profile.borrow();
                profile.daily_quests.iter().map(|q| q.completed).collect()
            };

            // Update command practice quests and exploration quests
            if let Some(cmd) = &command {
                // Track for exploration quests
                state.commands_used_today.insert(cmd.clone());

                // Update command progress in quests
                let mut profile = state.profile.borrow_mut();
                QuestTracker::update_command_progress(&mut profile.daily_quests, cmd);
            }

            // Update scenario completion quests and speed run quests
            if scenario_completed {
                let scenario_id = state
                    .session
                    .as_ref()
                    .map(|s| s.scenario().id.clone())
                    .unwrap_or_default();

                let mut profile = state.profile.borrow_mut();
                QuestTracker::update_scenario_progress(
                    &mut profile.daily_quests,
                    &scenario_id,
                    duration,
                );
            }

            // Update time invested quests
            let minutes = duration.as_secs() / 60;
            if minutes > 0 {
                let mut profile = state.profile.borrow_mut();
                QuestTracker::update_time_progress(&mut profile.daily_quests, minutes as u32);
            }

            // Detect progress changes AFTER updates
            {
                let profile = state.profile.borrow();
                for quest in &profile.daily_quests {
                    let old = progress_before.get(&quest.id).copied().unwrap_or(0);
                    let new = get_quest_current_progress(&quest.quest_type);

                    if new > old {
                        state.quest_progress_changes.push(QuestProgressChange {
                            quest_description: format_quest_description(&quest.quest_type),
                            old_progress: old,
                            new_progress: new,
                        });
                    }
                }
            }

            // Check for newly completed quests and award bonus XP
            let newly_completed_xp: Vec<u32> = {
                let profile = state.profile.borrow();
                profile
                    .daily_quests
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, quest)| {
                        if !was_completed[idx] && quest.completed {
                            Some(quest.xp_reward)
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            // Award XP for newly completed quests
            if !newly_completed_xp.is_empty() {
                let total_bonus_xp: u64 = newly_completed_xp.iter().map(|xp| *xp as u64).sum();
                let mut profile = state.profile.borrow_mut();
                profile.add_xp(total_bonus_xp);
            }

            Ok(())
        }
    }
}

/// Format quest type as readable description
fn format_quest_description(quest_type: &QuestType) -> String {
    use QuestType::*;
    match quest_type {
        CommandPractice {
            command, target, ..
        } => format!("Use '{}' {} times", command, target),
        ScenarioCompletion { target, .. } => format!("Complete {} scenarios", target),
        SpeedRun { scenario_id, .. } => format!("Speed run: {}", scenario_id),
        TimeInvested { target_minutes, .. } => format!("Practice {} min", target_minutes),
        Exploration {
            target_commands, ..
        } => format!("Try {} commands", target_commands),
    }
}

/// Get current progress value from quest type
fn get_quest_current_progress(quest_type: &QuestType) -> u32 {
    use QuestType::*;
    match quest_type {
        CommandPractice { current, .. } => *current,
        ScenarioCompletion { current, .. } => *current,
        TimeInvested {
            current_minutes, ..
        } => *current_minutes,
        Exploration { commands_used, .. } => commands_used.len() as u32,
        SpeedRun { .. } => 0, // Single-attempt quest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ScoringConfig, Setup, Solution, TargetState};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::{PerformanceTracker, Scheduler};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn create_test_scenario() -> Scenario {
        Scenario {
            id: "test_001".to_string(),
            name: "Test Scenario".to_string(),
            description: "A test scenario for UI testing".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\nline 3\n".to_string(),
                cursor_position: (0, 0),
            },
            target: TargetState {
                file_content: "line 2\nline 3\n".to_string(),
                cursor_position: (0, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Delete first line".to_string(),
            },
            alternatives: vec![],
            hints: vec!["Use dd to delete a line".to_string()],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
        }
    }

    fn create_test_app_state(scenarios: Vec<Scenario>) -> AppState {
        let profile = Rc::new(RefCell::new(UserProfile::new()));
        let storage = ProfileStorage::new();
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
        let scheduler = Scheduler::new(tracker.clone());
        AppState::new(scenarios, profile, storage, tracker, scheduler)
    }

    #[test]
    fn test_new_state() {
        let state = create_test_app_state(vec![]);
        assert_eq!(state.screen, Screen::MainMenu);
        assert_eq!(state.selected_menu_item, 0);
        assert!(state.running);
        assert!(state.session.is_none());
        assert!(!state.show_hint_panel);
    }

    #[test]
    fn test_quit_app_message() {
        let mut state = create_test_app_state(vec![]);
        assert!(state.running);

        update(&mut state, Message::QuitApp).unwrap();
        assert!(!state.running);
    }

    #[test]
    fn test_navigate_to_screen() {
        let mut state = create_test_app_state(vec![]);
        assert_eq!(state.screen, Screen::MainMenu);

        update(&mut state, Message::NavigateTo(Screen::Task)).unwrap();
        assert_eq!(state.screen, Screen::Task);

        update(&mut state, Message::NavigateTo(Screen::Results)).unwrap();
        assert_eq!(state.screen, Screen::Results);
    }

    #[test]
    fn test_menu_navigation_up() {
        let mut state = create_test_app_state(vec![]);
        state.selected_menu_item = 1;

        update(&mut state, Message::MenuUp).unwrap();
        assert_eq!(state.selected_menu_item, 0);

        // Can't go below 0
        update(&mut state, Message::MenuUp).unwrap();
        assert_eq!(state.selected_menu_item, 0);
    }

    #[test]
    fn test_menu_navigation_down() {
        let scenario1 = create_test_scenario();
        let scenario2 = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario1, scenario2]);
        assert_eq!(state.selected_menu_item, 0);

        // Move down once
        update(&mut state, Message::MenuDown).unwrap();
        assert_eq!(state.selected_menu_item, 1);

        // Move down again
        update(&mut state, Message::MenuDown).unwrap();
        assert_eq!(state.selected_menu_item, 2); // Now on Quit

        // Can't go past max items
        update(&mut state, Message::MenuDown).unwrap();
        assert_eq!(state.selected_menu_item, 2);
    }

    #[test]
    fn test_menu_select_start_training() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);
        state.selected_menu_item = 0;

        update(&mut state, Message::MenuSelect).unwrap();

        assert_eq!(state.screen, Screen::Task);
        assert!(state.session.is_some());
    }

    #[test]
    fn test_menu_select_quit() {
        let scenario1 = create_test_scenario();
        let scenario2 = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario1, scenario2]);
        // Select Quit option (index = scenario count)
        state.selected_menu_item = 2;

        update(&mut state, Message::MenuSelect).unwrap();

        assert!(!state.running);
    }

    #[test]
    fn test_start_scenario() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();

        assert!(state.session.is_some());
        assert_eq!(state.screen, Screen::Task);
    }

    #[test]
    fn test_start_invalid_scenario_index() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(999)).unwrap();

        // Should still have None session
        assert!(state.session.is_none());
    }

    #[test]
    fn test_complete_scenario_navigates_to_results() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert_eq!(state.screen, Screen::Task);

        update(&mut state, Message::CompleteScenario).unwrap();
        assert_eq!(state.screen, Screen::Results);
    }

    #[test]
    fn test_abandon_scenario_navigates_to_results() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        let session = state.session.as_ref().unwrap();
        assert!(session.is_active());

        update(&mut state, Message::AbandonScenario).unwrap();
        assert_eq!(state.screen, Screen::Results);
        let session = state.session.as_ref().unwrap();
        assert!(!session.is_active());
    }

    #[test]
    fn test_show_hint() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert!(!state.show_hint_panel);

        update(&mut state, Message::ShowHint).unwrap();
        assert!(state.show_hint_panel);
        assert!(state.current_hint.is_some());
    }

    #[test]
    fn test_retry_scenario_resets_state() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        if let Some(session) = &mut state.session {
            session.record_action("l".to_string()).unwrap();
        }
        assert_eq!(state.session.as_ref().unwrap().action_count(), 1);

        update(&mut state, Message::RetryScenario).unwrap();
        assert_eq!(state.screen, Screen::Task);
        assert_eq!(state.session.as_ref().unwrap().action_count(), 0);
    }

    #[test]
    fn test_next_scenario_clears_session() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert!(state.session.is_some());

        update(&mut state, Message::NextScenario).unwrap();
        assert_eq!(state.screen, Screen::MainMenu);
        assert!(state.session.is_none());
    }

    #[test]
    fn test_back_to_menu_clears_session() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        update(&mut state, Message::StartScenario(0)).unwrap();
        assert!(state.session.is_some());

        update(&mut state, Message::BackToMenu).unwrap();
        assert_eq!(state.screen, Screen::MainMenu);
        assert!(state.session.is_none());
    }

    #[test]
    fn test_menu_items() {
        let items = AppState::menu_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "Start Training");
        assert_eq!(items[1], "Quit");
    }

    #[test]
    fn test_scenario_count() {
        let scenarios = vec![create_test_scenario(), create_test_scenario()];
        let state = create_test_app_state(scenarios);
        assert_eq!(state.scenario_count(), 2);
    }

    #[test]
    fn test_get_scenario() {
        let scenario = create_test_scenario();
        let mut scenarios = vec![scenario.clone()];
        scenarios.push(scenario);
        let state = create_test_app_state(scenarios);

        assert!(state.get_scenario(0).is_some());
        assert!(state.get_scenario(1).is_some());
        assert!(state.get_scenario(999).is_none());
    }

    // Quest tracking tests
    #[test]
    fn test_quest_progress_command_practice() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add a CommandPractice quest to profile
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_dd".to_string(),
                QuestType::CommandPractice {
                    command: "dd".to_string(),
                    target: 3,
                    current: 0,
                },
                "Delete 3 lines".to_string(),
                QuestDifficulty::Easy,
            ));
        }

        // Execute "dd" command twice
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Quest should not be completed yet (2/3)
        {
            let profile = state.profile.borrow();
            assert!(!profile.daily_quests[0].is_completed());
        }

        // Execute once more
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Quest should now be completed and bonus XP awarded
        {
            let profile = state.profile.borrow();
            assert!(profile.daily_quests[0].is_completed());
            // XP should be at least the quest reward
            assert!(profile.total_xp >= 25); // Easy CommandPractice = 25 XP
        }
    }

    #[test]
    fn test_quest_progress_scenario_completion() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add a ScenarioCompletion quest to profile
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_scenario".to_string(),
                QuestType::ScenarioCompletion {
                    target: 2,
                    current: 0,
                },
                "Complete 2 scenarios".to_string(),
                QuestDifficulty::Medium,
            ));
        }

        // Start and "complete" a scenario
        update(&mut state, Message::StartScenario(0)).unwrap();

        // Simulate scenario completion
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: None,
                scenario_completed: true,
                duration: Duration::from_secs(5),
            },
        )
        .unwrap();

        // Quest should not be completed yet (1/2)
        {
            let profile = state.profile.borrow();
            assert!(!profile.daily_quests[0].is_completed());
        }

        // Complete another scenario
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: None,
                scenario_completed: true,
                duration: Duration::from_secs(5),
            },
        )
        .unwrap();

        // Quest should now be completed
        {
            let profile = state.profile.borrow();
            assert!(profile.daily_quests[0].is_completed());
            // XP should include quest reward
            assert!(profile.total_xp >= 75); // Medium ScenarioCompletion = 75 XP
        }
    }

    #[test]
    fn test_quest_completion_awards_bonus_xp() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        let initial_xp = {
            let profile = state.profile.borrow();
            profile.total_xp
        };

        // Add a quest
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_quest".to_string(),
                QuestType::CommandPractice {
                    command: "x".to_string(),
                    target: 1,
                    current: 0,
                },
                "Delete 1 character".to_string(),
                QuestDifficulty::Easy,
            ));
        }

        // Complete the quest
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("x".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Check that XP was awarded
        {
            let profile = state.profile.borrow();
            assert_eq!(profile.total_xp, initial_xp + 25); // Easy CommandPractice = 25 XP
            assert!(profile.daily_quests[0].is_completed());
        }
    }

    #[test]
    fn test_exploration_quest_tracking() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};
        use std::collections::HashSet;

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        let initial_xp = {
            let profile = state.profile.borrow();
            profile.total_xp
        };

        // Add an Exploration quest
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_exploration".to_string(),
                QuestType::Exploration {
                    target_commands: 3,
                    commands_used: HashSet::new(),
                },
                "Use 3 different commands".to_string(),
                QuestDifficulty::Hard,
            ));
        }

        // Execute different commands
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("yy".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Not completed yet (2/3)
        {
            let profile = state.profile.borrow();
            assert!(!profile.daily_quests[0].is_completed());
        }

        // Execute third unique command
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("p".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Should be completed now and bonus XP awarded
        {
            let profile = state.profile.borrow();
            assert!(profile.daily_quests[0].is_completed());
            assert_eq!(profile.total_xp, initial_xp + 160); // Hard Exploration = 160 XP
        }
    }

    #[test]
    fn test_commands_used_today_tracking() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        assert_eq!(state.commands_used_today.len(), 0);

        // Execute some commands
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("yy".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Same command again (should not duplicate)
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Should have 2 unique commands
        assert_eq!(state.commands_used_today.len(), 2);
        assert!(state.commands_used_today.contains("dd"));
        assert!(state.commands_used_today.contains("yy"));
    }

    // XP Breakdown tests
    #[test]
    fn test_xp_breakdown_base_only() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Complete a scenario with non-perfect score (not first today to avoid bonus)
        state.scenarios_completed_today = 1; // Not first today
        update(&mut state, Message::StartScenario(0)).unwrap();

        // Execute non-optimal solution to get points but not perfect
        if let Some(session) = &mut state.session {
            session.record_action("l".to_string()).unwrap(); // Extra move
            session.record_action("h".to_string()).unwrap(); // Extra move
            session.record_action("dd".to_string()).unwrap(); // Correct solution
        }

        update(&mut state, Message::CompleteScenario).unwrap();

        // Check XP breakdown
        assert!(state.xp_breakdown.is_some());
        let xp = state.xp_breakdown.as_ref().unwrap();

        // Should have base XP (score > 0 because scenario is completed)
        // Perfect bonus should be 0 because we used extra moves
        assert!(xp.base_xp > 0, "Base XP should be > 0, got {}", xp.base_xp);
        assert_eq!(xp.perfect_bonus, 0);
        assert_eq!(xp.first_today_bonus, 0);
        assert_eq!(xp.quest_bonuses.len(), 0);
        assert_eq!(xp.total_xp, xp.base_xp);
    }

    #[test]
    fn test_xp_breakdown_with_perfect_bonus() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.scenarios_completed_today = 1; // Not first today
        update(&mut state, Message::StartScenario(0)).unwrap();

        // Execute perfect solution
        if let Some(session) = &mut state.session {
            session.record_action("dd".to_string()).unwrap();
        }

        update(&mut state, Message::CompleteScenario).unwrap();

        assert!(state.xp_breakdown.is_some());
        let xp = state.xp_breakdown.as_ref().unwrap();

        // Should have perfect bonus (20% of base)
        assert!(xp.perfect_bonus > 0);
        assert_eq!(xp.perfect_bonus, xp.base_xp / 5);
        assert_eq!(xp.first_today_bonus, 0);
    }

    #[test]
    fn test_xp_breakdown_with_first_today_bonus() {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // First scenario today
        assert_eq!(state.scenarios_completed_today, 0);

        update(&mut state, Message::StartScenario(0)).unwrap();

        if let Some(session) = &mut state.session {
            session.record_action("dd".to_string()).unwrap();
        }

        update(&mut state, Message::CompleteScenario).unwrap();

        assert!(state.xp_breakdown.is_some());
        let xp = state.xp_breakdown.as_ref().unwrap();

        // Should have first today bonus
        assert_eq!(xp.first_today_bonus, 10);
    }

    #[test]
    fn test_xp_breakdown_with_quest_completion() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add a quest that will be completed
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_quest".to_string(),
                QuestType::CommandPractice {
                    command: "dd".to_string(),
                    target: 1,
                    current: 0,
                },
                "Delete 1 line".to_string(),
                QuestDifficulty::Easy,
            ));
        }

        update(&mut state, Message::StartScenario(0)).unwrap();

        // Execute command to complete quest through message
        update(
            &mut state,
            Message::ExecuteCommand(std::borrow::Cow::Borrowed("dd")),
        )
        .unwrap();

        update(&mut state, Message::CompleteScenario).unwrap();

        assert!(state.xp_breakdown.is_some());
        let xp = state.xp_breakdown.as_ref().unwrap();

        // Should have quest bonus
        assert_eq!(xp.quest_bonuses.len(), 1);
        let (desc, bonus) = &xp.quest_bonuses[0];
        assert!(desc.contains("dd"));
        assert_eq!(*bonus, 25); // Easy quest = 25 XP
    }

    #[test]
    fn test_quest_progress_changes_tracking() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add a quest
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_quest".to_string(),
                QuestType::CommandPractice {
                    command: "dd".to_string(),
                    target: 3,
                    current: 0,
                },
                "Delete 3 lines".to_string(),
                QuestDifficulty::Easy,
            ));
        }

        // Execute command to trigger progress
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some("dd".to_string()),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Check that progress changes were recorded
        assert_eq!(state.quest_progress_changes.len(), 1);
        let change = &state.quest_progress_changes[0];
        assert_eq!(change.old_progress, 0);
        assert_eq!(change.new_progress, 1);
        assert!(change.quest_description.contains("dd"));
    }

    #[test]
    fn test_previously_completed_quests_tracking() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        // Add a quest that will be completed on first scenario
        {
            let mut profile = state.profile.borrow_mut();
            profile.daily_quests.push(Quest::new(
                "test_quest".to_string(),
                QuestType::CommandPractice {
                    command: "dd".to_string(),
                    target: 1,
                    current: 0,
                },
                "Delete 1 line".to_string(),
                QuestDifficulty::Easy,
            ));
        }

        // First scenario completion
        update(&mut state, Message::StartScenario(0)).unwrap();

        // Execute command through message to trigger quest progress tracking
        update(
            &mut state,
            Message::ExecuteCommand(std::borrow::Cow::Borrowed("dd")),
        )
        .unwrap();

        update(&mut state, Message::CompleteScenario).unwrap();

        // Debug: Check if quest is actually completed
        {
            let profile = state.profile.borrow();
            let quest = &profile.daily_quests[0];
            assert!(
                quest.is_completed(),
                "Quest should be completed after scenario"
            );
        }

        // Quest should be marked as previously completed after first scenario
        assert!(
            !state.previously_completed_quests.is_empty(),
            "previously_completed_quests should not be empty, found {} items",
            state.previously_completed_quests.len()
        );
        assert!(
            state.previously_completed_quests.contains("test_quest"),
            "Quest 'test_quest' should be marked as previously completed. Found: {:?}",
            state.previously_completed_quests
        );

        // First breakdown should have quest bonus
        let first_xp = state.xp_breakdown.as_ref().unwrap();
        assert_eq!(
            first_xp.quest_bonuses.len(),
            1,
            "First completion should award quest bonus"
        );

        // Second scenario - quest already completed, should not award bonus again
        state.scenarios.push(scenario); // Add another scenario
        update(&mut state, Message::StartScenario(1)).unwrap();

        // Execute command through message
        update(
            &mut state,
            Message::ExecuteCommand(std::borrow::Cow::Borrowed("dd")),
        )
        .unwrap();

        update(&mut state, Message::CompleteScenario).unwrap();

        let second_xp = state.xp_breakdown.as_ref().unwrap();
        assert_eq!(
            second_xp.quest_bonuses.len(),
            0,
            "Second completion should not award quest bonus again"
        );
    }
}
