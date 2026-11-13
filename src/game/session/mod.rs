//! Game session management for training scenarios
//!
//! This module provides the `GameSession` type which manages the user's attempt
//! at solving a scenario. It tracks user actions, maintains game state, and
//! provides feedback based on performance.
//!
//! # Examples
//!
//! ```
//! use helix_trainer::config::ScenarioLoader;
//! use helix_trainer::game::{GameSession, SessionAfterAction};
//!
//! let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
//! let commands = scenario.solution.commands.clone();
//! let mut session = GameSession::new(scenario)?;
//!
//! // Each action consumes the session and returns its next state
//! for command in commands {
//!     match session.record_action(command)? {
//!         SessionAfterAction::StillActive(next) => session = next,
//!         SessionAfterAction::Completed(done) => {
//!             let feedback = done.feedback()?;
//!             assert!(feedback.success());
//!             println!("{}", feedback.summary());
//!             break;
//!         }
//!     }
//! }
//! # Ok::<(), helix_trainer::error::UserError>(())
//! ```

use crate::config::Scenario;
use crate::game::GameError;
use crate::game::{LiveScenario, PerformanceRating, Scorer};
use crate::helix::{EditorSnapshot, Mode};
use crate::ids::ScenarioId;
use crate::security::SecurityError;
use crate::time::{MonotonicClock, SystemMonotonicClock};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use typestate::{Abandoned, Active, Completed, SessionState};

/// Represents a single user action during gameplay
///
/// Stores the command/key sequence and timestamp for tracking
/// action sequence and timing analytics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAction {
    /// The command or key sequence entered by user
    pub command: String,
    /// Elapsed time when action was performed
    pub timestamp: Duration,
}

impl UserAction {
    /// Create a new user action with timestamp
    ///
    /// # Arguments
    /// * `command` - The command/key sequence entered
    /// * `elapsed` - Elapsed time since session start
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::UserAction;
    /// use std::time::Duration;
    ///
    /// let action = UserAction::new("d".to_string(), Duration::from_secs(1));
    /// assert_eq!(action.command, "d");
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn new(command: String, elapsed: Duration) -> Self {
        Self {
            command,
            timestamp: elapsed,
        }
    }
}

/// How a scenario run ended
///
/// Computed once when the feedback is built; the boolean views [`Feedback::success`] and
/// [`Feedback::is_optimal`] derive from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackOutcome {
    /// Completed within the scenario's optimal action count plus its tolerance
    Optimal,
    /// Completed, but with more actions than the optimal count plus tolerance
    Completed,
    /// Abandoned before reaching the target
    Abandoned,
}

/// Feedback provided to user after completing a scenario
///
/// Contains performance metrics, score, and guidance for improvement.
#[derive(Debug, Clone)]
pub struct Feedback {
    /// ID of the scenario that was completed
    pub scenario_id: ScenarioId,
    /// Score earned (0 to `max_points`)
    pub score: u32,
    /// Maximum possible points for this scenario
    pub max_points: u32,
    /// Performance rating (Perfect, Excellent, Good, Fair, Poor)
    pub rating: PerformanceRating,
    /// Number of actions actually taken
    pub actions_taken: usize,
    /// Optimal number of actions for this scenario
    pub optimal_actions: usize,
    /// Total time taken to complete scenario
    pub duration: Duration,
    /// Optional hint if user struggled
    pub hint: Option<String>,
    /// How the run ended; derive the boolean views through [`Feedback::success`] and
    /// [`Feedback::is_optimal`]
    pub outcome: FeedbackOutcome,
    /// All user actions taken (for FSRS tracking)
    pub user_actions: Vec<UserAction>,
}

impl Feedback {
    /// Whether the scenario was completed, optimally or not
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::{Feedback, FeedbackOutcome, PerformanceRating};
    /// use helix_trainer::ids::ScenarioId;
    /// use std::time::Duration;
    ///
    /// let mut feedback = Feedback {
    ///     outcome: FeedbackOutcome::Completed,
    ///     score: 80,
    ///     max_points: 100,
    ///     rating: PerformanceRating::Good,
    ///     actions_taken: 5,
    ///     optimal_actions: 2,
    ///     duration: Duration::from_secs(5),
    ///     hint: None,
    ///     scenario_id: ScenarioId::parse("delete_line")?,
    ///     user_actions: Vec::new(),
    /// };
    /// assert!(feedback.success());
    /// assert!(!feedback.is_optimal());
    /// feedback.outcome = FeedbackOutcome::Abandoned;
    /// assert!(!feedback.success());
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    #[must_use]
    pub fn success(&self) -> bool {
        !matches!(self.outcome, FeedbackOutcome::Abandoned)
    }

