//! Type-safe screen variants with required data
//!
//! This module implements the `TypedScreen` pattern, where each screen variant
//! carries the data required for that screen. This makes invalid states
//! unrepresentable at compile time - you can't be on the Task screen without
//! an active `GameSession`.

use crate::config::Scenario;
use crate::config::ScenarioCategory;
use crate::game::{
    Abandoned, Active, Completed, Feedback, GameSession, PlayableScenario, SessionAfterAction,
};
use crate::ids::ScenarioId;
use crate::input::typestate::InputStateMachine;
use crate::learning::{ReviewSession, ScenarioMastery};
use crate::minigame::MiniGameSession;
use crate::ui::state::{QuestProgressChange, XPBreakdown};
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of keys to keep in history
const KEY_HISTORY_CAPACITY: usize = 5;

/// Shared key history for tracking recent keypresses
///
/// Used by both training mode (`TaskData`) and arcade mode (`MiniGameData`)
/// to display recent key presses in the UI. Keeps the last 5 keys
/// with most recent first.
#[derive(Debug, Clone, Default)]
pub struct KeyHistory {
    /// History of keypresses (most recent first)
    keys: VecDeque<String>,
}

impl KeyHistory {
    /// Create a new empty key history
    pub fn new() -> Self {
        Self {
            keys: VecDeque::with_capacity(KEY_HISTORY_CAPACITY),
        }
    }

    /// Add a key to the history (keeps last 5, most recent first)
    pub fn push(&mut self, key: String) {
        self.keys.push_front(key);
        if self.keys.len() > KEY_HISTORY_CAPACITY {
            self.keys.truncate(KEY_HISTORY_CAPACITY);
        }
    }

    /// Clear all key history
    pub fn clear(&mut self) {
        self.keys.clear();
    }

    /// Get the keys, most recent first
    pub fn keys(&self) -> &VecDeque<String> {
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

    /// The arcade run this screen owns: the mini-game screen itself, or the detour screen
    /// (profile, statistics, achievements, category filters) opened from a paused run.
    pub fn minigame_data(&self) -> Option<&MiniGameData> {
        match self {
            Self::MiniGame(data) => Some(data),
            Self::Profile(ProfileData { return_to })
            | Self::Statistics(StatisticsData { return_to })
            | Self::Achievements(AchievementsData { return_to, .. })
            | Self::CategoryFilters(CategoryFiltersData { return_to, .. }) => match return_to {
                ReturnDestination::PausedMiniGame(data) => Some(data),
                ReturnDestination::Menu => None,
            },
            _ => None,
        }
    }

    /// The finished task while the task screen shows its success animation
    pub fn completed_task(&self) -> Option<&CompletedTask> {
        match self {
            Self::Task(data) => data.session.completed(),
            _ => None,
        }
    }

    /// Return destination for a detour screen opened from the current one.
    ///
    /// A paused arcade run is detached from the mini-game screen, or handed on from another
    /// detour screen, so the new detour screen owns it until the user returns; from any other
    /// screen the destination is the menu. The caller must replace `self` with the detour screen
    /// right after.
    pub fn detour_return_destination(&mut self) -> ReturnDestination {
        if let Some(destination) = self.take_return_destination() {
            return destination;
        }
        match std::mem::replace(self, Self::Menu(MenuData::default())) {
            Self::MiniGame(data) if data.session.state().is_paused() => {
                ReturnDestination::PausedMiniGame(Box::new(data))
            }
            other => {
                *self = other;
                ReturnDestination::Menu
            }
        }
    }

    /// Take the return destination out of a detour screen, leaving the menu in its place.
    ///
    /// `None` when the current screen is not a detour screen.
    pub fn take_return_destination(&mut self) -> Option<ReturnDestination> {
        match self {
            Self::Profile(ProfileData { return_to })
            | Self::Statistics(StatisticsData { return_to })
            | Self::Achievements(AchievementsData { return_to, .. })
            | Self::CategoryFilters(CategoryFiltersData { return_to, .. }) => {
                Some(std::mem::take(return_to))
            }
            _ => None,
        }
    }

    /// Mutable counterpart of [`TypedScreen::minigame_data`].
    pub fn minigame_data_mut(&mut self) -> Option<&mut MiniGameData> {
        match self {
            Self::MiniGame(data) => Some(data),
            Self::Profile(ProfileData { return_to })
            | Self::Statistics(StatisticsData { return_to })
            | Self::Achievements(AchievementsData { return_to, .. })
            | Self::CategoryFilters(CategoryFiltersData { return_to, .. }) => match return_to {
                ReturnDestination::PausedMiniGame(data) => Some(data),
                ReturnDestination::Menu => None,
            },
            _ => None,
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

    /// Multi-key navigation state (count prefix, `g` leader), the same machine gameplay uses
    pub input_state: InputStateMachine,
}

/// Whether the hint panel is open, and the hint it shows
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HintPanel {
    /// No hint panel
    #[default]
    Hidden,
    /// Panel open with this hint text
    Shown(String),
}

impl HintPanel {
    /// The hint text while the panel is open
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Hidden => None,
            Self::Shown(text) => Some(text),
        }
    }

    /// Whether the panel is open
    pub fn is_shown(&self) -> bool {
        matches!(self, Self::Shown(_))
    }
}

/// A finished task kept on the task screen during the success animation
///
/// Owns the completed session together with the feedback derived from it and the instant it
/// finished, so none of the three can exist without the others.
#[derive(Debug)]
pub struct CompletedTask {
    /// The finished session, rendered as the final editor state
    pub session: GameSession<Completed>,

