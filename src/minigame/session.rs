//! Mini-game session management
//!
//! Manages the complete mini-game session including scenario queue,
//! active scenario, timing, and score calculation.

use crate::config::{Difficulty, Scenario};
use crate::constants::EXTRA_LIFE_SCORE_MILESTONE;
use crate::game::CommandExecutor;
use crate::helix::{AnyModeSimulator, EditorSnapshot};
use crate::learning::PerformanceTracker;
use crate::minigame::{
    DifficultyController, LevelChange, MiniGameMode, MiniGameState, MiniGameStats,
    MultiplierChange, PerformancePoint, ScoreBreakdown, ScoreCalculator,
    select_challenge_scenarios,
};
use crate::security::UserError;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Size of scenario queue (number of upcoming scenarios visible)
const QUEUE_SIZE: usize = 3;

/// Helper to execute operations on a target snapshot's EditorDisplay
fn with_target_display<T, F>(snapshot: &EditorSnapshot, f: F) -> T
where
    F: FnOnce(&crate::helix::EditorDisplay) -> T,
{
    let rope = helix_core::Rope::from(snapshot.content.as_str());
    let selection = snapshot.to_helix_selection();
    let display = crate::helix::EditorDisplay::new(&rope, &selection);
    f(&display)
}

/// Active scenario being played with timer
///
/// Represents the scenario currently being solved by the player,
/// including the simulator state, timing information, and action history.
pub struct ActiveMiniScenario {
    /// The scenario being played
    pub scenario: Scenario,

    /// Helix simulator instance (source of truth for current state)
    simulator: AnyModeSimulator,

    /// Target as snapshot for efficient completion checking
    target_snapshot: EditorSnapshot,

    /// When scenario started
    started_at: Instant,

    /// Time limit for this scenario
    time_limit: Duration,

    /// Actions taken so far
    actions: Vec<String>,

    /// When the current pause span began, if paused
    paused_at: Option<Instant>,

    /// Total duration accumulated across all completed pause spans
    total_paused: Duration,
}

impl ActiveMiniScenario {
    /// Create a new active scenario
    ///
    /// # Errors
    ///
    /// Returns `UserError` if scenario setup or target state is invalid.
    fn new(scenario: Scenario, time_limit: Duration) -> Result<Self, UserError> {
        // Use unified ScenarioState helper for initialization
        let state = crate::game::ScenarioState::from_scenario(&scenario)?;

        Ok(Self {
            scenario,
            simulator: state.simulator,
            target_snapshot: state.target_snapshot,
            started_at: Instant::now(),
            time_limit,
            actions: Vec::new(),
            paused_at: None,
            total_paused: Duration::ZERO,
        })
    }

    /// Check if scenario is completed
    fn is_completed(&self) -> bool {
        self.simulator.matches_snapshot(&self.target_snapshot)
    }

    /// Freeze the countdown timer
    ///
    /// Idempotent: calling this while already paused has no effect.
    pub fn pause(&mut self) {
        if self.paused_at.is_none() {
            self.paused_at = Some(Instant::now());
        }
    }

    /// Resume the countdown timer, accumulating the just-finished pause span
    ///
    /// Idempotent: calling this while not paused has no effect.
    pub fn resume(&mut self) {
        if let Some(paused_at) = self.paused_at.take() {
            self.total_paused += paused_at.elapsed();
        }
    }

    /// Get elapsed time since scenario started, excluding any paused duration
    pub fn elapsed(&self) -> Duration {
        let paused = self.total_paused
            + self
                .paused_at
                .map(|p| p.elapsed())
                .unwrap_or(Duration::ZERO);
        self.started_at.elapsed().saturating_sub(paused)
    }

    /// Get remaining time before timeout
    pub fn remaining_time(&self) -> Duration {
        self.time_limit.saturating_sub(self.elapsed())
    }

    /// Check if scenario has timed out
    pub fn is_timed_out(&self) -> bool {
        self.elapsed() >= self.time_limit
    }

