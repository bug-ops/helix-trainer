//! Game session management for training scenarios
//!
//! This module provides the GameSession type which manages the user's attempt
//! at solving a scenario. It tracks user actions, maintains game state, and
//! provides feedback based on performance.
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::game::GameSession;
//! use helix_trainer::config::Scenario;
//!
//! // Create a new session for a scenario
//! let scenario = /* load from file */;
//! let mut session = GameSession::new(scenario)?;
//!
//! // User performs actions
//! session.record_action("d".to_string())?;
//! session.record_action("d".to_string())?;
//!
//! // Update editor state
//! let new_state = /* get from editor */;
//! session.update_state(new_state)?;
//!
//! // Check if scenario is complete
//! if session.is_completed() {
//!     let feedback = session.get_feedback()?;
//!     println!("{}", feedback.summary());
//! }
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```

use crate::config::Scenario;
use crate::game::{EditorState, PerformanceRating, Scorer};
use crate::helix::{AnyModeSimulator, Mode};
use crate::security::{self, SecurityError, UserError};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::marker::PhantomData;
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
    /// ```ignore
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

/// Feedback provided to user after completing a scenario
///
/// Contains performance metrics, score, and guidance for improvement.
#[derive(Debug, Clone)]
pub struct Feedback {
    /// ID of the scenario that was completed
    pub scenario_id: String,
    /// Whether the scenario was completed successfully
    pub success: bool,
    /// Score earned (0 to max_points)
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
    /// Whether user achieved optimal solution
    pub is_optimal: bool,
    /// All user actions taken (for FSRS tracking)
    pub user_actions: Vec<UserAction>,
}

impl Feedback {
    /// Get a summary message for the user
    ///
    /// Returns a human-readable single-line summary with emoji, score,
    /// and action counts.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{Feedback, PerformanceRating};
    /// use std::time::Duration;
    ///
    /// let feedback = Feedback {
    ///     success: true,
    ///     score: 100,
    ///     max_points: 100,
    ///     rating: PerformanceRating::Perfect,
    ///     actions_taken: 2,
    ///     optimal_actions: 2,
    ///     duration: Duration::from_secs(5),
    ///     hint: None,
    ///     is_optimal: true,
    /// };
    /// let summary = feedback.summary();
    /// assert!(summary.contains("100/100"));
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn summary(&self) -> String {
        if self.success {
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
    /// The scenario being played
    scenario: Scenario,
    /// Initial state from scenario setup
    initial_state: EditorState,
    /// Target state to achieve
    target_state: EditorState,
    /// Current editor state
    current_state: EditorState,
    /// Helix editor simulator for command execution
    simulator: AnyModeSimulator,
    /// All user actions taken so far
    user_actions: Vec<UserAction>,
    /// When the session started
    started_at: Instant,
    /// When the session completed (None if still active/abandoned)
    completed_at: Option<Instant>,
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
            .field("scenario_id", &self.scenario.id)
            .field("action_count", &self.user_actions.len())
            .field("hints_shown", &self.hints_shown)
            .field("elapsed", &self.started_at.elapsed())
            .field("state", &std::any::type_name::<State>())
            .finish()
    }
}

// Methods available in ALL states
impl<S: SessionState> GameSession<S> {
    /// Get reference to the scenario being played
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = /* scenario */;
    /// let session = GameSession::new(scenario.clone())?;
    /// assert_eq!(session.scenario().id, scenario.id);
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    /// Get reference to the current editor state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let session = GameSession::new(scenario)?;
    /// let current = session.current_state();
    /// assert_eq!(current.content(), scenario.setup.file_content.as_str());
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn current_state(&self) -> &EditorState {
        &self.current_state
    }

    /// Get reference to the target editor state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let session = GameSession::new(scenario)?;
    /// let target = session.target_state();
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn target_state(&self) -> &EditorState {
        &self.target_state
    }

    /// Get the number of actions taken so far
    pub fn action_count(&self) -> usize {
        self.user_actions.len()
    }

    /// Get elapsed time since session start
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get current editor mode
    pub fn mode(&self) -> Mode {
        self.simulator.mode()
    }

    /// Check if the simulator is in Insert mode
    pub fn is_insert_mode(&self) -> bool {
        self.simulator.mode() == Mode::Insert
    }

    /// Get current editor mode as string for UI display
    pub fn mode_name(&self) -> &str {
        match self.simulator.mode() {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        }
    }

