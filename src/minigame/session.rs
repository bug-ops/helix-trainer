//! Mini-game session management
//!
//! Manages the complete mini-game session including scenario queue,
//! active scenario, timing, and score calculation.

use crate::config::Scenario;
use crate::game::EditorState;
use crate::helix::AnyModeSimulator;
use crate::minigame::{DifficultyController, MiniGameState, MiniGameStats};
use crate::security::UserError;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Size of scenario queue (number of upcoming scenarios visible)
const QUEUE_SIZE: usize = 3;

/// Active scenario being played with timer
///
/// Represents the scenario currently being solved by the player,
/// including the simulator state, timing information, and action history.
pub struct ActiveMiniScenario {
    /// The scenario being played
    pub scenario: Scenario,

    /// Helix simulator instance
    simulator: AnyModeSimulator,

    /// Current editor state
    current_state: EditorState,

    /// Target state to achieve
    target_state: EditorState,

    /// When scenario started
    started_at: Instant,

    /// Time limit for this scenario
    time_limit: Duration,

    /// Actions taken so far
    actions: Vec<String>,
}

impl ActiveMiniScenario {
    /// Create a new active scenario
    ///
    /// # Errors
    ///
    /// Returns `UserError` if scenario setup or target state is invalid.
    fn new(scenario: Scenario, time_limit: Duration) -> Result<Self, UserError> {
        // Create initial state from scenario setup
        let initial_state = EditorState::from_setup(
            &scenario.setup.file_content,
            [
                scenario.setup.cursor_position.0,
                scenario.setup.cursor_position.1,
            ],
        )
        .map_err(|_| UserError::ScenarioTooComplex)?;

        // Create target state
        let target_state = EditorState::from_target(
            &scenario.target.file_content,
            [
                scenario.target.cursor_position.0,
                scenario.target.cursor_position.1,
            ],
            scenario.target.selection,
        )
        .map_err(|_| UserError::ScenarioTooComplex)?;

        // Initialize simulator from initial state
        let simulator = AnyModeSimulator::from_editor_state(&initial_state);
        let current_state = initial_state.clone();

        Ok(Self {
            scenario,
            simulator,
            current_state,
            target_state,
            started_at: Instant::now(),
            time_limit,
            actions: Vec::new(),
        })
    }

    /// Execute a command through the simulator
    ///
    /// # Errors
    ///
    /// Returns `UserError` if command execution fails.
    fn execute_command(&mut self, command: &str) -> Result<(), UserError> {
        self.simulator.execute_command(command)?;
        self.current_state = self.simulator.to_editor_state()?;
        self.actions.push(command.to_string());
        Ok(())
    }

    /// Check if scenario is completed
    fn is_completed(&self) -> bool {
        self.current_state.matches(&self.target_state)
    }

    /// Get elapsed time since scenario started
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get remaining time before timeout
    pub fn remaining_time(&self) -> Duration {
        self.time_limit.saturating_sub(self.elapsed())
    }

    /// Check if scenario has timed out
    pub fn is_timed_out(&self) -> bool {
        self.elapsed() >= self.time_limit
    }

    /// Get progress percentage (0.0 to 1.0)
    pub fn progress_percent(&self) -> f64 {
        self.elapsed().as_secs_f64() / self.time_limit.as_secs_f64()
    }

    /// Get current editor state
    pub fn current_state(&self) -> &EditorState {
        &self.current_state
    }

    /// Get target editor state
    pub fn target_state(&self) -> &EditorState {
        &self.target_state
    }

