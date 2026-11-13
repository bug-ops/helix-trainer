//! Mini-game session management
//!
//! Manages the complete mini-game session including scenario queue,
//! active scenario, timing, and score calculation.

use crate::config::{Difficulty, Scenario};
use crate::constants::EXTRA_LIFE_SCORE_MILESTONE;
use crate::game::GameError;
use crate::game::{LiveScenario, UserAction};
use crate::learning::PerformanceTracker;
use crate::minigame::{
    DifficultyController, DifficultyLevel, LevelChange, MiniGameMode, MiniGameState, MiniGameStats,
    MultiplierChange, PerformancePoint, Ratio, ScoreBreakdown, ScoreCalculator,
    select_challenge_scenarios,
};
use crate::time::{MonotonicClock, SystemMonotonicClock};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Size of scenario queue (number of upcoming scenarios visible)
const QUEUE_SIZE: usize = 3;

/// Stopwatch that excludes paused spans; the one pause-accounting implementation
/// shared by per-scenario and session-level timers.
#[derive(Debug, Clone, Copy)]
struct PausableClock {
    started_at: Instant,
    paused_at: Option<Instant>,
    total_paused: Duration,
}

impl PausableClock {
    fn start(now: Instant) -> Self {
        Self {
            started_at: now,
            paused_at: None,
            total_paused: Duration::ZERO,
        }
    }

    /// Idempotent: pausing while already paused has no effect.
    fn pause(&mut self, now: Instant) {
        self.paused_at.get_or_insert(now);
    }

    /// Idempotent: resuming while not paused has no effect.
    fn resume(&mut self, now: Instant) {
        if let Some(paused_at) = self.paused_at.take() {
            self.total_paused += now.saturating_duration_since(paused_at);
        }
    }

    fn elapsed(&self, now: Instant) -> Duration {
        let open_span = self
            .paused_at
            .map_or(Duration::ZERO, |p| now.saturating_duration_since(p));
        now.saturating_duration_since(self.started_at)
            .saturating_sub(self.total_paused + open_span)
    }
}

/// Internal state machine; each phase owns the data only it needs.
/// [`MiniGameState`] is the public, data-free projection.
#[derive(Debug, Clone, Copy)]
enum Phase {
    Countdown { remaining: u8 },
    Playing,
    Paused,
    Transition { success: bool, started_at: Instant },
    GameOver,
}

impl Phase {
    fn state(self) -> MiniGameState {
        match self {
            Self::Countdown { remaining } => MiniGameState::Countdown { remaining },
            Self::Playing => MiniGameState::Playing,
            Self::Paused => MiniGameState::Paused,
            Self::Transition { success, .. } => MiniGameState::Transition { success },
            Self::GameOver => MiniGameState::GameOver,
        }
    }
}

/// Where the next queued scenarios come from.
enum Lineup {
    /// Difficulty-controller selection (Arcade and Survival).
    Adaptive,
    /// Pre-selected seeded list (Challenge): `next` indexes `scenarios`, `limit` caps the run.
    Challenge {
        scenarios: Vec<Arc<Scenario>>,
        next: usize,
        limit: usize,
    },
}

/// Active scenario being played with timer
///
/// Represents the scenario currently being solved by the player,
/// including the simulator state, timing information, and action history.
pub struct ActiveMiniScenario {
    /// The scenario in play: simulator, target and the actions taken
    live: LiveScenario,

    /// Scenario stopwatch (excludes paused spans)
    timer: PausableClock,

    /// Source of the instants fed to `timer`
    clock: Arc<dyn MonotonicClock>,

    /// Time limit for this scenario
    time_limit: Duration,
}