    /// Get progress percentage (0.0 to 1.0, clamped)
    ///
    /// Returns value clamped to [0.0, 1.0] range even if time has expired.
    pub fn progress_percent(&self) -> f64 {
        (self.elapsed().as_secs_f64() / self.time_limit.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// Get number of actions taken
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Get reference to actions taken
    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    /// Check if currently in Insert mode
    pub fn is_insert_mode(&self) -> bool {
        self.simulator.is_insert_mode()
    }
}

// Implement PlayableScenario trait for ActiveMiniScenario
impl crate::game::PlayableScenario for ActiveMiniScenario {
    fn current_content(&self) -> String {
        self.simulator.display().content()
    }

    fn target_content(&self) -> String {
        self.target_snapshot.content.clone()
    }

    fn current_cursor(&self) -> (usize, usize) {
        self.simulator.display().cursor_position()
    }

    fn target_cursor(&self) -> (usize, usize) {
        with_target_display(&self.target_snapshot, |d| d.cursor_position())
    }

    fn current_selection(&self) -> Option<crate::helix::SelectionBounds> {
        self.simulator.display().selection()
    }

    fn target_selection(&self) -> Option<crate::helix::SelectionBounds> {
        with_target_display(&self.target_snapshot, |d| d.selection())
    }

    fn action_count(&self) -> usize {
        self.actions.len()
    }

    fn is_insert_mode(&self) -> bool {
        self.simulator.is_insert_mode()
    }

    fn elapsed(&self) -> std::time::Duration {
        ActiveMiniScenario::elapsed(self)
    }

    fn all_cursors(&self) -> Vec<(usize, usize)> {
        self.simulator.display().all_cursor_positions()
    }

    fn all_selections(&self) -> Vec<crate::helix::SelectionBounds> {
        self.simulator
            .display()
            .all_selection_bounds()
            .into_iter()
            .map(|((sr, sc), (er, ec))| crate::helix::SelectionBounds::new(sr, sc, er, ec))
            .collect()
    }

    fn all_target_cursors(&self) -> Vec<(usize, usize)> {
        with_target_display(&self.target_snapshot, |d| d.all_cursor_positions())
    }

    fn all_target_selections(&self) -> Vec<crate::helix::SelectionBounds> {
        with_target_display(&self.target_snapshot, |d| {
            d.all_selection_bounds()
                .into_iter()
                .map(|((sr, sc), (er, ec))| crate::helix::SelectionBounds::new(sr, sc, er, ec))
                .collect()
        })
    }
}

// Implement CommandExecutor trait for unified command handling with count prefix
impl CommandExecutor for ActiveMiniScenario {
    fn execute_single(&mut self, command: &str) -> Result<(), UserError> {
        self.simulator.execute_command(command)?;
        self.actions.push(command.to_string());
        Ok(())
    }

    fn check_completion(&self) -> bool {
        self.simulator.matches_snapshot(&self.target_snapshot)
    }
}

/// Mini-game session state
///
/// Manages the complete arcade-style gameplay session including:
/// - Scenario queue and selection
/// - Active scenario execution
/// - Score and statistics tracking
/// - Difficulty adaptation
/// - State machine management
/// - FSRS-based weighted scenario selection
pub struct MiniGameSession {
    /// Current active scenario being played
    current: Option<ActiveMiniScenario>,

    /// Queue of upcoming scenarios (2-3 visible)
    queue: VecDeque<Scenario>,

    /// Game statistics
    pub(crate) stats: MiniGameStats,

    /// Difficulty controller
    difficulty: DifficultyController,

    /// Score calculator with combo tracking
    score_calculator: ScoreCalculator,

    /// Last score breakdown (for UI display)
    last_score_breakdown: Option<ScoreBreakdown>,

    /// Current game state
    state: MiniGameState,

    /// All available scenarios (reference)
    scenarios: Arc<Vec<Scenario>>,

    /// When transition state started (for auto-advance)
    transition_started_at: Option<Instant>,

    /// When the session-level clock started (set once gameplay begins after countdown)
    session_started_at: Option<Instant>,

    /// When the current session-level pause span began, if paused
    session_paused_at: Option<Instant>,

    /// Total duration accumulated across all completed session-level pause spans
    session_total_paused: Duration,

    /// Performance tracker for FSRS-based scenario selection (read-only clone)
    ///
    /// When present, scenarios with commands needing practice are prioritized.
    /// This is a snapshot of the tracker at session creation time.
    tracker: Option<PerformanceTracker>,

    /// Game mode configuration
    mode: MiniGameMode,

    /// Pre-selected scenarios for Challenge mode (None for other modes)
    selected_scenarios: Option<Vec<Scenario>>,

    /// Index into selected_scenarios for Challenge mode
    challenge_scenario_index: usize,

    /// Whether game-over bookkeeping (FSRS recording, XP award, profile save)
    /// has already run for this session.
    ///
    /// Guards against double-processing when independent call sites can each
    /// try to run bookkeeping for the same session - e.g. a per-scenario
    /// timeout that reaches [`MiniGameState::GameOver`] and a later manual
    /// quit both reacting to the same finished session. See
    /// [`Self::try_begin_game_over`].
    game_over_processed: bool,
}

impl MiniGameSession {
    /// Create a new mini-game session with scenario collection and optional FSRS tracker.
    ///
    /// Uses the default Arcade mode configuration.
    ///
    /// # Arguments
    ///
    /// * `scenarios` - Arc reference to available scenarios
    /// * `tracker` - Optional performance tracker for FSRS-based weighted selection.
    ///   When `Some`, scenarios with commands needing practice are prioritized.
    ///   When `None`, random selection is used (backward compatibility).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::learning::PerformanceTracker;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(vec![/* scenarios */]);
    /// let tracker = PerformanceTracker::new();
    ///
    /// // With FSRS weighting
    /// let session = MiniGameSession::new(scenarios.clone(), Some(tracker));
    ///
    /// // Without FSRS weighting (backward compat)
    /// let session = MiniGameSession::new(scenarios, None);
    /// ```
    pub fn new(scenarios: Arc<Vec<Scenario>>, tracker: Option<PerformanceTracker>) -> Self {
        Self::with_mode(scenarios, tracker, MiniGameMode::default())
    }

    /// Create a new mini-game session with specified game mode.
    ///
    /// # Arguments
    ///
    /// * `scenarios` - Arc reference to available scenarios
    /// * `tracker` - Optional performance tracker for FSRS-based weighted selection
    /// * `mode` - Game mode configuration (Arcade, Survival, or Challenge)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::{MiniGameSession, MiniGameMode, SurvivalConfig};
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(vec![/* scenarios */]);
    ///
    /// // Create Survival mode session
    /// let mode = MiniGameMode::Survival(SurvivalConfig::default());
    /// let session = MiniGameSession::with_mode(scenarios, None, mode);
    /// assert_eq!(session.stats().lives(), 1); // Survival mode has 1 life
    /// ```
    pub fn with_mode(
        scenarios: Arc<Vec<Scenario>>,
        tracker: Option<PerformanceTracker>,
        mode: MiniGameMode,
    ) -> Self {
        let starting_lives = mode.starting_lives();

        // For Challenge mode, pre-select scenarios using seeded RNG
        let selected_scenarios = match &mode {
            MiniGameMode::Challenge(config) => Some(select_challenge_scenarios(&scenarios, config)),
            _ => None,
        };

        let mut session = Self {
            current: None,
            queue: VecDeque::with_capacity(QUEUE_SIZE),
            stats: MiniGameStats::new_with_lives(starting_lives),
            difficulty: DifficultyController::new(),
            score_calculator: ScoreCalculator::new(),
            last_score_breakdown: None,
            state: MiniGameState::default(),
            scenarios,
            transition_started_at: None,
            session_started_at: None,
            session_paused_at: None,
            session_total_paused: Duration::ZERO,
            tracker,
            mode,
            selected_scenarios,
            challenge_scenario_index: 0,
            game_over_processed: false,
        };

        // Pre-fill queue
        session.refill_queue();

        session
    }

    /// Start the game (begins countdown)
    ///
    /// Transitions from any state to Countdown state.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    /// assert!(session.state().is_countdown());
    /// ```
    pub fn start(&mut self) {
        self.state = MiniGameState::Countdown { remaining: 3 };
    }

    /// Process countdown tick
    ///
    /// Decrements countdown and transitions to Playing when done.
    /// Should be called every second during countdown.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    ///
    /// session.tick_countdown();
    /// session.tick_countdown();
    /// session.tick_countdown();
    /// assert!(session.state().is_playing());
    /// ```
    pub fn tick_countdown(&mut self) {
        if let MiniGameState::Countdown { remaining } = self.state {
            if remaining > 1 {
                self.state = MiniGameState::Countdown {
                    remaining: remaining - 1,
                };
            } else {
                // Countdown finished, start playing
                self.state = MiniGameState::Playing;
                if self.session_started_at.is_none() {
                    self.session_started_at = Some(Instant::now());
                }
                // Load first scenario if not already loaded
                if self.current.is_none() {
                    let _ = self.load_next_scenario();
                }
            }
        }
    }

    /// Process a command input during gameplay
    ///
    /// Executes the command on the current scenario and checks for completion.
    ///
    /// # Errors
    ///
    /// Returns `UserError` if command execution fails or no scenario is active.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    /// session.handle_command("x")?; // select line
    /// session.handle_command("d")?; // delete
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn handle_command(&mut self, command: &str) -> Result<(), UserError> {
        if !self.state.is_playing() {
            return Ok(()); // Ignore commands in non-playing states
        }

        let current = self.current.as_mut().ok_or(UserError::OperationFailed)?;

        // Use CommandExecutor trait for unified count prefix handling (e.g., "3d" -> 3x "d")
        current.execute_with_count(command)?;

        Ok(())
    }

    /// Check if current scenario is complete
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// if session.check_completion() {
    ///     session.advance_to_next();
    /// }
    /// ```
    pub fn check_completion(&self) -> bool {
        self.current
            .as_ref()
            .map(|s| s.is_completed())
            .unwrap_or(false)
    }

    /// Advance to next scenario after completion
    ///
    /// Calculates score, updates statistics, and loads the next scenario.
    /// Transitions to Transition state briefly before loading next scenario.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.advance_to_next();
    /// ```
    pub fn advance_to_next(&mut self) {
        if let Some(ref scenario) = self.current {
            // Calculate metrics for scoring
            let time_ratio = scenario.progress_percent();
            let optimal_count = scenario.scenario.scoring.optimal_count.get();
            let actual_count = scenario.action_count().max(1);
            let efficiency = (optimal_count as f64 / actual_count as f64).min(1.0);
            let scenario_difficulty = scenario
                .scenario
                .metadata
                .as_ref()
                .and_then(|m| m.difficulty)
                .unwrap_or(Difficulty::Beginner);

            // Update completion counter and multiplier state (streak, grace, milestones).
            // Must precede scoring: the new multiplier feeds `calculate` and `add_score`.
            self.stats.record_completion();

            // Calculate score using enhanced ScoreCalculator
            let base_points = self.base_points_for(scenario);
            let breakdown = self.score_calculator.calculate(
                base_points,
                time_ratio,
                efficiency,
                scenario_difficulty,
                self.stats.multiplier(),
            );

            // Store breakdown for UI display
            self.last_score_breakdown = Some(breakdown.clone());

            // Award points (total already includes difficulty multiplier)
            self.stats.add_score(breakdown.total);

            // Create performance point with full data
            let performance_point =
                PerformancePoint::new(true, time_ratio, scenario_difficulty, efficiency);

            // Update difficulty with comprehensive performance data
            self.difficulty.update_after_scenario(performance_point);

            // Check for extra life milestone (every EXTRA_LIFE_SCORE_MILESTONE points)
            let prev_milestone =
                (self.stats.score.saturating_sub(breakdown.total)) / EXTRA_LIFE_SCORE_MILESTONE;
            let curr_milestone = self.stats.score / EXTRA_LIFE_SCORE_MILESTONE;
            if curr_milestone > prev_milestone {
                self.stats.gain_life();
            }
        }

        // Transition state (success)
        self.state = MiniGameState::Transition { success: true };
        self.transition_started_at = Some(Instant::now());
    }

    /// Complete transition and load next scenario
    ///
    /// Should be called after brief transition delay.
    ///
    /// # Errors
    ///
    /// Returns `UserError` if scenario loading fails.
    pub fn complete_transition(&mut self) -> Result<(), UserError> {
        if self.state.is_transition() {
            self.state = MiniGameState::Playing;
            self.transition_started_at = None;
            self.load_next_scenario()?;
        }
        Ok(())
    }

    /// Check if transition delay has elapsed and should auto-advance
    ///
    /// Returns true if in transition state and delay (1 second) has passed.
    pub fn should_advance_to_next(&self) -> bool {
        if !self.state.is_transition() {
            return false;
        }
        self.transition_started_at
            .map(|t| t.elapsed() >= Duration::from_secs(1))
            .unwrap_or(false)
    }

    /// Handle timeout for current scenario
    ///
    /// Deducts a life, resets streak, and loads next scenario.
    /// If no lives remain, transitions to GameOver state.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.handle_timeout();
    /// ```
    pub fn handle_timeout(&mut self) {
        // Update multiplier state (handles grace mechanics) and failure counter
        self.stats.record_failure();

        // Lose life
        let has_lives = self.stats.lose_life();

        // Reset combo via score calculator
        let breakdown = self.score_calculator.calculate_failure();
        self.last_score_breakdown = Some(breakdown);

        // Create performance point for failure
        let scenario_difficulty = self
            .current
            .as_ref()
            .and_then(|s| s.scenario.metadata.as_ref())
            .and_then(|m| m.difficulty)
            .unwrap_or(Difficulty::Beginner);

        let performance_point = PerformancePoint::new(
            false, // failure
            1.0,   // timeout = 100% time used
            scenario_difficulty,
            0.0, // no efficiency on failure
        );

        // Update difficulty with comprehensive performance data
        self.difficulty.update_after_scenario(performance_point);

        if has_lives {
            // Continue to next scenario (failure transition)
            self.state = MiniGameState::Transition { success: false };
            self.transition_started_at = Some(Instant::now());
        } else {
            // Game over
            self.state = MiniGameState::GameOver;
            self.current = None;
            self.transition_started_at = None;
        }
    }

    /// Get remaining time for current scenario
    ///
    /// Returns None if no scenario is active.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let session = MiniGameSession::new(scenarios, None);
    /// if let Some(time) = session.remaining_time() {
    ///     println!("Time left: {:?}", time);
    /// }
    /// ```
    pub fn remaining_time(&self) -> Option<Duration> {
        self.current.as_ref().map(|s| s.remaining_time())
    }

    /// Check if current scenario has timed out
    pub fn is_timed_out(&self) -> bool {
        self.current
            .as_ref()
            .map(|s| s.is_timed_out())
            .unwrap_or(false)
    }

    /// Get elapsed session-level time, excluding any paused duration
    ///
    /// Returns `Duration::ZERO` if gameplay has not started yet (still in countdown).
    fn session_elapsed(&self) -> Duration {
        let Some(started_at) = self.session_started_at else {
            return Duration::ZERO;
        };
        let paused = self.session_total_paused
            + self
                .session_paused_at
                .map(|p| p.elapsed())
                .unwrap_or(Duration::ZERO);
        started_at.elapsed().saturating_sub(paused)
    }

    /// Check if the mode's session-level time limit has elapsed
    ///
    /// Always `false` for modes without a session timer (see
    /// [`MiniGameMode::has_session_timer`]).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let session = MiniGameSession::new(scenarios, None);
    /// if session.is_session_expired() {
    ///     session.end_session_on_timeout();
    /// }
    /// ```
    pub fn is_session_expired(&self) -> bool {
        self.mode
            .session_duration()
            .is_some_and(|limit| self.session_elapsed() >= limit)
    }

    /// End the session immediately because its time limit (not a per-scenario
    /// timeout) has elapsed
    ///
    /// Unlike [`Self::handle_timeout`], this does not consume a life: the
    /// session clock ran out, so the run ends regardless of lives remaining.
    pub fn end_session_on_timeout(&mut self) {
        self.state = MiniGameState::GameOver;
        self.current = None;
        self.transition_started_at = None;
    }

    /// Pause the game
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.pause();
    /// assert!(session.state().is_paused());
    /// ```
    pub fn pause(&mut self) {
        if self.state.is_playing() {
            self.state = MiniGameState::Paused;
            if let Some(ref mut current) = self.current {
                current.pause();
            }
            if self.session_paused_at.is_none() {
                self.session_paused_at = Some(Instant::now());
            }
        }
    }

    /// Resume the game from paused state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.pause();
    /// session.resume();
    /// assert!(session.state().is_playing());
    /// ```
    pub fn resume(&mut self) {
        if self.state.is_paused() {
            self.state = MiniGameState::Playing;
            if let Some(ref mut current) = self.current {
                current.resume();
            }
            if let Some(paused_at) = self.session_paused_at.take() {
                self.session_total_paused += paused_at.elapsed();
            }
        }
    }

    /// Get reference to current scenario
    pub fn current_scenario(&self) -> Option<&ActiveMiniScenario> {
        self.current.as_ref()
    }

    /// Get reference to scenario queue
    pub fn queue(&self) -> &VecDeque<Scenario> {
        &self.queue
    }

    /// Get reference to statistics
    pub fn stats(&self) -> &MiniGameStats {
        &self.stats
    }

    /// Get current game state
    pub fn state(&self) -> MiniGameState {
        self.state
    }

    /// Check-and-set, in a single call, whether game-over bookkeeping should
    /// run for this session.
    ///
    /// Returns `true` the first time it is called for a given session, and
    /// `false` on every subsequent call. Callers that run game-over
    /// bookkeeping (FSRS recording, XP award, profile save) should call this
    /// unconditionally and skip the work when it returns `false`, instead of
    /// inferring "already processed" from [`MiniGameState::GameOver`] - that
    /// state can be reached, or bookkeeping can be requested mid-session,
    /// from more than one call site.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// assert!(session.try_begin_game_over());
    /// assert!(!session.try_begin_game_over());
    /// ```
    pub(crate) fn try_begin_game_over(&mut self) -> bool {
        if self.game_over_processed {
            false
        } else {
            self.game_over_processed = true;
            true
        }
    }

    /// Get difficulty level
    pub fn difficulty_level(&self) -> u8 {
        self.difficulty.current_level()
    }

    /// Take level change event (for UI notifications)
    ///
    /// Returns the level change event if one occurred, consuming it.
    /// Subsequent calls return None until another level change occurs.
    ///
    /// # Returns
    ///
    /// `Some(LevelChange)` if level changed recently, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::{MiniGameSession, LevelChange};
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// // ... gameplay ...
    ///
    /// if let Some(change) = session.take_level_change() {
    ///     match change {
    ///         LevelChange::Increased { from, to } => println!("Level up!"),
    ///         LevelChange::Decreased { from, to } => println!("Level down"),
    ///     }
    /// }
    /// ```
    pub fn take_level_change(&mut self) -> Option<LevelChange> {
        self.difficulty.take_level_change()
    }

    /// Get current performance score (0.0 to 1.0)
    ///
    /// Returns the weighted performance score based on recent scenarios.
    /// Higher scores indicate better overall performance.
    pub fn performance_score(&self) -> f64 {
        self.difficulty.performance_score()
    }

    /// Get progress toward next difficulty level (0.0 to 1.0)
    ///
    /// Shows how close the player is to leveling up.
    pub fn level_progress(&self) -> f64 {
        self.difficulty.level_progress()
    }

    /// Get scenarios completed at current level
    pub fn scenarios_at_level(&self) -> u32 {
        self.difficulty.scenarios_at_level()
    }

    /// Check if current scenario is in Insert mode
    pub fn is_insert_mode(&self) -> bool {
        self.current
            .as_ref()
            .map(|s| s.is_insert_mode())
            .unwrap_or(false)
    }

    /// Get last score breakdown for UI display
    ///
    /// Returns the detailed breakdown from the most recent score calculation.
    /// Useful for showing bonus details in the UI.
    pub fn last_score_breakdown(&self) -> Option<&ScoreBreakdown> {
        self.last_score_breakdown.as_ref()
    }

    /// Get current combo count from score calculator
    pub fn combo_count(&self) -> u32 {
        self.score_calculator.combo_count()
    }

    /// Get best combo achieved this session
    pub fn best_combo(&self) -> u32 {
        self.score_calculator.best_combo()
    }

    /// Take multiplier change event (for UI animations)
    ///
    /// Returns the change event if one occurred since last call.
    pub fn take_multiplier_change(&mut self) -> Option<MultiplierChange> {
        self.stats.take_multiplier_change()
    }

    /// Get grace failures remaining
    pub fn grace_remaining(&self) -> u8 {
        self.stats.grace_remaining()
    }

    /// Record commands from completed scenario for FSRS learning
    ///
    /// Records each command used with timing and success information.
    /// This helps the FSRS system learn which commands need more practice.
    ///
    /// # Arguments
    ///
    /// * `tracker` - Performance tracker to record to
    /// * `success` - Whether the scenario was completed successfully
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::learning::PerformanceTracker;
    ///
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// let mut tracker = PerformanceTracker::new();
    ///
    /// // After scenario completion
    /// session.record_to_fsrs(&mut tracker, true);
    /// ```
    pub fn record_to_fsrs(&self, tracker: &mut crate::learning::PerformanceTracker, success: bool) {
        if let Some(ref scenario) = self.current {
            let elapsed = scenario.elapsed();
            let actions = scenario.actions();
            let action_count = actions.len();

            if action_count == 0 {
                return; // No actions to record
            }

            // Estimate time per command (divide total time by number of actions)
            let time_per_command = elapsed / action_count as u32;

            // Optimal time estimate: time_limit / action_count would be "perfect"
            // We'll use optimal_count from scenario as reference
            let optimal_count = scenario.scenario.scoring.optimal_count.get();
            let optimal_time_per_command = scenario.time_limit / optimal_count as u32;

            // Record each unique command used, normalized to the same card
            // id training mode uses (`"ay` -> `"y`, `:g 3` -> `:goto`) so
            // arcade and training practice of the same skill share one card.
            let mut recorded_commands = std::collections::HashSet::new();
            for command in actions {
                let normalized = crate::helix::commands::normalize_command_id(command).into_owned();
                if recorded_commands.insert(normalized.clone()) {
                    tracker.record_attempt(
                        &normalized,
                        time_per_command,
                        success,
                        optimal_time_per_command,
                    );
                }
            }
        }
    }

    /// Get base points for a scenario based on difficulty
    fn base_points_for(&self, scenario: &ActiveMiniScenario) -> u64 {
        if let Some(ref metadata) = scenario.scenario.metadata
            && let Some(difficulty) = metadata.difficulty
        {
            use crate::config::Difficulty;
            match difficulty {
                Difficulty::Beginner => 100,
                Difficulty::Intermediate => 200,
                Difficulty::Advanced => 350,
            }
        } else {
            // Fallback if no metadata
            100
        }
    }

    /// Load next scenario from queue
    ///
    /// # Errors
    ///
    /// Returns `UserError` if scenario initialization fails or queue is empty.
    fn load_next_scenario(&mut self) -> Result<(), UserError> {
        // Pop from queue
        let scenario = self.queue.pop_front().ok_or(UserError::OperationFailed)?;

        // Get time limit based on mode
        let time_limit = self.get_scenario_time_limit(&scenario);

        // Create active scenario
        self.current = Some(ActiveMiniScenario::new(scenario, time_limit)?);

        // Refill queue
        self.refill_queue();

        Ok(())
    }

    /// Get time limit for a scenario based on current game mode
    ///
    /// - Arcade: Uses DifficultyController's adaptive time limit
    /// - Survival: Uses survival config's level-based time limit
    /// - Challenge: Uses fixed time limit from config
    fn get_scenario_time_limit(&self, scenario: &Scenario) -> Duration {
        match &self.mode {
            MiniGameMode::Arcade(_) => {
                // Use DifficultyController's adaptive time limit
                self.difficulty.time_limit_for(scenario)
            }
            MiniGameMode::Survival(config) => {
                // Use survival-specific time calculation
                config.time_limit_for_level(self.difficulty_level())
            }
            MiniGameMode::Challenge(config) => {
                // Fixed time limit for challenge
                config.time_per_scenario
            }
        }
    }

    /// Refill queue to maintain QUEUE_SIZE
    ///
    /// Uses different selection strategies based on mode:
    /// - Arcade/Survival: FSRS-weighted selection if tracker available
    /// - Challenge: Uses pre-selected scenarios
    fn refill_queue(&mut self) {
        // Challenge mode uses pre-selected scenarios
        if let MiniGameMode::Challenge(config) = &self.mode {
            if let Some(ref selected) = self.selected_scenarios {
                while self.queue.len() < QUEUE_SIZE
                    && self.challenge_scenario_index < config.scenario_count
                    && self.challenge_scenario_index < selected.len()
                {
                    self.queue
                        .push_back(selected[self.challenge_scenario_index].clone());
                    self.challenge_scenario_index += 1;
                }
            }
            return;
        }

        // Arcade and Survival use difficulty controller
        let mut rng = rand::rng();
        while self.queue.len() < QUEUE_SIZE {
            if let Some(scenario) =
                self.difficulty
                    .next_scenario(&self.scenarios, self.tracker.as_ref(), &mut rng)
            {
                self.queue.push_back(scenario);
            } else {
                break; // No more scenarios available
            }
        }
    }

    /// Get reference to the current game mode
    pub fn mode(&self) -> &MiniGameMode {
        &self.mode
    }

    /// Check if game should end based on mode
    ///
    /// - Arcade: Lives depleted (session timer handled externally)
    /// - Survival: Single failure = game over (lives will be 0)
    /// - Challenge: Lives depleted or all scenarios completed
    pub fn check_game_over(&self) -> bool {
        match &self.mode {
            MiniGameMode::Arcade(_) | MiniGameMode::Survival(_) => {
                // Lives depleted
                self.stats.is_game_over()
            }
            MiniGameMode::Challenge(config) => {
                // Lives depleted or all scenarios completed
                self.stats.is_game_over()
                    || self.stats.scenarios_completed >= config.scenario_count as u32
            }
        }
    }

    /// Check if Challenge mode is complete (all scenarios done)
    pub fn is_challenge_complete(&self) -> bool {
        if let MiniGameMode::Challenge(config) = &self.mode {
            self.stats.scenarios_completed >= config.scenario_count as u32
        } else {
            false
        }
    }

    /// Get scenarios completed count for Challenge mode progress display
    pub fn challenge_scenarios_completed(&self) -> Option<(u32, usize)> {
        if let MiniGameMode::Challenge(config) = &self.mode {
            Some((self.stats.scenarios_completed, config.scenario_count))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Difficulty;
    use crate::game::PlayableScenario;
    use crate::minigame::{ArcadeConfig, ChallengeConfig, SurvivalConfig};
    use crate::testing::ScenarioBuilder;

    fn create_test_scenario(id: &str, difficulty: Difficulty) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .difficulty(difficulty)
            .build()
    }

    #[test]
    fn test_new_session() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
            create_test_scenario("s3", Difficulty::Beginner),
        ]);

        let session = MiniGameSession::new(scenarios, None);

        assert_eq!(session.stats.lives(), 3);
        assert_eq!(session.stats.score, 0);
        assert!(session.state.is_countdown());
        assert_eq!(session.queue.len(), QUEUE_SIZE);
    }