    /// Whether the run stayed within the optimal action count plus the scenario's tolerance
    #[must_use]
    pub fn is_optimal(&self) -> bool {
        matches!(self.outcome, FeedbackOutcome::Optimal)
    }

    /// Get a summary message for the user
    ///
    /// Returns a human-readable single-line summary with emoji, score,
    /// and action counts.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::game::{Feedback, FeedbackOutcome, PerformanceRating};
    /// use helix_trainer::ids::ScenarioId;
    /// use std::time::Duration;
    ///
    /// let feedback = Feedback {
    ///     outcome: FeedbackOutcome::Optimal,
    ///     score: 100,
    ///     max_points: 100,
    ///     rating: PerformanceRating::Perfect,
    ///     actions_taken: 2,
    ///     optimal_actions: 2,
    ///     duration: Duration::from_secs(5),
    ///     hint: None,
    ///     scenario_id: ScenarioId::parse("delete_line")?,
    ///     user_actions: Vec::new(),
    /// };
    /// let summary = feedback.summary();
    /// assert!(summary.contains("100/100"));
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn summary(&self) -> String {
        if self.success() {
            format!(
                "{} Score: {}/{} - {} actions (optimal: {})",
                self.rating.emoji(),
                self.score,
                self.max_points,
                self.actions_taken,
                self.optimal_actions
            )
        } else {
            "Scenario not completed. Try again!".to_string()
        }
    }
}

/// Result of recording an action - either still active or completed
///
/// This enum is returned by `GameSession<Active>::record_action()` to indicate
/// whether the session is still ongoing or has transitioned to completed.
#[derive(Debug)]
pub enum SessionAfterAction {
    /// Session is still active after the action
    StillActive(GameSession<Active>),
    /// Session completed after the action
    Completed(GameSession<Completed>),
}

impl SessionAfterAction {
    /// Check if session is still active
    pub fn is_active(&self) -> bool {
        matches!(self, Self::StillActive(_))
    }

    /// Check if session completed
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed(_))
    }
}

/// Manages a single training scenario session with compile-time state tracking
///
/// Uses the typestate pattern to enforce valid state transitions at compile time.
/// The `State` parameter can be `Active`, `Completed`, or `Abandoned`, which
/// determines which methods are available.
///
/// Tracks the user's progress through a scenario, including:
/// - Initial and target editor states
/// - Current editor state
/// - All user actions taken
/// - Session timing
///
/// The session validates all state transitions and provides
/// score calculation and feedback generation.
pub struct GameSession<State: SessionState = Active> {
    /// The scenario in play: simulator, target and the actions taken
    live: LiveScenario,
    /// When the session started
    started_at: Instant,
    /// When the session completed (None if still active/abandoned)
    completed_at: Option<Instant>,
    /// Source of `started_at`/`completed_at` instants
    clock: Arc<dyn MonotonicClock>,
    /// Number of hints shown to user
    hints_shown: usize,
    /// Cached completion progress percentage (0-100)
    cached_progress: Cell<Option<u8>>,
    /// Flag indicating if progress cache needs update
    progress_needs_update: Cell<bool>,
    /// Phantom data for typestate pattern
    _state: PhantomData<State>,
}

// Manual Debug implementation because Cell doesn't implement Debug
impl<State: SessionState> std::fmt::Debug for GameSession<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSession")
            .field("scenario_id", &self.live.scenario().id)
            .field("action_count", &self.live.action_count())
            .field("hints_shown", &self.hints_shown)
            .field("elapsed", &self.elapsed())
            .field("state", &std::any::type_name::<State>())
            .finish_non_exhaustive()
    }
}