    /// Get number of actions taken
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    /// Get reference to actions taken
    pub fn actions(&self) -> &[String] {
        &self.actions
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
pub struct MiniGameSession {
    /// Current active scenario being played
    current: Option<ActiveMiniScenario>,

    /// Queue of upcoming scenarios (2-3 visible)
    queue: VecDeque<Scenario>,

    /// Game statistics
    pub(crate) stats: MiniGameStats,

    /// Difficulty controller
    difficulty: DifficultyController,

    /// Current game state
    state: MiniGameState,

    /// All available scenarios (reference)
    scenarios: Arc<Vec<Scenario>>,
}

impl MiniGameSession {
    /// Create a new mini-game session with scenario collection
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(vec![/* scenarios */]);
    /// let session = MiniGameSession::new(scenarios);
    /// ```
    pub fn new(scenarios: Arc<Vec<Scenario>>) -> Self {
        let mut session = Self {
            current: None,
            queue: VecDeque::with_capacity(QUEUE_SIZE),
            stats: MiniGameStats::new(),
            difficulty: DifficultyController::new(),
            state: MiniGameState::default(),
            scenarios,
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
    /// let mut session = MiniGameSession::new(scenarios);
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
    /// let mut session = MiniGameSession::new(scenarios);
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
    /// let mut session = MiniGameSession::new(scenarios);
    /// session.start();
    /// session.handle_command("dd")?;
    /// # Ok::<(), helix_trainer::security::UserError>(())
    /// ```
    pub fn handle_command(&mut self, command: &str) -> Result<(), UserError> {
        if !self.state.is_playing() {
            return Ok(()); // Ignore commands in non-playing states
        }

        let current = self.current.as_mut().ok_or(UserError::OperationFailed)?;

        current.execute_command(command)?;

        Ok(())
    }

    /// Check if current scenario is complete
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios);
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
    /// let mut session = MiniGameSession::new(scenarios);
    /// session.advance_to_next();
    /// ```
    pub fn advance_to_next(&mut self) {
        if let Some(ref scenario) = self.current {
            // Calculate and award points
            let points = self.calculate_points(scenario);
            self.stats.add_score(points);

            // Update statistics
            self.stats.increase_streak();
            self.stats.record_completion();

            // Update difficulty based on success
            self.difficulty.update_after_result(true);

            // Check for extra life milestone (every 5000 points)
            let prev_milestone = (self.stats.score - points) / 5000;
            let curr_milestone = self.stats.score / 5000;
            if curr_milestone > prev_milestone {
                self.stats.gain_life();
            }
        }

        // Transition state
        self.state = MiniGameState::Transition;
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
            self.load_next_scenario()?;
        }
        Ok(())
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
    /// let mut session = MiniGameSession::new(scenarios);
    /// session.handle_timeout();
    /// ```
    pub fn handle_timeout(&mut self) {
        // Lose life
        let has_lives = self.stats.lose_life();

        // Reset streak
        self.stats.reset_streak();

        // Update statistics
        self.stats.record_failure();

        // Update difficulty based on failure
        self.difficulty.update_after_result(false);

        if has_lives {
            // Continue to next scenario
            self.state = MiniGameState::Transition;
        } else {
            // Game over
            self.state = MiniGameState::GameOver;
            self.current = None;
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
    /// let session = MiniGameSession::new(scenarios);
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

    /// Pause the game
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios);
    /// session.pause();
    /// assert!(session.state().is_paused());
    /// ```
    pub fn pause(&mut self) {
        if self.state.is_playing() {
            self.state = MiniGameState::Paused;
        }
    }

    /// Resume the game from paused state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameSession;
    ///
    /// let mut session = MiniGameSession::new(scenarios);
    /// session.pause();
    /// session.resume();
    /// assert!(session.state().is_playing());
    /// ```
    pub fn resume(&mut self) {
        if self.state.is_paused() {
            self.state = MiniGameState::Playing;
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

    /// Get difficulty level
    pub fn difficulty_level(&self) -> u8 {
        self.difficulty.current_level()
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
    /// let mut session = MiniGameSession::new(scenarios);
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
            let optimal_count = scenario.scenario.scoring.optimal_count.max(1);
            let optimal_time_per_command = scenario.time_limit / optimal_count as u32;

            // Record each unique command used
            let mut recorded_commands = std::collections::HashSet::new();
            for command in actions {
                if recorded_commands.insert(command.clone()) {
                    tracker.record_attempt(
                        command,
                        time_per_command,
                        success,
                        optimal_time_per_command,
                    );
                }
            }
        }
    }

    /// Calculate points for completing a scenario
    ///
    /// Point calculation:
    /// - Base points from scenario difficulty
    /// - Time bonus: faster completion = more points (up to 50% bonus)
    /// - Efficiency bonus: optimal actions = 25% bonus
    /// - Multiplier applied based on streak
    fn calculate_points(&self, scenario: &ActiveMiniScenario) -> u64 {
        let base_points = self.base_points_for(scenario);

        // Time bonus: faster = more points (max 50% bonus)
        let time_ratio = 1.0 - scenario.progress_percent();
        let time_bonus = (base_points as f64 * time_ratio * 0.5) as u64;

        // Efficiency bonus: optimal actions = 25% bonus
        let optimal = scenario.scenario.scoring.optimal_count;
        let actual = scenario.action_count();
        let efficiency_bonus = if actual <= optimal {
            base_points / 4
        } else {
            0
        };

        // Total before multiplier and return
        base_points + time_bonus + efficiency_bonus
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

        // Get time limit from difficulty controller
        let time_limit = self.difficulty.time_limit_for(&scenario);

        // Create active scenario
        self.current = Some(ActiveMiniScenario::new(scenario, time_limit)?);

        // Refill queue
        self.refill_queue();

        Ok(())
    }

    /// Refill queue to maintain QUEUE_SIZE
    fn refill_queue(&mut self) {
        while self.queue.len() < QUEUE_SIZE {
            if let Some(scenario) = self.difficulty.next_scenario(&self.scenarios) {
                self.queue.push_back(scenario);
            } else {
                break; // No more scenarios available
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Difficulty, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
    };

    fn create_test_scenario(id: &str, difficulty: Difficulty) -> Scenario {
        Scenario {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\n".to_string(),
                cursor_position: (1, 0),
            },
            target: TargetState {
                file_content: "line 1\n".to_string(),
                cursor_position: (1, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Delete line".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
            metadata: Some(ScenarioMetadata {
                difficulty: Some(difficulty),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn test_new_session() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
            create_test_scenario("s3", Difficulty::Beginner),
        ]);

        let session = MiniGameSession::new(scenarios);

        assert_eq!(session.stats.lives, 3);
        assert_eq!(session.stats.score, 0);
        assert!(session.state.is_countdown());
        assert_eq!(session.queue.len(), QUEUE_SIZE);
    }

    #[test]
    fn test_countdown_progression() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios);

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
        let mut session = MiniGameSession::new(scenarios);
        session.state = MiniGameState::Playing;
        let _ = session.load_next_scenario();

        assert_eq!(session.stats.lives, 3);

        session.handle_timeout();
        assert_eq!(session.stats.lives, 2);

        session.handle_timeout();
        assert_eq!(session.stats.lives, 1);

        session.handle_timeout();
        assert_eq!(session.stats.lives, 0);
        assert!(session.state.is_game_over());
    }

    #[test]
    fn test_pause_resume() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios);
        session.state = MiniGameState::Playing;

        session.pause();
        assert!(session.state.is_paused());

        session.resume();
        assert!(session.state.is_playing());
    }

    #[test]
    fn test_stats_increment_on_advance() {
        let mut stats = MiniGameStats::new();

        assert_eq!(stats.scenarios_completed, 0);
        assert_eq!(stats.streak, 0);

        stats.record_completion();
        stats.increase_streak();

        assert_eq!(stats.scenarios_completed, 1);
        assert_eq!(stats.streak, 1);
    }

    #[test]
    fn test_timeout_resets_streak() {
        let scenarios = Arc::new(vec![
            create_test_scenario("s1", Difficulty::Beginner),
            create_test_scenario("s2", Difficulty::Beginner),
        ]);
        let mut session = MiniGameSession::new(scenarios);

        // Start and countdown
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();

        // Manually set streak high
        session.stats.streak = 5;
        session.handle_timeout();

        assert_eq!(session.stats().streak, 0);
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

        let mut session = MiniGameSession::new(scenarios);
        assert_eq!(session.queue().len(), QUEUE_SIZE);

        // Start and countdown
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();

        // After loading one, queue should still have enough scenarios
        assert!(session.queue().len() >= 2);
    }
}