    /// Feedback derived once from the completed session
    pub feedback: Feedback,

    /// When the scenario was completed; the success animation runs from here
    pub completed_at: Instant,
}

impl CompletedTask {
    /// Derive the feedback of a completed session and stamp the completion time `now`
    pub fn new(session: GameSession<Completed>, now: Instant) -> Self {
        let feedback = session.feedback();
        Self {
            session,
            feedback,
            completed_at: now,
        }
    }
}

/// Session of the task screen: being played, or finished and showing the success animation
///
/// A completed task cannot exist without its session and feedback, and an active task carries
/// no completion data.
#[derive(Debug)]
pub enum TaskSession {
    /// The scenario is being played
    Active(GameSession<Active>),

    /// The scenario was completed and the screen waits for the success animation to end
    Completed(CompletedTask),
}

impl TaskSession {
    /// The session while it is being played, `None` once completed
    pub fn active(&self) -> Option<&GameSession<Active>> {
        match self {
            Self::Active(session) => Some(session),
            Self::Completed(_) => None,
        }
    }

    /// Mutable counterpart of [`TaskSession::active`]
    pub fn active_mut(&mut self) -> Option<&mut GameSession<Active>> {
        match self {
            Self::Active(session) => Some(session),
            Self::Completed(_) => None,
        }
    }

    /// The completed task, `None` while the scenario is being played
    pub fn completed(&self) -> Option<&CompletedTask> {
        match self {
            Self::Active(_) => None,
            Self::Completed(task) => Some(task),
        }
    }

    /// Read-only view of the editor state, valid in either state
    pub fn playable(&self) -> &dyn PlayableScenario {
        match self {
            Self::Active(session) => session,
            Self::Completed(task) => &task.session,
        }
    }

    /// The scenario being played, in either state
    pub fn scenario(&self) -> &Scenario {
        match self {
            Self::Active(session) => session.scenario(),
            Self::Completed(task) => task.session.scenario(),
        }
    }