    /// Get number of hints shown so far
    pub fn hints_shown(&self) -> usize {
        self.hints_shown
    }

    /// Get reference to initial editor state
    pub fn initial_state(&self) -> &EditorState {
        &self.initial_state
    }

    /// Get reference to all user actions
    ///
    /// Returns a slice of all actions taken during the session.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let session = GameSession::new(scenario)?;
    /// let actions = session.actions();
    /// println!("Total actions: {}", actions.len());
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn actions(&self) -> &[UserAction] {
        &self.user_actions
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
        let current_content = self.current_state.content();
        let target_content = self.target_state.content();

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

        // Calculate percentage (0-100)
        let percentage = (matching_lines * 100) / target_lines.len().max(1);
        percentage.min(100) as u8
    }
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
    /// Returns `UserError` if:
    /// - Scenario setup or target content is invalid
    /// - Cursor positions are out of bounds
    /// - Content size exceeds limits
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let scenario = /* load from file */;
    /// let session = GameSession::new(scenario)?;
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn new(scenario: Scenario) -> Result<Self, UserError> {
        // Create initial state from scenario setup
        let initial_state = EditorState::from_setup(
            &scenario.setup.file_content,
            [
                scenario.setup.cursor_position.0,
                scenario.setup.cursor_position.1,
            ],
        )
        .map_err(|_| UserError::ScenarioTooComplex)?;

        // Create target state with optional selection
        let target_state = EditorState::from_target(
            &scenario.target.file_content,
            [
                scenario.target.cursor_position.0,
                scenario.target.cursor_position.1,
            ],
            scenario.target.selection,
        )
        .map_err(|_| UserError::ScenarioTooComplex)?;

        // Clone initial state as current state
        let current_state = initial_state.clone();

        // Initialize Helix simulator from initial state (includes cursor position)
        let simulator = AnyModeSimulator::from_editor_state(&initial_state);

        Ok(Self {
            scenario,
            initial_state,
            target_state,
            current_state,
            simulator,
            user_actions: Vec::new(),
            started_at: Instant::now(),
            completed_at: None,
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
    /// exceed the maximum allowed, or `UserError` if command execution fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::{GameSession, SessionAfterAction};
    /// use helix_trainer::helix::commands::CMD_DELETE_LINE;
    ///
    /// let session = GameSession::new(scenario)?;
    /// match session.record_action(CMD_DELETE_LINE.to_string())? {
    ///     SessionAfterAction::StillActive(session) => {
    ///         // Continue playing
    ///     }
    ///     SessionAfterAction::Completed(session) => {
    ///         // Session complete, get feedback
    ///         let feedback = session.feedback()?;
    ///     }
    /// }
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn record_action(mut self, command: String) -> Result<SessionAfterAction, UserError> {
        // Validate action count doesn't exceed limits
        security::arithmetic::validate_action_count(self.user_actions.len() + 1)
            .map_err(UserError::from)?;

        // Execute command through simulator
        self.simulator.execute_command(&command)?;

        // Sync current state with simulator
        self.current_state = self.simulator.to_editor_state()?;

        // Invalidate progress cache since state changed
        self.progress_needs_update.set(true);

        // Record action in history
        let elapsed = self.elapsed();
        let action = UserAction::new(command, elapsed);
        self.user_actions.push(action);

        // Check if scenario is completed and return appropriate state
        if self.check_completion() {
            Ok(SessionAfterAction::Completed(self.into_completed()))
        } else {
            Ok(SessionAfterAction::StillActive(self))
        }
    }

    /// Transition to completed state (private helper)
    fn into_completed(self) -> GameSession<Completed> {
        GameSession {
            scenario: self.scenario,
            initial_state: self.initial_state,
            target_state: self.target_state,
            current_state: self.current_state,
            simulator: self.simulator,
            user_actions: self.user_actions,
            started_at: self.started_at,
            completed_at: Some(Instant::now()),
            hints_shown: self.hints_shown,
            cached_progress: self.cached_progress,
            progress_needs_update: self.progress_needs_update,
            _state: PhantomData,
        }
    }

    /// Check if the scenario is completed successfully
    ///
    /// Compares current state with target state (both content and cursor).
    /// Returns true only if they match exactly.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let mut session = GameSession::new(scenario)?;
    /// assert!(!session.check_completion());
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn check_completion(&self) -> bool {
        self.current_state.matches(&self.target_state)
    }

    /// Check if content matches target (ignoring cursor position)
    ///
    /// Returns true if file content is correct but cursor position
    /// may differ from target.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let session = GameSession::new(scenario)?;
    /// if session.check_content_matches() {
    ///     println!("Content is correct!");
    /// }
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn check_content_matches(&self) -> bool {
        self.current_state.content_matches(&self.target_state)
    }

    /// Get the next available hint
    ///
    /// Returns hints in order from the scenario. Once all hints are
    /// shown, subsequent calls return None.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let mut session = GameSession::new(scenario)?;
    /// if let Some(hint) = session.get_hint() {
    ///     println!("Hint: {}", hint);
    /// }
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn get_hint(&mut self) -> Option<String> {
        if self.hints_shown < self.scenario.hints.len() {
            let hint = self.scenario.hints[self.hints_shown].clone();
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
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let session = GameSession::new(scenario)?;
    /// let abandoned = session.abandon();
    /// let feedback = abandoned.feedback();
    /// assert_eq!(feedback.score, 0);
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn abandon(self) -> GameSession<Abandoned> {
        GameSession {
            scenario: self.scenario,
            initial_state: self.initial_state,
            target_state: self.target_state,
            current_state: self.current_state,
            simulator: self.simulator,
            user_actions: self.user_actions,
            started_at: self.started_at,
            completed_at: None,
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
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// let mut session = GameSession::new(scenario)?;
    /// // ... play some ...
    /// session.reset()?;
    /// assert_eq!(session.action_count(), 0);
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn reset(&mut self) -> Result<(), SecurityError> {
        self.current_state = self.initial_state.clone();
        // Reset simulator to initial content
        self.simulator = AnyModeSimulator::new(self.scenario.setup.file_content.clone());
        self.user_actions.clear();
        self.started_at = Instant::now();
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
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// // session is GameSession<Completed>
    /// let score = session.score()?;
    /// println!("Score: {}", score);
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn score(&self) -> Result<u32, SecurityError> {
        Scorer::score_with_config(&self.scenario.scoring, self.user_actions.len())
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
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// // session is GameSession<Completed>
    /// let feedback = session.feedback()?;
    /// println!("{}", feedback.summary());
    /// # Ok::<(), helix_trainer::security::SecurityError>(())
    /// ```
    pub fn feedback(&self) -> Result<Feedback, SecurityError> {
        let actions_taken = self.user_actions.len();
        let optimal_actions = self.scenario.scoring.optimal_count;
        let max_points = self.scenario.scoring.max_points;

        let score = self.score()?;
        let rating = Scorer::get_rating(score, max_points);

        let duration = self
            .completed_at
            .expect("Completed session must have completion time")
            .duration_since(self.started_at);

        // Provide hint if user struggled (took >2x optimal actions)
        let hint = if actions_taken > optimal_actions * 2 {
            Some(format!(
                "Try using: {}. {}",
                self.scenario.solution.commands.join(", "),
                self.scenario.solution.description
            ))
        } else {
            None
        };

        let is_optimal = actions_taken <= optimal_actions + self.scenario.scoring.tolerance;

        Ok(Feedback {
            scenario_id: self.scenario.id.clone(),
            success: true,
            score,
            max_points,
            rating,
            actions_taken,
            optimal_actions,
            duration,
            hint,
            is_optimal,
            user_actions: self.user_actions.clone(),
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
    /// ```ignore
    /// use helix_trainer::game::GameSession;
    ///
    /// // session is GameSession<Abandoned>
    /// let feedback = session.feedback();
    /// assert_eq!(feedback.score, 0);
    /// assert!(!feedback.success);
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn feedback(&self) -> Feedback {
        Feedback {
            scenario_id: self.scenario.id.clone(),
            success: false,
            score: 0,
            max_points: self.scenario.scoring.max_points,
            rating: PerformanceRating::Poor,
            actions_taken: self.user_actions.len(),
            optimal_actions: self.scenario.scoring.optimal_count,
            duration: self.started_at.elapsed(),
            hint: Some(format!(
                "Solution: {}. {}",
                self.scenario.solution.commands.join(", "),
                self.scenario.solution.description
            )),
            is_optimal: false,
            user_actions: self.user_actions.clone(),
        }
    }
}

/// Typestate pattern markers for compile-time state machine enforcement
pub mod typestate;

#[cfg(test)]
mod tests;