// Methods available in ALL states
impl<S: SessionState> GameSession<S> {
    /// Get reference to the scenario being played
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario.clone())?;
    /// assert_eq!(session.scenario().id, scenario.id);
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn scenario(&self) -> &Scenario {
        self.live.scenario()
    }

    /// Get the number of actions taken so far
    pub fn action_count(&self) -> usize {
        self.live.action_count()
    }

    /// Get elapsed time since session start
    pub fn elapsed(&self) -> Duration {
        self.clock.now().saturating_duration_since(self.started_at)
    }

    /// Get current editor mode
    pub fn mode(&self) -> Mode {
        self.live.mode()
    }

    /// Check if the simulator is in Insert mode
    pub fn is_insert_mode(&self) -> bool {
        self.live.is_insert_mode()
    }

    /// Check if a `Q`/`q` macro is currently being recorded
    pub fn is_recording_macro(&self) -> bool {
        self.live.is_recording_macro()
    }

    /// Get number of hints shown so far
    pub fn hints_shown(&self) -> usize {
        self.hints_shown
    }

    /// Get reference to all user actions
    ///
    /// Returns a slice of all actions taken during the session.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario)?;
    /// assert!(session.actions().is_empty());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn actions(&self) -> &[UserAction] {
        self.live.actions()
    }

    /// Calculate completion progress as percentage (0-100)
    ///
    /// Compares current state with target state line by line and returns
    /// the percentage of lines that match. Used for progress visualization.
    ///
    /// This method uses caching to avoid recalculating progress on every call.
    /// The cache is invalidated when the editor state changes.
    ///
    /// Uses interior mutability (Cell) to allow caching with immutable self.
    pub fn completion_progress(&self) -> u8 {
        if self.progress_needs_update.get() {
            self.cached_progress.set(Some(self.calculate_progress()));
            self.progress_needs_update.set(false);
        }
        self.cached_progress.get().unwrap_or(0)
    }

    /// Calculate completion progress by comparing current and target states
    ///
    /// This is a private helper method that performs the actual calculation.
    /// The public `completion_progress()` method caches this result.
    fn calculate_progress(&self) -> u8 {
        let current_content = self.live.current_content();
        let target_content = &self.live.target_snapshot().content;

        let current_lines: Vec<&str> = current_content.lines().collect();
        let target_lines: Vec<&str> = target_content.lines().collect();

        // If target has no lines, consider 100% complete
        if target_lines.is_empty() {
            return 100;
        }

        // Count matching lines
        let matching_lines = current_lines
            .iter()
            .zip(target_lines.iter())
            .filter(|(current, target)| current == target)
            .count();

        let text_percentage = (matching_lines * 100) / target_lines.len().max(1);
        let percentage = if text_percentage < 100 {
            text_percentage
        } else {
            self.cursor_progress()
        };

        // Mirror matches_snapshot's mode requirement: content can match the
        // target while still in Insert mode (e.g. right after `o`), but the
        // scenario isn't actually complete until Escape returns to Normal
        // mode, so don't report 100% until then.
        let cap = if self.mode() == Mode::Normal { 100 } else { 99 };
        u8::try_from(percentage.min(cap)).unwrap_or(100)
    }

    /// Progress once the text is on target: 100 only when cursor/selection
    /// also match, otherwise a value below 100 from their remaining distance.
    fn cursor_progress(&self) -> usize {
        let current = self.live.simulator().to_snapshot();
        let target = self.live.target_snapshot();
        if current.matches(target) {
            return 100;
        }
        let (distance, max_distance) = selection_distance(&current, target);
        let remaining = max_distance.saturating_sub(distance);
        (remaining * 100 / max_distance).min(99)
    }
}

/// Distance between the cursor/selection of two snapshots and its maximum.
///
/// The maximum is always at least 1, so callers can divide by it.
fn selection_distance(current: &EditorSnapshot, target: &EditorSnapshot) -> (usize, usize) {
    let doc_len = current
        .content
        .chars()
        .count()
        .max(target.content.chars().count())
        .max(1);

    let target_is_cursor_only = target.selections.len() <= 1
        && target
            .selections
            .iter()
            .all(crate::helix::simulator::SerializableRange::is_point);
    if target_is_cursor_only {
        let distance = current.cursor_offset().abs_diff(target.cursor_offset());
        return (distance.min(doc_len), doc_len);
    }

    let normalized = |snapshot: &EditorSnapshot| {
        let mut ranges: Vec<(usize, usize)> = snapshot
            .selections
            .iter()
            .map(|r| (r.anchor.min(r.head), r.anchor.max(r.head)))
            .collect();
        ranges.sort_unstable();
        ranges
    };
    let (current_ranges, target_ranges) = (normalized(current), normalized(target));
    let range_max = 2 * doc_len;
    let paired: usize = current_ranges
        .iter()
        .zip(&target_ranges)
        .map(|(a, b)| (a.0.abs_diff(b.0) + a.1.abs_diff(b.1)).min(range_max))
        .sum();
    let unpaired = current_ranges.len().abs_diff(target_ranges.len()) * range_max;
    let range_count = current_ranges.len().max(target_ranges.len()).max(1);
    (paired + unpaired, range_count * range_max)
}