    /// Resolve the state after an action was applied: an active session whose editor now matches
    /// the target becomes a completed task stamped with `now`, everything else is unchanged
    #[must_use]
    pub fn finish_action(self, now: Instant) -> Self {
        match self {
            Self::Active(session) => match session.finish_action() {
                SessionAfterAction::StillActive(session) => Self::Active(session),
                SessionAfterAction::Completed(session) => {
                    Self::Completed(CompletedTask::new(session, now))
                }
            },
            completed @ Self::Completed(_) => completed,
        }
    }
}

/// Data required for task screen
#[derive(Debug)]
pub struct TaskData {
    /// Session of the task: active while playing, completed during the success animation
    pub session: TaskSession,

    /// Typestate-based input state machine for multi-key commands
    ///
    /// Replaces the old `command_buffer: String` with a compile-time safe
    /// state machine that tracks pending input states (goto, find, count, etc.)
    pub input_state: InputStateMachine,

    /// History of recent keypresses (shared `KeyHistory` struct)
    pub key_history: KeyHistory,

    /// Hint panel state
    pub hint: HintPanel,

    /// Last command executed (for display)
    pub last_command: Option<String>,

    /// Index of current scenario in filtered list (for next/prev navigation)
    /// None if scenario was started from outside the normal list flow
    pub scenario_index: Option<usize>,
}

impl TaskData {
    /// Create new `TaskData` with an active session
    ///
    /// # Arguments
    /// * `session` - Active game session
    pub fn new(session: GameSession<Active>) -> Self {
        Self {
            session: TaskSession::Active(session),
            input_state: InputStateMachine::new(),
            key_history: KeyHistory::new(),
            hint: HintPanel::Hidden,
            last_command: None,
            scenario_index: None,
        }
    }

    /// Create new `TaskData` with an active session and scenario index
    ///
    /// # Arguments
    /// * `session` - Active game session
    /// * `index` - Index of scenario in filtered list
    pub fn with_index(session: GameSession<Active>, index: usize) -> Self {
        Self {
            session: TaskSession::Active(session),
            input_state: InputStateMachine::new(),
            key_history: KeyHistory::new(),
            hint: HintPanel::Hidden,
            last_command: None,
            scenario_index: Some(index),
        }
    }

    /// Resolve the session state after an action: see [`TaskSession::finish_action`]
    #[must_use]
    pub fn finish_action(self, now: Instant) -> Self {
        Self {
            session: self.session.finish_action(now),
            ..self
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
    pub scenario_mastery: Option<MasteryInfo>,

    /// Index of current scenario in filtered list (for next/prev navigation)
    /// None if scenario was started from outside the normal list flow
    pub scenario_index: Option<usize>,
}

impl ResultsData {
    /// Create new `ResultsData` from a completed session
    ///
    /// # Arguments
    /// * `session` - Completed game session
    /// * `feedback` - Performance feedback
    /// * `scenario_index` - Index of scenario in filtered list (for navigation)
    pub fn from_completed(
        session: GameSession<Completed>,
        feedback: Feedback,
        scenario_index: Option<usize>,
    ) -> Self {
        Self {
            session: CompletedOrAbandoned::Completed(session),
            feedback,
            xp_breakdown: None,
            quest_changes: Vec::new(),
            scenario_mastery: None,
            scenario_index,
        }
    }

    /// Create new `ResultsData` from an abandoned session
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
    pub fn scenario_id(&self) -> &ScenarioId {
        match self {
            Self::Completed(session) => &session.scenario().id,
            Self::Abandoned(session) => &session.scenario().id,
        }
    }
}

/// Return destination when navigating back from profile/statistics screens
#[derive(Debug, Default)]
pub enum ReturnDestination {
    /// Return to main menu (default)
    #[default]
    Menu,
    /// Return to the paused mini-game, which stays owned here while the detour screen is shown
    PausedMiniGame(Box<MiniGameData>),
}

impl ReturnDestination {
    /// Whether this detour was entered from a paused mini-game
    pub fn is_paused_minigame(&self) -> bool {
        matches!(self, Self::PausedMiniGame(_))
    }
}

/// Data required for profile screen
#[derive(Debug, Default)]
pub struct ProfileData {
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,
}

/// Data required for statistics screen
#[derive(Debug, Default)]
pub struct StatisticsData {
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,
}

/// Data required for achievements screen
#[derive(Debug, Default)]
pub struct AchievementsData {
    /// Where to return when pressing Esc/back
    pub return_to: ReturnDestination,

    /// Scroll offset for the achievement list (top visible row index).
    /// Clamped to the valid range on render, the same way `MenuData::scroll_offset` is.
    pub scroll_offset: usize,
}

/// Data required for category filters screen
#[derive(Debug, Default)]
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
    pub session: ReviewSession,
}

impl ReviewData {
    /// Create new `ReviewData` with a review session
    pub fn new(session: ReviewSession) -> Self {
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
    /// Per-category progress, sorted by category enum order (matches the filter screen and menu).
    pub category_breakdown: Vec<CategoryProgress>,
    /// Suggested next actions, in display order.
    pub next_steps: Vec<NextStep>,
}

impl EndGameSummaryData {
    /// Scenarios not yet perfected. Reaches 0 at full mastery.
    pub fn imperfect(&self) -> usize {
        self.scenarios_total.saturating_sub(self.perfected)
    }
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

/// Top-level mode choice on the mode selection screen
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TopMode {
    /// Manual scenario selection
    #[default]
    Training,
    /// Arcade mini-games
    Arcade,
}

impl TopMode {
    /// The mode above this one, or itself at the top
    #[must_use]
    pub fn previous(self) -> Self {
        Self::Training
    }