impl ActiveMiniScenario {
    /// Create a new active scenario
    fn new(scenario: Arc<Scenario>, time_limit: Duration, clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            live: LiveScenario::new(scenario),
            timer: PausableClock::start(clock.now()),
            clock,
            time_limit,
        }
    }

    /// The scenario being played
    pub fn scenario(&self) -> &Scenario {
        self.live.scenario()
    }

    /// Check if scenario is completed
    fn is_completed(&self) -> bool {
        self.live.is_completed()
    }

    /// Freeze the countdown timer
    ///
    /// Idempotent: calling this while already paused has no effect.
    pub fn pause(&mut self) {
        self.timer.pause(self.clock.now());
    }

    /// Resume the countdown timer, accumulating the just-finished pause span
    ///
    /// Idempotent: calling this while not paused has no effect.
    pub fn resume(&mut self) {
        self.timer.resume(self.clock.now());
    }

    /// Get elapsed time since scenario started, excluding any paused duration
    pub fn elapsed(&self) -> Duration {
        self.timer.elapsed(self.clock.now())
    }

    /// Get remaining time before timeout
    pub fn remaining_time(&self) -> Duration {
        self.time_limit.saturating_sub(self.elapsed())
    }

    /// Check if scenario has timed out
    pub fn is_timed_out(&self) -> bool {
        self.elapsed() >= self.time_limit
    }

    /// Fraction of the time limit used, clamped to `0.0..=1.0` even after expiry.
    pub fn time_used(&self) -> Ratio {
        Ratio::new(self.elapsed().as_secs_f64() / self.time_limit.as_secs_f64())
    }

    /// Get number of actions taken
    pub fn action_count(&self) -> usize {
        self.live.action_count()
    }

    /// Get reference to actions taken
    pub fn actions(&self) -> &[UserAction] {
        self.live.actions()
    }

    /// Check if currently in Insert mode
    pub fn is_insert_mode(&self) -> bool {
        self.live.is_insert_mode()
    }

    /// Check if a `Q`/`q` macro is currently being recorded
    pub fn is_recording_macro(&self) -> bool {
        self.live.is_recording_macro()
    }

    /// Run one command string and record it as one action.
    fn apply_text(&mut self, command: &str) -> Result<(), GameError> {
        let at = self.elapsed();
        self.live.apply_text(command, at)
    }

    /// Run one parsed Normal-mode command and record it as one action.
    fn apply_command(
        &mut self,
        command: &crate::helix::normal_command::NormalCommand,
    ) -> Result<(), GameError> {
        let at = self.elapsed();
        self.live.apply_command(command, at)
    }

    /// Run one typed Insert-mode input and record it as one action.
    fn apply_insert(&mut self, input: &crate::helix::InsertInput) -> Result<(), GameError> {
        let at = self.elapsed();
        self.live.apply_insert(input, at)
    }
}

impl crate::game::PlayableScenario for ActiveMiniScenario {
    fn live(&self) -> &LiveScenario {
        &self.live
    }