// Methods ONLY available in Active state
impl GameSession<Active> {
    /// Create a new game session for a scenario
    ///
    /// Initializes the session with the scenario's setup state and
    /// prepares it for user interaction.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if:
    /// - Scenario setup or target content is invalid
    /// - Cursor positions are out of bounds
    /// - Content size exceeds limits
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario)?;
    /// assert_eq!(session.action_count(), 0);
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn new(scenario: Scenario) -> Result<Self, GameError> {
        Self::with_clock(scenario, Arc::new(SystemMonotonicClock))
    }

    /// Create a session that reads time from `clock` instead of the system clock.
    ///
    /// Tests inject a `FakeMonotonicClock` (see [`crate::time`]) to drive elapsed time
    /// without sleeping.
    ///
    /// # Errors
    ///
    /// Same as [`GameSession::new`].
    pub fn with_clock(
        scenario: Scenario,
        clock: Arc<dyn MonotonicClock>,
    ) -> Result<Self, GameError> {
        Ok(Self {
            live: LiveScenario::new(Arc::new(scenario)),
            started_at: clock.now(),
            completed_at: None,
            clock,
            hints_shown: 0,
            cached_progress: Cell::new(None),
            progress_needs_update: Cell::new(true),
            _state: PhantomData,
        })
    }

    /// Record a user action and execute it through the simulator
    ///
    /// Consumes self and returns either a still-active session or a completed session.
    /// This enforces the state transition at compile time.
    ///
    /// Validates that the action count doesn't exceed security limits,
    /// executes the command through the Helix simulator, and synchronizes
    /// the editor state with the simulator's internal state.
    ///
    /// # Errors
    ///
    /// Returns `SecurityError::TooManyActions` if action count would
    /// exceed the maximum allowed, or [`GameError`] if command execution fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::{GameSession, SessionAfterAction};
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let commands = scenario.solution.commands.clone();
    /// let mut session = GameSession::new(scenario)?;
    ///
    /// // Every command but the last leaves the session active
    /// let (last, rest) = commands.split_last().expect("solutions are never empty");
    /// for command in rest {
    ///     match session.record_action(command.clone())? {
    ///         SessionAfterAction::StillActive(next) => session = next,
    ///         SessionAfterAction::Completed(_) => unreachable!("completes only on the last command"),
    ///     }
    /// }
    ///
    /// // The last command completes the scenario
    /// let SessionAfterAction::Completed(done) = session.record_action(last.clone())? else {
    ///     panic!("the solution completes the scenario");
    /// };
    /// assert!(done.feedback()?.success());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn record_action(
        mut self,
        command: impl AsRef<str>,
    ) -> Result<SessionAfterAction, GameError> {
        self.apply_action(command.as_ref())?;
        Ok(self.finish_action())
    }

    /// Execute and record one command without consuming the session.
    ///
    /// On `Err` the action is not recorded and the session stays usable, so a
    /// caller holding it behind a screen can report the error and carry on
    /// instead of losing the session. Pair with [`GameSession::finish_action`].
    ///
    /// # Errors
    ///
    /// Returns an error if the action limit is exceeded or the simulator
    /// rejects the command.
    pub fn apply_action(&mut self, command: &str) -> Result<(), GameError> {
        let at = self.elapsed();
        self.live.apply_text(command, at)?;
        self.progress_needs_update.set(true);
        Ok(())
    }

    /// Execute and record one parsed Normal-mode command, as
    /// [`GameSession::apply_action`] does for its string form: the recorded
    /// action is the command's canonical string.
    ///
    /// # Errors
    ///
    /// Returns an error if the action limit is exceeded or the simulator
    /// rejects the command.
    pub(crate) fn apply_command(
        &mut self,
        command: &crate::helix::normal_command::NormalCommand,
    ) -> Result<(), GameError> {
        let at = self.elapsed();
        self.live.apply_command(command, at)?;
        self.progress_needs_update.set(true);
        Ok(())
    }

    /// Resolve the session state after [`GameSession::apply_action`]: completed
    /// if the editor now matches the target, otherwise still active.
    pub fn finish_action(self) -> SessionAfterAction {
        if self.live.is_completed() {
            SessionAfterAction::Completed(self.into_completed())
        } else {
            SessionAfterAction::StillActive(self)
        }
    }

    /// Transition to completed state (private helper)
    fn into_completed(self) -> GameSession<Completed> {
        GameSession {
            live: self.live,
            started_at: self.started_at,
            completed_at: Some(self.clock.now()),
            clock: self.clock,
            hints_shown: self.hints_shown,
            cached_progress: self.cached_progress,
            progress_needs_update: self.progress_needs_update,
            _state: PhantomData,
        }
    }

    /// Check if the scenario is completed successfully
    ///
    /// Uses `matches_snapshot()` for efficient comparison directly
    /// against helix-core primitives.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario)?;
    /// assert!(!session.check_completion());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn check_completion(&self) -> bool {
        self.live.is_completed()
    }

    /// Check if content matches target (ignoring cursor position)
    ///
    /// Returns true if file content is correct but cursor position
    /// may differ from target.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario)?;
    /// // The setup text differs from the target until the player edits it
    /// assert!(!session.check_content_matches());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn check_content_matches(&self) -> bool {
        self.live
            .simulator()
            .to_snapshot()
            .content_matches(self.live.target_snapshot())
    }

    /// Get the next available hint
    ///
    /// Returns hints in order from the scenario. Once all hints are
    /// shown, subsequent calls return None.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let mut session = GameSession::new(scenario)?;
    /// if let Some(hint) = session.hint() {
    ///     assert_eq!(session.hints_shown(), 1);
    ///     println!("Hint: {hint}");
    /// }
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn hint(&mut self) -> Option<String> {
        if self.hints_shown < self.live.scenario().hints.len() {
            let hint = self.live.scenario().hints[self.hints_shown].clone();
            self.hints_shown += 1;
            Some(hint)
        } else {
            None
        }
    }

    /// Abandon the session (give up)
    ///
    /// Consumes self and returns a session in the Abandoned state.
    /// This results in a score of 0 if feedback is requested.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario)?;
    /// let abandoned = session.abandon();
    /// let feedback = abandoned.feedback();
    /// assert_eq!(feedback.score, 0);
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn abandon(self) -> GameSession<Abandoned> {
        GameSession {
            live: self.live,
            started_at: self.started_at,
            completed_at: None,
            clock: self.clock,
            hints_shown: self.hints_shown,
            cached_progress: self.cached_progress,
            progress_needs_update: self.progress_needs_update,
            _state: PhantomData,
        }
    }

    /// Reset the session to start over
    ///
    /// Clears all actions, resets state to initial editor state,
    /// and keeps session as Active. Allows user to retry
    /// the same scenario.
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if state validation fails during reset.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let mut session = GameSession::new(scenario)?;
    /// session.hint();
    /// session.reset()?;
    /// assert_eq!(session.hints_shown(), 0);
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn reset(&mut self) -> Result<(), SecurityError> {
        self.live.reset();
        self.started_at = self.clock.now();
        self.completed_at = None;
        self.hints_shown = 0;
        // Reset progress cache
        self.cached_progress.set(None);
        self.progress_needs_update.set(true);
        Ok(())
    }
}