    /// The mode below this one, or itself at the bottom
    #[must_use]
    pub fn next(self) -> Self {
        Self::Arcade
    }
}

/// Data required for mode selection screen
#[derive(Debug, Clone, Default)]
pub struct ModeSelectionData {
    /// Index of selected mode (0 = Training, 1 = Arcade)
    pub selected_mode: TopMode,

    /// Mini-game mode selection state (when arcade is chosen)
    pub minigame_mode_selection: Option<MiniGameModeSelection>,
}

/// A mini-game mode offered in the mode submenu, in menu order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MiniGameModeKind {
    /// Timed arcade run
    #[default]
    Arcade,
    /// Lives-based survival run
    Survival,
    /// Daily challenge
    Challenge,
}

impl MiniGameModeKind {
    /// Every mode, in menu order
    pub const ALL: [Self; 3] = [Self::Arcade, Self::Survival, Self::Challenge];

    /// The mode above this one, wrapping from the first to the last
    #[must_use]
    pub fn previous(self) -> Self {
        match self {
            Self::Arcade => Self::Challenge,
            Self::Survival => Self::Arcade,
            Self::Challenge => Self::Survival,
        }
    }

    /// The mode below this one, wrapping from the last to the first
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Arcade => Self::Survival,
            Self::Survival => Self::Challenge,
            Self::Challenge => Self::Arcade,
        }
    }

    /// The default configuration of this mode; the challenge is seeded from `today`
    pub fn mode(self, today: chrono::NaiveDate) -> crate::minigame::MiniGameMode {
        use crate::minigame::{ArcadeConfig, ChallengeConfig, MiniGameMode, SurvivalConfig};

        match self {
            Self::Arcade => MiniGameMode::Arcade(ArcadeConfig::default()),
            Self::Survival => MiniGameMode::Survival(SurvivalConfig::default()),
            Self::Challenge => MiniGameMode::Challenge(ChallengeConfig::for_date(today)),
        }
    }
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
    /// Currently highlighted mode
    pub selected: MiniGameModeKind,
}

impl MiniGameModeSelection {
    /// Available mode count
    pub const MODE_COUNT: usize = MiniGameModeKind::ALL.len();

    /// Create a new mode selection starting at Arcade
    pub fn new() -> Self {
        Self::default()
    }

    /// Move selection up (wraps around)
    pub fn select_previous(&mut self) {
        self.selected = self.selected.previous();
    }