    fn elapsed(&self) -> Duration {
        ActiveMiniScenario::elapsed(self)
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
pub struct MiniGameSession<R: Rng = StdRng> {
    /// Current active scenario being played
    current: Option<ActiveMiniScenario>,

    /// Queue of upcoming scenarios (2-3 visible)
    queue: VecDeque<Arc<Scenario>>,

    /// Game statistics
    pub(crate) stats: MiniGameStats,

    /// Difficulty controller
    difficulty: DifficultyController,

    /// Score calculator with combo tracking
    score_calculator: ScoreCalculator,

    /// Last score breakdown (for UI display)
    last_score_breakdown: Option<ScoreBreakdown>,

    /// Current phase of the state machine
    phase: Phase,

    /// All available scenarios (reference)
    scenarios: Arc<Vec<Scenario>>,

    /// Session-level stopwatch; `None` until gameplay begins after the countdown
    session_timer: Option<PausableClock>,

    /// Performance tracker for FSRS-based scenario selection (read-only clone)
    ///
    /// When present, scenarios with commands needing practice are prioritized.
    /// This is a snapshot of the tracker at session creation time.
    tracker: Option<PerformanceTracker>,

    /// Game mode configuration
    mode: MiniGameMode,

    /// Source of upcoming scenarios, fixed by the mode
    lineup: Lineup,

    /// Source of every instant the session reads
    clock: Arc<dyn MonotonicClock>,

    /// Randomness for adaptive scenario selection
    rng: R,

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
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::minigame::MiniGameSession;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let tracker = PerformanceTracker::new();
    ///
    /// // With FSRS weighting
    /// let weighted = MiniGameSession::new(scenarios.clone(), Some(tracker));
    ///
    /// // Without FSRS weighting
    /// let uniform = MiniGameSession::new(scenarios, None);
    /// # Ok::<(), helix_trainer::error::UserError>(())
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
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::minigame::{MiniGameMode, MiniGameSession, SurvivalConfig};
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    ///
    /// let mode = MiniGameMode::Survival(SurvivalConfig::default());
    /// let session = MiniGameSession::with_mode(scenarios, None, mode);
    /// assert_eq!(session.stats().lives(), 1); // Survival mode has 1 life
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn with_mode(
        scenarios: Arc<Vec<Scenario>>,
        tracker: Option<PerformanceTracker>,
        mode: MiniGameMode,
    ) -> Self {
        Self::with_sources(
            scenarios,
            tracker,
            mode,
            Arc::new(SystemMonotonicClock),
            StdRng::from_rng(&mut rand::rng()),
        )
    }
}

impl<R: Rng> MiniGameSession<R> {
    /// Create a session with explicit time and randomness sources.
    ///
    /// [`MiniGameSession::new`] and [`MiniGameSession::with_mode`] use the system
    /// clock and an OS-seeded RNG; tests inject a `FakeMonotonicClock` (see
    /// [`crate::time`]) and a seeded RNG so timers and scenario selection are deterministic.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::config::ScenarioLoader;
    /// use helix_trainer::minigame::{MiniGameMode, MiniGameSession};
    /// use helix_trainer::time::SystemMonotonicClock;
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en").unwrap());
    /// let session = MiniGameSession::with_sources(
    ///     scenarios,
    ///     None,
    ///     MiniGameMode::default(),
    ///     Arc::new(SystemMonotonicClock),
    ///     StdRng::seed_from_u64(7),
    /// );
    /// assert!(session.state().is_countdown());
    /// ```
    pub fn with_sources(
        scenarios: Arc<Vec<Scenario>>,
        tracker: Option<PerformanceTracker>,
        mode: MiniGameMode,
        clock: Arc<dyn MonotonicClock>,
        rng: R,
    ) -> Self {
        let starting_lives = mode.starting_lives();

        // For Challenge mode, pre-select scenarios using seeded RNG
        let lineup = match &mode {
            MiniGameMode::Challenge(config) => Lineup::Challenge {
                scenarios: select_challenge_scenarios(&scenarios, config),
                next: 0,
                limit: config.scenario_count,
            },
            _ => Lineup::Adaptive,
        };

        let mut session = Self {
            current: None,
            queue: VecDeque::with_capacity(QUEUE_SIZE),
            stats: MiniGameStats::new_with_lives(starting_lives),
            difficulty: DifficultyController::new(),
            score_calculator: ScoreCalculator::new(),
            last_score_breakdown: None,
            phase: Phase::Countdown { remaining: 3 },
            scenarios,
            session_timer: None,
            tracker,
            mode,
            lineup,
            clock,
            rng,
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
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    /// assert!(session.state().is_countdown());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn start(&mut self) {
        self.phase = Phase::Countdown { remaining: 3 };
    }

    /// Process countdown tick
    ///
    /// Decrements countdown and transitions to Playing when done.
    /// Should be called every second during countdown.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    ///
    /// session.tick_countdown();
    /// session.tick_countdown();
    /// session.tick_countdown();
    /// assert!(session.state().is_playing());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn tick_countdown(&mut self) {
        if let Phase::Countdown { remaining } = self.phase {
            if remaining > 1 {
                self.phase = Phase::Countdown {
                    remaining: remaining - 1,
                };
            } else {
                // Countdown finished, start playing
                self.phase = Phase::Playing;
                self.session_timer
                    .get_or_insert_with(|| PausableClock::start(self.clock.now()));
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
    /// Returns [`GameError`] if command execution fails or no scenario is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    /// session.handle_command("x")?; // select line
    /// session.handle_command("d")?; // delete
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn handle_command(&mut self, command: &str) -> Result<(), GameError> {
        if !self.state().is_playing() {
            return Ok(()); // Ignore commands in non-playing states
        }

        let current = self.current.as_mut().ok_or(GameError::NoActiveScenario)?;

        // The simulator interprets any count prefix (e.g. "3d", "2G")
        current.apply_text(command)?;

        Ok(())
    }

    /// Run one parsed Normal-mode command, as [`MiniGameSession::handle_command`]
    /// does for its string form; the recorded action is the canonical string.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no current scenario or the simulator rejects the command.
    pub(crate) fn apply_command(
        &mut self,
        command: &crate::helix::normal_command::NormalCommand,
    ) -> Result<(), GameError> {
        if !self.state().is_playing() {
            return Ok(()); // Ignore commands in non-playing states
        }

        let current = self.current.as_mut().ok_or(GameError::NoActiveScenario)?;
        current.apply_command(command)
    }

    /// Run one typed Insert-mode input, recorded as one action.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no current scenario or the simulator rejects the input.
    pub(crate) fn apply_insert(
        &mut self,
        input: &crate::helix::InsertInput,
    ) -> Result<(), GameError> {
        if !self.state().is_playing() {
            return Ok(()); // Ignore commands in non-playing states
        }

        let current = self.current.as_mut().ok_or(GameError::NoActiveScenario)?;
        current.apply_insert(input)
    }

    /// Check if current scenario is complete
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// if session.check_completion() {
    ///     session.advance_to_next();
    /// }
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn check_completion(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(ActiveMiniScenario::is_completed)
    }

    /// Advance to next scenario after completion
    ///
    /// Calculates score, updates statistics, and loads the next scenario.
    /// Transitions to Transition state briefly before loading next scenario.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.advance_to_next();
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    pub fn advance_to_next(&mut self) {
        if let Some(ref scenario) = self.current {
            // Calculate metrics for scoring
            let time_ratio = scenario.time_used().get();
            let optimal_count = scenario.scenario().scoring.optimal_count.get();
            let actual_count = scenario.action_count().max(1);
            let efficiency = (optimal_count as f64 / actual_count as f64).min(1.0);
            let scenario_difficulty = scenario
                .scenario()
                .metadata
                .difficulty
                .unwrap_or(Difficulty::Beginner);

            // Update completion counter and multiplier state (streak, grace, milestones).
            // Must precede scoring: the new multiplier feeds `calculate` and `add_score`.
            self.stats.record_completion();

            // Calculate score using enhanced ScoreCalculator
            let base_points = Self::base_points_for(scenario);
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
        self.phase = Phase::Transition {
            success: true,
            started_at: self.clock.now(),
        };
    }

    /// Complete transition and load next scenario
    ///
    /// Should be called after brief transition delay.
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if scenario loading fails.
    pub fn complete_transition(&mut self) -> Result<(), GameError> {
        if matches!(self.phase, Phase::Transition { .. }) {
            self.phase = Phase::Playing;
            self.load_next_scenario()?;
        }
        Ok(())
    }

    /// Check if transition delay has elapsed and should auto-advance
    ///
    /// Returns true if in transition state and delay (1 second) has passed.
    pub fn should_advance_to_next(&self) -> bool {
        match self.phase {
            Phase::Transition { started_at, .. } => {
                self.clock.now().saturating_duration_since(started_at) >= Duration::from_secs(1)
            }
            _ => false,
        }
    }

    /// Handle timeout for current scenario
    ///
    /// Deducts a life, resets streak, and loads next scenario.
    /// If no lives remain, transitions to `GameOver` state.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.handle_timeout();
    /// # Ok::<(), helix_trainer::error::UserError>(())
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
            .and_then(|s| s.scenario().metadata.difficulty)
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
            self.phase = Phase::Transition {
                success: false,
                started_at: self.clock.now(),
            };
        } else {
            // Game over
            self.phase = Phase::GameOver;
            self.current = None;
        }
    }

    /// Get remaining time for current scenario
    ///
    /// Returns None if no scenario is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let session = MiniGameSession::new(scenarios, None);
    /// if let Some(time) = session.remaining_time() {
    ///     println!("Time left: {:?}", time);
    /// }
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn remaining_time(&self) -> Option<Duration> {
        self.current
            .as_ref()
            .map(ActiveMiniScenario::remaining_time)
    }

    /// Check if current scenario has timed out
    pub fn is_timed_out(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(ActiveMiniScenario::is_timed_out)
    }

    /// Get elapsed session-level time, excluding any paused duration
    ///
    /// Returns `Duration::ZERO` if gameplay has not started yet (still in countdown).
    fn session_elapsed(&self) -> Duration {
        self.session_timer
            .map_or(Duration::ZERO, |timer| timer.elapsed(self.clock.now()))
    }

    /// Check if the mode's session-level time limit has elapsed
    ///
    /// Always `false` for modes without a session timer (see
    /// [`MiniGameMode::has_session_timer`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// if session.is_session_expired() {
    ///     session.end_session_on_timeout();
    /// }
    /// # Ok::<(), helix_trainer::error::UserError>(())
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
        self.phase = Phase::GameOver;
        self.current = None;
    }

    /// Pause the game
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    /// for _ in 0..3 {
    ///     session.tick_countdown();
    /// }
    /// session.pause();
    /// assert!(session.state().is_paused());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn pause(&mut self) {
        if matches!(self.phase, Phase::Playing) {
            self.phase = Phase::Paused;
            if let Some(ref mut current) = self.current {
                current.pause();
            }
            if let Some(timer) = self.session_timer.as_mut() {
                timer.pause(self.clock.now());
            }
        }
    }

    /// Resume the game from paused state
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// session.start();
    /// for _ in 0..3 {
    ///     session.tick_countdown();
    /// }
    /// session.pause();
    /// session.resume();
    /// assert!(session.state().is_playing());
    /// # Ok::<(), helix_trainer::error::UserError>(())
    /// ```
    pub fn resume(&mut self) {
        if matches!(self.phase, Phase::Paused) {
            self.phase = Phase::Playing;
            if let Some(ref mut current) = self.current {
                current.resume();
            }
            if let Some(timer) = self.session_timer.as_mut() {
                timer.resume(self.clock.now());
            }
        }
    }

    /// Get reference to current scenario
    pub fn current_scenario(&self) -> Option<&ActiveMiniScenario> {
        self.current.as_ref()
    }

    /// Get reference to scenario queue
    pub fn queue(&self) -> &VecDeque<Arc<Scenario>> {
        &self.queue
    }

    /// Get reference to statistics
    pub fn stats(&self) -> &MiniGameStats {
        &self.stats
    }

    /// Get current game state
    pub fn state(&self) -> MiniGameState {
        self.phase.state()
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
    pub(crate) fn try_begin_game_over(&mut self) -> bool {
        if self.game_over_processed {
            false
        } else {
            self.game_over_processed = true;
            true
        }
    }

    /// Get difficulty level
    pub fn difficulty_level(&self) -> DifficultyLevel {
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
    /// ```
    /// use helix_trainer::minigame::{MiniGameSession, LevelChange};
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// // ... gameplay ...
    ///
    /// if let Some(change) = session.take_level_change() {
    ///     match change {
    ///         LevelChange::Increased { from, to } => println!("Level up: {} -> {}", from.get(), to.get()),
    ///         LevelChange::Decreased { from, to } => println!("Level down: {} -> {}", from.get(), to.get()),
    ///     }
    /// }
    /// # Ok::<(), helix_trainer::error::UserError>(())
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
            .is_some_and(ActiveMiniScenario::is_insert_mode)
    }

    /// The editor mode of the current scenario for display and key dispatch
    /// (Normal when there is none)
    pub fn mode_kind(&self) -> crate::helix::ModeKind {
        self.current.as_ref().map_or(
            crate::helix::ModeKind::Normal,
            crate::game::PlayableScenario::mode_kind,
        )
    }

    /// Check if a `Q`/`q` macro is currently being recorded
    pub fn is_recording_macro(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(ActiveMiniScenario::is_recording_macro)
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
    /// ```
    /// use helix_trainer::minigame::MiniGameSession;
    /// use helix_trainer::learning::PerformanceTracker;
    /// use helix_trainer::config::ScenarioLoader;
    /// use std::sync::Arc;
    ///
    /// let scenarios = Arc::new(ScenarioLoader::new().load_from_embedded("en")?);
    /// let mut session = MiniGameSession::new(scenarios, None);
    /// let mut tracker = PerformanceTracker::new();
    ///
    /// // After scenario completion
    /// session.record_to_fsrs(&mut tracker, true);
    /// # Ok::<(), helix_trainer::error::UserError>(())
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
            let time_per_command = elapsed / u32::try_from(action_count).unwrap_or(u32::MAX);

            // Optimal time estimate: time_limit / action_count would be "perfect"
            // We'll use optimal_count from scenario as reference
            let optimal_count = scenario.scenario().scoring.optimal_count.get();
            let optimal_time_per_command =
                scenario.time_limit / u32::try_from(optimal_count).unwrap_or(u32::MAX);

            // Same card ids as training mode, so arcade and training practice of one
            // skill share a card; typed text and arguments never become cards.
            let mut recorded_commands = std::collections::HashSet::new();
            for command in actions.iter().filter_map(UserAction::command_id) {
                if recorded_commands.insert(command) {
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

    /// Get base points for a scenario based on difficulty
    fn base_points_for(scenario: &ActiveMiniScenario) -> u64 {
        if let Some(difficulty) = scenario.scenario().metadata.difficulty {
            use crate::config::Difficulty;
            match difficulty {
                Difficulty::Beginner => 100,
                Difficulty::Intermediate => 200,
                Difficulty::Advanced => 350,
            }
        } else {
            // Fallback if the scenario has no difficulty
            100
        }
    }

    /// Load next scenario from queue
    ///
    /// # Errors
    ///
    /// Returns [`GameError`] if scenario initialization fails or queue is empty.
    fn load_next_scenario(&mut self) -> Result<(), GameError> {
        // Pop from queue
        let scenario = self.queue.pop_front().ok_or(GameError::NoActiveScenario)?;

        // Get time limit based on mode
        let time_limit = self.get_scenario_time_limit(&scenario);

        // Create active scenario
        self.current = Some(ActiveMiniScenario::new(
            scenario,
            time_limit,
            Arc::clone(&self.clock),
        ));

        // Refill queue
        self.refill_queue();

        Ok(())
    }

    /// Get time limit for a scenario based on current game mode
    ///
    /// - Arcade: Uses `DifficultyController`'s adaptive time limit
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

    /// Refill queue to maintain `QUEUE_SIZE`
    ///
    /// Uses different selection strategies based on mode:
    /// - Arcade/Survival: FSRS-weighted selection if tracker available
    /// - Challenge: Uses pre-selected scenarios
    fn refill_queue(&mut self) {
        match &mut self.lineup {
            Lineup::Challenge {
                scenarios,
                next,
                limit,
            } => {
                while self.queue.len() < QUEUE_SIZE && *next < *limit && *next < scenarios.len() {
                    self.queue.push_back(Arc::clone(&scenarios[*next]));
                    *next += 1;
                }
            }
            Lineup::Adaptive => {
                while self.queue.len() < QUEUE_SIZE {
                    let Some(scenario) = self.difficulty.next_scenario(
                        &self.scenarios,
                        self.tracker.as_ref(),
                        &mut self.rng,
                    ) else {
                        break; // No more scenarios available
                    };
                    self.queue.push_back(Arc::new(scenario));
                }
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
                self.stats.is_game_over() || self.challenge_target_met(config.scenario_count)
            }
        }
    }

    /// Check if Challenge mode is complete (all scenarios done)
    pub fn is_challenge_complete(&self) -> bool {
        if let MiniGameMode::Challenge(config) = &self.mode {
            self.challenge_target_met(config.scenario_count)
        } else {
            false
        }
    }

    fn challenge_target_met(&self, scenario_count: usize) -> bool {
        u32::try_from(scenario_count).is_ok_and(|target| self.stats.scenarios_completed >= target)
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
    use crate::helix::Language;
    use crate::minigame::{ArcadeConfig, ChallengeConfig, SurvivalConfig};
    use crate::testing::ScenarioBuilder;
    use crate::testing::cid;
    use crate::time::FakeMonotonicClock;

    fn seeded_session(
        scenarios: Arc<Vec<Scenario>>,
        mode: MiniGameMode,
        clock: Arc<FakeMonotonicClock>,
    ) -> MiniGameSession {
        MiniGameSession::with_sources(scenarios, None, mode, clock, StdRng::seed_from_u64(1))
    }

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
        assert!(session.state().is_countdown());
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
        assert_eq!(session.state().countdown_remaining(), Some(3));

        session.tick_countdown();
        assert_eq!(session.state().countdown_remaining(), Some(2));

        session.tick_countdown();
        assert_eq!(session.state().countdown_remaining(), Some(1));

        session.tick_countdown();
        assert!(session.state().is_playing());
    }

    #[test]
    fn test_timeout_loses_life() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.phase = Phase::Playing;
        let _ = session.load_next_scenario();

        assert_eq!(session.stats.lives(), 3);

        session.handle_timeout();
        assert_eq!(session.stats.lives(), 2);

        session.handle_timeout();
        assert_eq!(session.stats.lives(), 1);

        session.handle_timeout();
        assert_eq!(session.stats.lives(), 0);
        assert!(session.state().is_game_over());
    }

    #[test]
    fn test_pause_resume() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.phase = Phase::Playing;

        session.pause();
        assert!(session.state().is_paused());

        session.resume();
        assert!(session.state().is_playing());
    }

    #[test]
    fn test_active_scenario_language_defaults_to_rs_when_scenario_omits_it() {
        let scenario = create_test_scenario("s1", Difficulty::Beginner);
        let active = ActiveMiniScenario::new(
            Arc::new(scenario),
            Duration::from_secs(60),
            Arc::new(SystemMonotonicClock),
        );
        assert_eq!(active.language(), Language::Rs);
    }

    #[test]
    fn test_active_scenario_language_resolves_from_scenario_setup() {
        let scenario = ScenarioBuilder::new()
            .id("s1")
            .setup_content("line 1\nline 2\n")
            .setup_language(Language::Py)
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .build();
        let active = ActiveMiniScenario::new(
            Arc::new(scenario),
            Duration::from_secs(60),
            Arc::new(SystemMonotonicClock),
        );
        assert_eq!(active.language(), Language::Py);
    }

    fn active_with_clock() -> (ActiveMiniScenario, Arc<FakeMonotonicClock>) {
        let clock = Arc::new(FakeMonotonicClock::new());
        let active = ActiveMiniScenario::new(
            Arc::new(create_test_scenario("s1", Difficulty::Beginner)),
            Duration::from_secs(60),
            clock.clone(),
        );
        (active, clock)
    }

    #[test]
    fn test_active_scenario_pause_freezes_elapsed_time() {
        let (mut active, clock) = active_with_clock();

        clock.advance(Duration::from_millis(10));
        active.pause();
        clock.advance(Duration::from_millis(50));

        assert_eq!(active.elapsed(), Duration::from_millis(10));

        active.resume();
        clock.advance(Duration::from_millis(30));

        assert_eq!(active.elapsed(), Duration::from_millis(40));
    }

    #[test]
    fn test_active_scenario_repeated_pause_resume_accumulates_total_paused() {
        let (mut active, clock) = active_with_clock();

        active.pause();
        clock.advance(Duration::from_millis(30));
        active.resume();
        assert_eq!(active.timer.total_paused, Duration::from_millis(30));

        active.pause();
        clock.advance(Duration::from_millis(20));
        active.resume();
        assert_eq!(active.timer.total_paused, Duration::from_millis(50));
    }

    #[test]
    fn test_active_scenario_pause_resume_idempotent() {
        let (mut active, clock) = active_with_clock();

        active.resume();
        assert_eq!(active.timer.total_paused, Duration::ZERO);
        assert!(active.timer.paused_at.is_none());

        active.pause();
        let paused_at_first = active.timer.paused_at;
        clock.advance(Duration::from_millis(10));
        active.pause();
        assert_eq!(active.timer.paused_at, paused_at_first);

        active.resume();
        assert_eq!(active.timer.total_paused, Duration::from_millis(10));
    }

    #[test]
    fn test_minigame_session_pause_resume_freezes_scenario_elapsed() {
        let clock = Arc::new(FakeMonotonicClock::new());
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = seeded_session(scenarios, MiniGameMode::default(), clock.clone());
        session.phase = Phase::Playing;
        let _ = session.load_next_scenario();

        clock.advance(Duration::from_millis(5));
        session.pause();
        assert!(session.state().is_paused());
        clock.advance(Duration::from_millis(40));

        let elapsed_while_paused = session
            .current_scenario()
            .expect("scenario should still be active")
            .elapsed();
        assert_eq!(elapsed_while_paused, Duration::from_millis(5));

        session.resume();
        assert!(session.state().is_playing());
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
            session.state().is_playing(),
            "Should be playing after countdown"
        );

        // Execute 'ge' command
        assert!(
            session.current_scenario().is_some(),
            "Current scenario should be set"
        );

        let result = session.handle_command("ge");
        assert!(result.is_ok(), "ge command should succeed: {result:?}");

        // Check that the cursor moved
        if let Some(scenario) = session.current_scenario() {
            let row = scenario.current_cursor().row();
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
        assert!(session.state().is_playing());

        session.handle_command("\"ay").unwrap();
        session.handle_command("\"by").unwrap();

        let mut tracker = PerformanceTracker::new();
        session.record_to_fsrs(&mut tracker, true);

        assert!(
            tracker.performance(&cid("\"y")).is_some(),
            "expected the normalized '\"y' card to exist"
        );
        assert_eq!(
            tracker.all_commands().len(),
            1,
            "raw register ops must not become their own cards"
        );
    }

    #[test]
    fn test_record_to_fsrs_skips_counts_arguments_and_typed_text() {
        let scenario = ScenarioBuilder::new()
            .id("test_reviewable_fsrs")
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
        assert!(session.state().is_playing());

        for cmd in ["2l", "fb", "i", "!", "Escape"] {
            session.handle_command(cmd).unwrap();
        }

        let mut tracker = PerformanceTracker::new();
        session.record_to_fsrs(&mut tracker, true);

        let mut cards: Vec<&str> = tracker
            .all_commands()
            .into_iter()
            .map(crate::ids::CommandId::as_str)
            .collect();
        cards.sort_unstable();
        assert_eq!(cards, ["f", "i", "l"]);
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
            .map(|i| create_test_scenario(&format!("s{i}"), Difficulty::Beginner))
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

    /// An Arcade session with a short session duration, driven by `clock`.
    fn short_arcade_session(
        session_duration: Duration,
        clock: Arc<FakeMonotonicClock>,
    ) -> MiniGameSession {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mode = MiniGameMode::Arcade(ArcadeConfig {
            session_duration,
            ..ArcadeConfig::default()
        });
        let mut session = seeded_session(scenarios, mode, clock);
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(session.state().is_playing());
        session
    }

    #[test]
    fn test_session_expires_after_duration_elapses() {
        let clock = Arc::new(FakeMonotonicClock::new());
        let session = short_arcade_session(Duration::from_millis(200), clock.clone());

        assert!(!session.is_session_expired());
        clock.advance(Duration::from_millis(199));
        assert!(!session.is_session_expired());
        clock.advance(Duration::from_millis(1));
        assert!(session.is_session_expired());
    }

    #[test]
    fn test_end_session_on_timeout_transitions_to_game_over_without_consuming_life() {
        let clock = Arc::new(FakeMonotonicClock::new());
        let mut session = short_arcade_session(Duration::from_millis(200), clock.clone());
        let lives_before = session.stats().lives();
        clock.advance(Duration::from_millis(500));
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
        let clock = Arc::new(FakeMonotonicClock::new());
        let mut session = short_arcade_session(Duration::from_secs(60), clock.clone());

        clock.advance(Duration::from_millis(10));
        session.pause();
        assert!(session.state().is_paused());
        assert!(!session.is_session_expired());

        clock.advance(Duration::from_millis(50));
        assert_eq!(session.session_elapsed(), Duration::from_millis(10));

        session.resume();
        assert!(session.state().is_playing());
        clock.advance(Duration::from_millis(50));
        assert_eq!(session.session_elapsed(), Duration::from_millis(60));
    }

    #[test]
    fn test_modes_without_session_timer_never_expire() {
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);

        let survival_mode = MiniGameMode::Survival(SurvivalConfig::default());
        let clock = Arc::new(FakeMonotonicClock::new());
        let mut survival = seeded_session(scenarios.clone(), survival_mode, clock.clone());
        survival.start();
        survival.tick_countdown();
        survival.tick_countdown();
        survival.tick_countdown();

        let challenge_mode =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));
        let mut challenge = seeded_session(scenarios, challenge_mode, clock.clone());
        challenge.start();
        challenge.tick_countdown();
        challenge.tick_countdown();
        challenge.tick_countdown();

        clock.advance(Duration::from_secs(3600));

        assert!(!survival.mode().has_session_timer());
        assert!(!survival.is_session_expired());
        assert!(!challenge.mode().has_session_timer());
        assert!(!challenge.is_session_expired());
    }

    #[test]
    fn test_time_used_is_clamped_fraction_of_limit() {
        let (active, clock) = active_with_clock();

        clock.advance(Duration::from_secs(15));
        assert_eq!(active.time_used(), Ratio::new(0.25));
        clock.advance(Duration::from_secs(600));
        assert_eq!(active.time_used(), Ratio::ONE);
    }

    #[test]
    fn test_ratio_nan_is_exhausted() {
        assert_eq!(Ratio::new(f64::NAN), Ratio::ONE);
    }

    #[test]
    fn test_same_seed_yields_same_queue() {
        let scenarios = Arc::new(
            (0..12)
                .map(|i| create_test_scenario(&format!("s{i}"), Difficulty::Beginner))
                .collect::<Vec<_>>(),
        );
        let queue_ids = |seed: u64| {
            let session = MiniGameSession::with_sources(
                scenarios.clone(),
                None,
                MiniGameMode::default(),
                Arc::new(FakeMonotonicClock::new()),
                StdRng::seed_from_u64(seed),
            );
            session
                .queue()
                .iter()
                .map(|s| s.id.clone())
                .collect::<Vec<_>>()
        };

        assert_eq!(queue_ids(42), queue_ids(42));
    }

    #[test]
    fn test_transition_auto_advances_after_one_second() {
        let clock = Arc::new(FakeMonotonicClock::new());
        let scenarios = Arc::new(vec![create_test_scenario("s1", Difficulty::Beginner)]);
        let mut session = seeded_session(scenarios, MiniGameMode::default(), clock.clone());
        session.advance_to_next();

        clock.advance(Duration::from_millis(999));
        assert!(!session.should_advance_to_next());
        clock.advance(Duration::from_millis(1));
        assert!(session.should_advance_to_next());
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
            .map(|i| create_test_scenario(&format!("s{i}"), Difficulty::Beginner))
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
                "Remaining time {remaining:?} should be <= base time {SURVIVAL_BASE_TIME:?}"
            );
        }

        // Verify config calculation directly
        let level1_time = config.time_limit_for_level(DifficultyLevel::MIN);
        assert_eq!(level1_time, SURVIVAL_BASE_TIME);

        let level5_time = config.time_limit_for_level(DifficultyLevel::new(5).unwrap());
        // Level 5 should have less time than level 1 (500ms * 4 levels = 2s less)
        let expected_decrease = Duration::from_secs(2);
        assert_eq!(
            level5_time,
            SURVIVAL_BASE_TIME.saturating_sub(expected_decrease)
        );
    }
}