// Methods ONLY available in Completed state
impl GameSession<Completed> {
    /// Calculate the final score for this session
    ///
    /// Applies the scenario's scoring configuration to the actual
    /// number of actions taken. Guaranteed to be called on a completed session.
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if score calculation fails (e.g., overflow).
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::{GameSession, SessionAfterAction};
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// # let mut active = GameSession::new(scenario.clone())?;
    /// # let mut completed = None;
    /// # for command in &scenario.solution.commands {
    /// #     match active.record_action(command.clone())? {
    /// #         SessionAfterAction::StillActive(next) => active = next,
    /// #         SessionAfterAction::Completed(done) => {
    /// #             completed = Some(done);
    /// #             break;
    /// #         }
    /// #     }
    /// # }
    /// # let session = completed.expect("the canonical solution completes the scenario");
    /// let score = session.score()?;
    /// assert!(score > 0);
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn score(&self) -> Result<u32, SecurityError> {
        Scorer::score_with_config(&self.live.scenario().scoring, self.live.action_count())
    }

    /// Get detailed feedback for the completed session
    ///
    /// Generates comprehensive feedback including score, performance
    /// rating, hint if needed, and optimality assessment.
    ///
    /// # Errors
    ///
    /// Returns `SecurityError` if feedback generation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::{GameSession, SessionAfterAction};
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// # let mut active = GameSession::new(scenario.clone())?;
    /// # let mut completed = None;
    /// # for command in &scenario.solution.commands {
    /// #     match active.record_action(command.clone())? {
    /// #         SessionAfterAction::StillActive(next) => active = next,
    /// #         SessionAfterAction::Completed(done) => {
    /// #             completed = Some(done);
    /// #             break;
    /// #         }
    /// #     }
    /// # }
    /// # let session = completed.expect("the canonical solution completes the scenario");
    /// let feedback = session.feedback()?;
    /// assert!(feedback.success());
    /// println!("{}", feedback.summary());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the session carries no completion time, which the typestate rules
    /// out for a completed session.
    pub fn feedback(&self) -> Result<Feedback, SecurityError> {
        let actions_taken = self.live.action_count();
        let optimal_actions = self.live.scenario().scoring.optimal_count.get();
        let max_points = self.live.scenario().scoring.max_points;

        let score = self.score()?;
        let rating = Scorer::rating(score, max_points);

        let duration = self
            .completed_at
            .expect("Completed session must have completion time")
            .duration_since(self.started_at);

        // Provide hint if user struggled (took >2x optimal actions)
        let hint = if actions_taken > optimal_actions * 2 {
            Some(format!(
                "Try using: {}. {}",
                self.live.scenario().solution.commands.join(", "),
                self.live.scenario().solution.description
            ))
        } else {
            None
        };

        let outcome = if actions_taken <= optimal_actions + self.live.scenario().scoring.tolerance {
            FeedbackOutcome::Optimal
        } else {
            FeedbackOutcome::Completed
        };

        Ok(Feedback {
            scenario_id: self.live.scenario().id.clone(),
            score,
            max_points,
            rating,
            actions_taken,
            optimal_actions,
            duration,
            hint,
            outcome,
            user_actions: self.live.actions().to_vec(),
        })
    }
}