    /// Regression test for #323: `try_begin_game_over` must only return `true`
    /// once per session, regardless of how many independent call sites ask.
    #[test]
    fn test_try_begin_game_over_is_idempotent() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);

        assert!(session.try_begin_game_over());
        assert!(!session.try_begin_game_over());
        assert!(!session.try_begin_game_over());
    }

    #[test]
    fn test_countdown_progression() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);

        session.start();
        assert_eq!(session.state.countdown_remaining(), Some(3));

        session.tick_countdown();
        assert_eq!(session.state.countdown_remaining(), Some(2));

        session.tick_countdown();
        assert_eq!(session.state.countdown_remaining(), Some(1));

        session.tick_countdown();
        assert!(session.state.is_playing());
    }

    #[test]
    fn test_timeout_loses_life() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.state = MiniGameState::Playing;
        let _ = session.load_next_scenario();

        assert_eq!(session.stats.lives(), 3);

        session.handle_timeout();
        assert_eq!(session.stats.lives(), 2);

        session.handle_timeout();
        assert_eq!(session.stats.lives(), 1);

        session.handle_timeout();
        assert_eq!(session.stats.lives(), 0);
        assert!(session.state.is_game_over());
    }

    #[test]
    fn test_pause_resume() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.state = MiniGameState::Playing;

        session.pause();
        assert!(session.state.is_paused());

        session.resume();
        assert!(session.state.is_playing());
    }

    #[test]
    fn test_active_scenario_pause_freezes_elapsed_time() {
        let scenario = create_test_scenario("s1", Difficulty::Beginner);
        let mut active = ActiveMiniScenario::new(scenario, Duration::from_secs(60)).unwrap();

        let elapsed_before_pause = active.elapsed();
        active.pause();

        std::thread::sleep(Duration::from_millis(50));
        let elapsed_while_paused = active.elapsed();

        // Elapsed time must not advance while paused, even though real time passed.
        assert!(
            elapsed_while_paused < elapsed_before_pause + Duration::from_millis(20),
            "elapsed grew while paused: before={elapsed_before_pause:?} during={elapsed_while_paused:?}"
        );

        active.resume();
        std::thread::sleep(Duration::from_millis(30));
        let elapsed_after_resume = active.elapsed();

        // Once resumed, elapsed time should advance again.
        assert!(
            elapsed_after_resume > elapsed_while_paused,
            "elapsed did not advance after resume: during={elapsed_while_paused:?} after={elapsed_after_resume:?}"
        );
    }

    #[test]
    fn test_active_scenario_repeated_pause_resume_accumulates_total_paused() {
        let scenario = create_test_scenario("s1", Difficulty::Beginner);
        let mut active = ActiveMiniScenario::new(scenario, Duration::from_secs(60)).unwrap();

        // First pause/resume cycle.
        active.pause();
        std::thread::sleep(Duration::from_millis(30));
        active.resume();
        let total_paused_after_first = active.total_paused;
        assert!(total_paused_after_first >= Duration::from_millis(20));

        // Second pause/resume cycle should add to the accumulated total, not reset it.
        active.pause();
        std::thread::sleep(Duration::from_millis(30));
        active.resume();
        let total_paused_after_second = active.total_paused;

        assert!(
            total_paused_after_second > total_paused_after_first,
            "second pause cycle did not accumulate: first={total_paused_after_first:?} second={total_paused_after_second:?}"
        );
    }

    #[test]
    fn test_active_scenario_pause_resume_idempotent() {
        let scenario = create_test_scenario("s1", Difficulty::Beginner);
        let mut active = ActiveMiniScenario::new(scenario, Duration::from_secs(60)).unwrap();

        // Resuming without a prior pause is a no-op.
        active.resume();
        assert_eq!(active.total_paused, Duration::ZERO);
        assert!(active.paused_at.is_none());

        // Pausing twice in a row should not reset the paused_at anchor.
        active.pause();
        let paused_at_first = active.paused_at;
        std::thread::sleep(Duration::from_millis(10));
        active.pause();
        assert_eq!(active.paused_at, paused_at_first);

        active.resume();
        assert!(active.total_paused >= Duration::from_millis(5));
    }

    #[test]
    fn test_minigame_session_pause_resume_freezes_scenario_elapsed() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.state = MiniGameState::Playing;
        let _ = session.load_next_scenario();

        session.pause();
        assert!(session.state.is_paused());

        let elapsed_at_pause = session
            .current_scenario()
            .expect("scenario should be active")
            .elapsed();
        std::thread::sleep(Duration::from_millis(40));

        let elapsed_while_paused = session
            .current_scenario()
            .expect("scenario should still be active")
            .elapsed();
        assert!(
            elapsed_while_paused < elapsed_at_pause + Duration::from_millis(20),
            "session-level pause did not freeze scenario elapsed time"
        );

        session.resume();
        assert!(session.state.is_playing());
    }

    #[test]
    fn test_stats_increment_on_advance() {
        let mut stats = MiniGameStats::new();

        assert_eq!(stats.scenarios_completed, 0);
        assert_eq!(stats.streak(), 0);

        stats.record_completion();

        assert_eq!(stats.scenarios_completed, 1);
        assert_eq!(stats.streak(), 1);
    }

    #[test]
    fn test_timeout_resets_streak() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);
        let mut session = MiniGameSession::new(scenarios, None);

        // Start and countdown
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();

        // Build up a real streak
        for _ in 0..5 {
            session.stats.record_completion();
        }
        session.handle_timeout();

        assert_eq!(session.stats().streak(), 0);
    }

    #[test]
    fn test_queue_refills() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
            create_test_scenario("s3", Difficulty::Beginner),
            create_test_scenario("s4", Difficulty::Beginner),
            create_test_scenario("s5", Difficulty::Beginner),
        ]);

        let mut session = MiniGameSession::new(scenarios, None);
        assert_eq!(session.queue().len(), QUEUE_SIZE);

        // Start and countdown
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();

        // After loading one, queue should still have enough scenarios
        assert!(session.queue().len() >= 2);
    }

    #[test]
    fn test_handle_command_ge() {
        // Test that 'ge' command works in minigame session
        // This is a regression test for the 'ge' command error
        let scenario = ScenarioBuilder::new()
            .id("test_ge")
            .name("Test ge command")
            .description("Test goto last line")
            .setup_content("line 1\nline 2\nline 3\n")
            .target_content("line 1\nline 2\nline 3\n")
            .target_cursor(2, 0)
            .commands(vec!["ge"])
            .command_description("Go to last line")
            .optimal_count(1)
            .difficulty(Difficulty::Beginner)
            .build();

        let scenarios = Arc::new(vec![scenario]);
        let mut session = MiniGameSession::new(scenarios, None);

        // Start and countdown
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(
            session.state.is_playing(),
            "Should be playing after countdown"
        );

        // Execute 'ge' command
        assert!(
            session.current_scenario().is_some(),
            "Current scenario should be set"
        );

        let result = session.handle_command("ge");
        assert!(result.is_ok(), "ge command should succeed: {:?}", result);

        // Check that the cursor moved
        if let Some(scenario) = session.current_scenario() {
            let (row, _col) = scenario.current_cursor();
            assert_eq!(row, 2, "Cursor should be on last line (row 2)");
        }
    }

    /// Regression test: `record_to_fsrs` is the one normalization call site
    /// the original plan missed (per the critic's M2/M3 finding). Without
    /// normalizing before dedup+record, `"ay` and `"by` would mint two
    /// separate FSRS cards instead of collapsing to one `"y` card.
    #[test]
    fn test_record_to_fsrs_normalizes_register_ops_to_one_card() {
        let scenario = ScenarioBuilder::new()
            .id("test_register_fsrs")
            .setup_content("alpha beta")
            .target_content("zzz")
            .optimal_count(1)
            .difficulty(Difficulty::Beginner)
            .build();

        let scenarios = Arc::new(vec![scenario]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(session.state.is_playing());

        session.handle_command("\"ay").unwrap();
        session.handle_command("\"by").unwrap();

        let mut tracker = PerformanceTracker::new();
        session.record_to_fsrs(&mut tracker, true);

        assert!(
            tracker.get_performance("\"y").is_some(),
            "expected the normalized '\"y' card to exist"
        );
        assert!(
            tracker.get_performance("\"ay").is_none(),
            "raw '\"ay' must not exist as its own card"
        );
        assert!(
            tracker.get_performance("\"by").is_none(),
            "raw '\"by' must not exist as its own card"
        );
    }

    #[test]
    fn test_survival_mode_single_life() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);

        let mode = MiniGameMode::Survival(SurvivalConfig::default());
        let session = MiniGameSession::with_mode(scenarios, None, mode);

        assert_eq!(session.stats().lives(), 1);
        assert!(session.mode().is_survival());
    }

    #[test]
    fn test_survival_mode_game_over_on_first_failure() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);

        let mode = MiniGameMode::Survival(SurvivalConfig::default());
        let mut session = MiniGameSession::with_mode(scenarios, None, mode);

        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();

        // Single timeout should end the game
        session.handle_timeout();

        assert_eq!(session.stats().lives(), 0);
        assert!(session.state().is_game_over());
    }

    #[test]
    fn test_challenge_mode_starting_lives() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);

        let mode =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));
        let session = MiniGameSession::with_mode(scenarios, None, mode);

        assert_eq!(session.stats().lives(), 3);
        assert!(session.mode().is_challenge());
    }

    #[test]
    fn test_challenge_mode_scenario_progress() {
        let scenarios: Vec<Scenario> = (0..20)
            .map(|i| create_test_scenario(&format!("s{}", i), Difficulty::Beginner))
            .collect();

        let mode =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));
        let session = MiniGameSession::with_mode(Arc::new(scenarios), None, mode);

        let progress = session.challenge_scenarios_completed();
        assert!(progress.is_some());
        let (completed, total) = progress.unwrap();
        assert_eq!(completed, 0);
        assert_eq!(total, 10); // CHALLENGE_SCENARIO_COUNT
    }

    #[test]
    fn test_arcade_mode_defaults() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);

        // Default constructor uses Arcade mode
        let session = MiniGameSession::new(scenarios, None);

        assert_eq!(session.stats().lives(), 3);
        assert!(session.mode().is_arcade());
        assert!(session.mode().has_session_timer());
    }

    #[test]
    fn test_mode_has_session_timer() {
        let arcade = MiniGameMode::Arcade(ArcadeConfig::default());
        let survival = MiniGameMode::Survival(SurvivalConfig::default());
        let challenge =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));

        assert!(arcade.has_session_timer());
        assert!(!survival.has_session_timer());
        assert!(!challenge.has_session_timer());
    }

    /// Helper for #327 tests: an Arcade session with a very short session
    /// duration so tests can wait it out with a real (short) sleep rather
    /// than mocking `Instant`.
    fn short_arcade_session(session_duration: Duration) -> MiniGameSession {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mode = MiniGameMode::Arcade(ArcadeConfig {
            session_duration,
            ..ArcadeConfig::default()
        });
        let mut session = MiniGameSession::with_mode(scenarios, None, mode);
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(session.state().is_playing());
        session
    }

    #[test]
    fn test_session_expires_after_duration_elapses() {
        // A generous limit (well above typical session-construction time) and an
        // even more generous sleep multiple keep this robust on loaded CI runners.
        let session = short_arcade_session(Duration::from_millis(200));

        assert!(!session.is_session_expired());
        std::thread::sleep(Duration::from_millis(500));
        assert!(session.is_session_expired());
    }

    #[test]
    fn test_end_session_on_timeout_transitions_to_game_over_without_consuming_life() {
        let mut session = short_arcade_session(Duration::from_millis(200));
        let lives_before = session.stats().lives();
        std::thread::sleep(Duration::from_millis(500));
        assert!(session.is_session_expired());

        session.end_session_on_timeout();

        assert!(session.state().is_game_over());
        assert_eq!(
            session.stats().lives(),
            lives_before,
            "session-timeout must not consume a life, unlike a per-scenario timeout"
        );
    }

    #[test]
    fn test_session_pause_freezes_session_elapsed_time() {
        let mut session = short_arcade_session(Duration::from_secs(60));

        session.pause();
        assert!(session.state().is_paused());
        assert!(
            !session.is_session_expired(),
            "sanity: 60s session should not already be expired"
        );

        let elapsed_at_pause = session.session_elapsed();
        std::thread::sleep(Duration::from_millis(50));
        let elapsed_while_paused = session.session_elapsed();

        // Wide margin: the paused-duration subtraction cancels wall-clock drift
        // exactly (mathematically constant while paused, mod the sub-millisecond
        // gap between `pause()` and reading `elapsed_at_pause`), so this is not a
        // tight race against the sleep above — 200ms leaves ample headroom on a
        // loaded CI runner.
        assert!(
            elapsed_while_paused < elapsed_at_pause + Duration::from_millis(200),
            "session-level elapsed time advanced while paused: before={elapsed_at_pause:?} during={elapsed_while_paused:?}"
        );

        session.resume();
        assert!(session.state().is_playing());
        std::thread::sleep(Duration::from_millis(50));
        assert!(
            session.session_elapsed() > elapsed_while_paused,
            "session-level elapsed time did not resume advancing after resume"
        );
    }

    #[test]
    fn test_modes_without_session_timer_never_expire() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);

        let survival_mode = MiniGameMode::Survival(SurvivalConfig::default());
        let mut survival = MiniGameSession::with_mode(scenarios.clone(), None, survival_mode);
        survival.start();
        survival.tick_countdown();
        survival.tick_countdown();
        survival.tick_countdown();

        let challenge_mode =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));
        let mut challenge = MiniGameSession::with_mode(scenarios, None, challenge_mode);
        challenge.start();
        challenge.tick_countdown();
        challenge.tick_countdown();
        challenge.tick_countdown();

        std::thread::sleep(Duration::from_millis(30));

        assert!(!survival.mode().has_session_timer());
        assert!(!survival.is_session_expired());
        assert!(!challenge.mode().has_session_timer());
        assert!(!challenge.is_session_expired());
    }

    #[test]
    fn test_check_game_over_arcade() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);

        // Not game over with lives
        assert!(!session.check_game_over());

        // Game over when lives depleted
        while session.stats.lose_life() {}
        assert!(session.check_game_over());
    }

    #[test]
    fn test_check_game_over_challenge() {
        let scenarios: Vec<Scenario> = (0..20)
            .map(|i| create_test_scenario(&format!("s{}", i), Difficulty::Beginner))
            .collect();

        let mode =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));
        let mut session = MiniGameSession::with_mode(Arc::new(scenarios), None, mode);

        // Not game over initially
        assert!(!session.check_game_over());

        // Game over when all scenarios completed
        session.stats.scenarios_completed = 10;
        assert!(session.check_game_over());
        assert!(session.is_challenge_complete());

        // Also game over when lives depleted
        session.stats.scenarios_completed = 5;
        while session.stats.lose_life() {}
        assert!(session.check_game_over());
        assert!(!session.is_challenge_complete());
    }

    // CR-012: Test Survival mode time limit integration
    #[test]
    fn test_survival_mode_time_limit_integration() {
        use crate::constants::SURVIVAL_BASE_TIME;
        use std::time::Duration;

        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);

        let config = SurvivalConfig::default();
        let mode = MiniGameMode::Survival(config.clone());
        let mut session = MiniGameSession::with_mode(scenarios, None, mode);

        // Start session and complete countdown
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(session.state().is_playing());

        // Verify time limit matches SurvivalConfig at level 1
        if let Some(scenario) = session.current_scenario() {
            let remaining = scenario.remaining_time();
            // At level 1, time should be base_time (within margin due to elapsed time)
            assert!(
                remaining <= SURVIVAL_BASE_TIME,
                "Remaining time {:?} should be <= base time {:?}",
                remaining,
                SURVIVAL_BASE_TIME
            );
        }

        // Verify config calculation directly
        let level1_time = config.time_limit_for_level(1);
        assert_eq!(level1_time, SURVIVAL_BASE_TIME);

        let level5_time = config.time_limit_for_level(5);
        // Level 5 should have less time than level 1 (500ms * 4 levels = 2s less)
        let expected_decrease = Duration::from_millis(500 * 4);
        assert_eq!(level5_time, SURVIVAL_BASE_TIME - expected_decrease);
    }
}