    /// Move selection down (wraps around)
    pub fn select_next(&mut self) {
        self.selected = self.selected.next();
    }

    /// All selectable modes, in menu order.
    ///
    /// Single source of truth for the mode list, shared by `selected_mode`
    /// and the mode-selection submenu renderer so they cannot drift out of
    /// sync. Name/description text comes from `MiniGameMode`'s own exhaustive
    /// match.
    pub(crate) fn all_modes(
        today: chrono::NaiveDate,
    ) -> [crate::minigame::MiniGameMode; Self::MODE_COUNT] {
        MiniGameModeKind::ALL.map(|kind| kind.mode(today))
    }

    /// Get the selected mode configuration
    pub fn selected_mode(&self, today: chrono::NaiveDate) -> crate::minigame::MiniGameMode {
        self.selected.mode(today)
    }
}

/// Mastery level reached for a scenario and the XP multiplier it applied
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasteryInfo {
    /// Mastery level of the completed scenario
    pub mastery: ScenarioMastery,
    /// Combined XP scaling applied to the completion
    pub xp_multiplier: f64,
}

/// Perfected scenarios within one category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CategoryProgress {
    /// The category
    pub category: ScenarioCategory,
    /// Scenarios in the category completed perfectly
    pub perfected: usize,
    /// All scenarios in the category
    pub total: usize,
}

/// Data required for mini-game screen
///
/// Owns the arcade session, so the mini-game screen cannot exist without one.
pub struct MiniGameData {
    /// The running arcade session (boxed to keep `TypedScreen` variants small)
    pub session: Box<MiniGameSession>,
    /// Typestate-based input state machine for multi-key commands
    ///
    /// Replaces the old `command_buffer: String` with a compile-time safe
    /// state machine that tracks pending input states (goto, find, count, etc.)
    pub input_state: InputStateMachine,
    /// Last XP earned (for popup display during transition)
    pub last_xp_earned: Option<u64>,
    /// History of recent keypresses (shared `KeyHistory` struct)
    pub key_history: KeyHistory,
}

impl std::fmt::Debug for MiniGameData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniGameData")
            .field("input_state", &self.input_state)
            .field("last_xp_earned", &self.last_xp_earned)
            .finish_non_exhaustive()
    }
}

impl MiniGameData {
    /// Wrap a freshly created session with empty input and key history
    pub fn new(session: MiniGameSession) -> Self {
        Self {
            session: Box::new(session),
            input_state: InputStateMachine::new(),
            last_xp_earned: None,
            key_history: KeyHistory::new(),
        }
    }

    /// Add a key to the history (keeps last 5)
    pub fn add_key_to_history(&mut self, key: String) {
        self.key_history.push(key);
    }

    /// Clear key history (called when starting new scenario)
    pub fn clear_key_history(&mut self) {
        self.key_history.clear();
    }
}