// Methods ONLY available in Abandoned state
impl GameSession<Abandoned> {
    /// Get feedback for abandoned session (score = 0)
    ///
    /// Returns feedback indicating the session was not completed,
    /// with a score of 0 and the solution as a hint.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
    /// let session = GameSession::new(scenario)?.abandon();
    /// let feedback = session.feedback();
    /// assert_eq!(feedback.score, 0);
    /// assert!(!feedback.success());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn feedback(&self) -> Feedback {
        Feedback {
            scenario_id: self.live.scenario().id.clone(),
            score: 0,
            max_points: self.live.scenario().scoring.max_points,
            rating: PerformanceRating::Poor,
            actions_taken: self.live.action_count(),
            optimal_actions: self.live.scenario().scoring.optimal_count.get(),
            duration: self.elapsed(),
            hint: Some(format!(
                "Solution: {}. {}",
                self.live.scenario().solution.commands.join(", "),
                self.live.scenario().solution.description
            )),
            outcome: FeedbackOutcome::Abandoned,
            user_actions: self.live.actions().to_vec(),
        }
    }
}

/// Typestate pattern markers for compile-time state machine enforcement
pub mod typestate;

// Implement PlayableScenario trait for GameSession in any state; only `elapsed()`
// differs per state, delegated to `SessionState::session_elapsed`.
impl<S: SessionState> super::PlayableScenario for GameSession<S> {
    fn live(&self) -> &LiveScenario {
        &self.live
    }

    fn elapsed(&self) -> Duration {
        S::session_elapsed(self.started_at, self.completed_at, self.clock.now())
    }
}

#[cfg(test)]
mod tests;