/// Trait for types that use typestate-based input state machine
///
/// `TaskData` and `MiniGameData` use `InputStateMachine` for handling multi-key
/// Helix commands with compile-time safety. This trait provides uniform
/// access to the input state machine.
///
/// # Examples
///
/// ```
/// use helix_trainer::input::typestate::InputStateMachine;
/// use helix_trainer::ui::state::InputStateAccess;
///
/// #[derive(Default)]
/// struct Panel {
///     input: InputStateMachine,
/// }
///
/// impl InputStateAccess for Panel {
///     fn input_state(&self) -> &InputStateMachine {
///         &self.input
///     }
///
///     fn input_state_mut(&mut self) -> &mut InputStateMachine {
///         &mut self.input
///     }
/// }
///
/// assert!(!Panel::default().is_waiting_for_char());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::sid;
    use std::assert_matches;

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
    fn test_task_data_key_history() {
        use crate::config::{
            Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
        };

        let scenario = Scenario {
            id: sid("test"),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                language: None,
                cursor: crate::testing::point(0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor: crate::testing::point(0, 0),
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Test".to_string(),
            },
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: ScenarioMetadata::default(),
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
        use crate::config::{
            Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
        };

        let scenario = Scenario {
            id: sid("test"),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                language: None,
                cursor: crate::testing::point(0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor: crate::testing::point(0, 0),
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Test".to_string(),
            },
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: ScenarioMetadata::default(),
        };

        let session = GameSession::new(scenario).unwrap();
        let mut task_data = TaskData::new(session);

        for i in 0..7 {
            task_data.add_key_to_history(format!("key{i}"));
        }

        // Should only keep last 5
        assert_eq!(task_data.key_history.len(), 5);
        assert_eq!(task_data.key_history.keys()[0], "key6"); // Most recent
        assert_eq!(task_data.key_history.keys()[4], "key2");
    }

    #[test]
    fn test_completed_or_abandoned_helpers() {
        use crate::config::{
            Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
        };

        let scenario = Scenario {
            id: sid("test_123"),
            name: "Test".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "test".to_string(),
                language: None,
                cursor: crate::testing::point(0, 0),
            },
            target: TargetState {
                file_content: "test".to_string(),
                cursor: crate::testing::point(0, 0),
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Test".to_string(),
            },
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: ScenarioMetadata::default(),
        };

        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();

        let coa = CompletedOrAbandoned::Abandoned(abandoned);
        assert!(coa.is_abandoned());
        assert!(!coa.is_completed());
        assert_eq!(coa.scenario_id().as_str(), "test_123");
    }

    #[test]
    fn test_mode_selection_cycles_through_every_mode_and_wraps() {
        let mut selection = MiniGameModeSelection::new();
        let mut seen = vec![selection.selected];
        for _ in 1..MiniGameModeSelection::MODE_COUNT {
            selection.select_next();
            seen.push(selection.selected);
        }
        assert_eq!(seen, MiniGameModeKind::ALL);

        selection.select_next();
        assert_eq!(selection.selected, MiniGameModeKind::Arcade);
        selection.select_previous();
        assert_eq!(selection.selected, MiniGameModeKind::Challenge);
    }

    #[test]
    fn test_minigame_mode_selection_valid_modes() {
        let today = chrono::Utc::now().date_naive();
        let mut selection = MiniGameModeSelection::new();
        assert!(selection.selected_mode(today).is_arcade());

        selection.selected = MiniGameModeKind::Survival;
        assert!(selection.selected_mode(today).is_survival());

        selection.selected = MiniGameModeKind::Challenge;
        assert!(selection.selected_mode(today).is_challenge());
    }

    /// `all_modes` must stay in lockstep with `MiniGameModeKind::ALL`.
    #[test]
    fn test_all_modes_follow_the_kind_order() {
        let today = chrono::Utc::now().date_naive();
        let modes = MiniGameModeSelection::all_modes(today);

        for (kind, mode) in MiniGameModeKind::ALL.into_iter().zip(&modes) {
            assert_eq!(
                std::mem::discriminant(&kind.mode(today)),
                std::mem::discriminant(mode)
            );
        }
    }

    /// Structural backstop for #328: every currently-defined `MiniGameMode`
    /// variant must appear exactly once in `all_modes`.
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
        assert_matches!(data.return_to, ReturnDestination::Menu);
    }

    #[test]
    fn test_category_filters_data_with_values() {
        let data = CategoryFiltersData {
            selected_index: 5,
            return_to: crate::testing::paused_minigame_return(),
        };
        assert_eq!(data.selected_index, 5);
        assert!(data.return_to.is_paused_minigame());
    }

    #[test]
    fn test_typed_screen_end_game_summary() {
        let data = EndGameSummaryData {
            scenarios_total: 1,
            perfected: 1,
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
    }

    #[test]
    fn test_typed_screen_category_filters() {
        let screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());
        assert_eq!(screen.screen_type(), "CategoryFilters");
    }
}
