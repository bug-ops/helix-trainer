use chrono::{DateTime, Utc};
use fsrs::{FSRS, MemoryState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::traits::ProgressionTier;
use crate::time::{Clock, SystemClock};

const DEFAULT_DIFFICULTY: f32 = 5.0; // Mixle difficulty (0-10 scale)
const DEFAULT_DESIRED_RETENTION: f32 = 0.9; // 90% target retention

/// FSRS model parameters this tracker's [`FSRS`] instance is constructed with.
///
/// Hoisted so the decay value passed to `fsrs::current_retrievability` in
/// [`PerformanceTracker::update_fsrs_state`] (`FSRS_PARAMS[20]`) is structurally tied
/// to whatever parameters actually built the tracker, instead of a same-named
/// constant that could silently diverge if these parameters ever become
/// user-configurable.
const FSRS_PARAMS: [f32; 21] = fsrs::DEFAULT_PARAMETERS;

/// Card state tracked by the application (FSRS doesn't track this)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardState {
    New,
    Learning,
    Review,
    Relearning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPerformance {
    pub command: String,

    // FSRS state (stored as Option<MemoryState>)
    pub stability: f32,   // Memory stability (days)
    pub difficulty: f32,  // Item difficulty (0-10)
    pub state: CardState, // Card state (application-managed)
    pub reps: u32,        // Total repetitions
    pub lapses: u32,      // Number of lapses (forgot)

    // Performance metrics (application-specific)
    pub attempts: u32,
    pub successes: u32,
    #[serde(with = "duration_serde")]
    pub total_time: Duration,
    #[serde(with = "duration_serde")]
    pub avg_time: Duration,

    // Scheduling
    pub last_review: DateTime<Utc>,
    pub due: DateTime<Utc>,
    pub scheduled_days: u32,

    // Derived fields
    pub mastery_level: MasteryLevel,
    /// Current recall probability (0.0-1.0).
    ///
    /// Deserialized tolerantly: `serde_json` writes a `NaN` `f32` as `null`, and a
    /// past build could produce one here (see the decay-sign fix in
    /// `PerformanceTracker::update_fsrs_state`). Without this, a profile saved
    /// while that bug was live would fail to deserialize at all -- permanently
    /// bricking `profile.json` -- rather than just having one stale field. A
    /// *present* key with value `null` recovers as `1.0`, the same default a
    /// brand-new card gets in [`CommandPerformance::new`] -- this does not cover a
    /// genuinely missing key, which still errors (no build has ever omitted this
    /// field, so that case doesn't need tolerance).
    #[serde(deserialize_with = "deserialize_retrievability")]
    pub retrievability: f32,
}

/// See the `retrievability` field doc on [`CommandPerformance`]. `#[serde(default)]`
/// alone would not have covered this even if paired here: it only fires when the key
/// is absent, not when it's present with value `null`, which is what a `NaN` `f32`
/// round-trips to.
fn deserialize_retrievability<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<f32>::deserialize(deserializer)?.unwrap_or(1.0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MasteryLevel {
    Beginner,
    Intermediate,
    Advanced,
    Master,
}

impl super::traits::ProgressionTier for MasteryLevel {
    fn name(&self) -> &'static str {
        match self {
            MasteryLevel::Beginner => "Beginner",
            MasteryLevel::Intermediate => "Intermediate",
            MasteryLevel::Advanced => "Advanced",
            MasteryLevel::Master => "Master",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            MasteryLevel::Beginner => "🔰",
            MasteryLevel::Intermediate => "📚",
            MasteryLevel::Advanced => "⭐",
            MasteryLevel::Master => "🏆",
        }
    }

    fn tier_level(&self) -> u32 {
        match self {
            MasteryLevel::Beginner => 0,
            MasteryLevel::Intermediate => 1,
            MasteryLevel::Advanced => 2,
            MasteryLevel::Master => 3,
        }
    }

    fn is_max_tier(&self) -> bool {
        matches!(self, MasteryLevel::Master)
    }
}

impl CommandPerformance {
    pub fn new(command: String, now: DateTime<Utc>) -> Self {
        Self {
            command,
            stability: 0.0,
            difficulty: DEFAULT_DIFFICULTY,
            state: CardState::New,
            reps: 0,
            lapses: 0,
            attempts: 0,
            successes: 0,
            total_time: Duration::ZERO,
            avg_time: Duration::ZERO,
            last_review: now,
            due: now,
            scheduled_days: 0,
            mastery_level: MasteryLevel::Beginner,
            retrievability: 1.0, // New card, perfect retrievability
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }

    pub fn update_mastery_level(&mut self) {
        self.mastery_level = match self.state {
            CardState::New | CardState::Learning => MasteryLevel::Beginner,
            CardState::Review | CardState::Relearning => match (self.stability, self.difficulty) {
                (s, d) if s > 30.0 && d < 5.0 => MasteryLevel::Master,
                (s, _) if s > 7.0 => MasteryLevel::Advanced,
                _ => MasteryLevel::Intermediate,
            },
        };
    }

    /// Get MemoryState for FSRS (None if New)
    pub fn memory_state(&self) -> Option<MemoryState> {
        if self.reps == 0 {
            None
        } else {
            Some(MemoryState {
                stability: self.stability,
                difficulty: self.difficulty,
            })
        }
    }

    /// Update from FSRS NextState and determine card state
    pub fn update_from_next_state(
        &mut self,
        memory: MemoryState,
        interval: f32,
        rating_index: usize,
        now: DateTime<Utc>,
    ) {
        // Determine new state based on rating and current state
        self.state = match (self.state, rating_index) {
            // Again (0) - failed
            (_, 0) => {
                self.lapses += 1;
                if self.reps > 0 {
                    CardState::Relearning
                } else {
                    CardState::Learning
                }
            }
            // Hard (1) - struggled but passed
            (CardState::New, 1) | (CardState::Learning, 1) => CardState::Learning,
            (CardState::Relearning, 1) => CardState::Relearning,
            (CardState::Review, 1) => CardState::Review,
            // Good (2) or Easy (3) - succeeded well
            (CardState::New, _) | (CardState::Learning, _) => {
                if interval >= 1.0 {
                    CardState::Review
                } else {
                    CardState::Learning
                }
            }
            (CardState::Relearning, _) => CardState::Review,
            (CardState::Review, _) => CardState::Review,
        };

        self.stability = memory.stability;
        self.difficulty = memory.difficulty;
        self.scheduled_days = interval.round().max(1.0) as u32;
        self.last_review = now;

        // For first review (reps == 0), make command immediately due
        // This allows users to practice new commands right away
        // Subsequent reviews use FSRS scheduling with 1+ day intervals
        if self.reps == 0 {
            self.due = self.last_review; // Immediately due
        } else {
            self.due = self.last_review + chrono::Duration::days(self.scheduled_days as i64);
        }

        self.reps += 1;
    }
}

// Helper module for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Performance tracker for command practice using FSRS algorithm.
///
/// Tracks attempt history, calculates spaced repetition scheduling,
/// and determines command mastery levels.
#[derive(Clone)]
pub struct PerformanceTracker {
    stats: HashMap<String, CommandPerformance>,
    fsrs: FSRS, // FSRS scheduler instance
    clock: Arc<dyn Clock>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemClock))
    }

    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self {
            stats: HashMap::new(),
            fsrs: FSRS::new(&FSRS_PARAMS).unwrap(),
            clock,
        }
    }

    /// Replace the clock this tracker reads time from.
    ///
    /// Used by [`crate::ui::state::ProgressState::with_clock`] to keep this tracker and its
    /// sibling [`super::Scheduler`] reading from the same injected clock instance.
    pub fn set_clock(&mut self, clock: Arc<dyn Clock>) {
        self.clock = clock;
    }

    pub fn record_attempt(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
    ) {
        // Single `now` read shared by CommandPerformance::new (first-attempt case),
        // update_fsrs_state's elapsed-days calculation, and the persisted last_review
        // timestamp — previously each read Utc::now() independently.
        let now = self.clock.now();

        // Get or create performance entry
        let perf = self
            .stats
            .entry(command.to_string())
            .or_insert_with(|| CommandPerformance::new(command.to_string(), now));

        // Update attempt counters
        perf.attempts += 1;
        if success {
            perf.successes += 1;
        }

        perf.total_time += duration;
        perf.avg_time = perf.total_time / perf.attempts;

        // Update FSRS state
        self.update_fsrs_state(command, duration, success, optimal_time, now);

        // Update mastery level
        if let Some(perf) = self.stats.get_mut(command) {
            perf.update_mastery_level();
        }
    }

    /// Calculate next FSRS state based on rating
    fn calculate_next_fsrs_state(
        &self,
        memory_state: Option<MemoryState>,
        elapsed_days: u32,
        rating_index: usize,
    ) -> fsrs::ItemState {
        let next_states = self
            .fsrs
            .next_states(memory_state, DEFAULT_DESIRED_RETENTION, elapsed_days)
            .unwrap();

        match rating_index {
            0 => next_states.again,
            1 => next_states.hard,
            2 => next_states.good,
            3 => next_states.easy,
            _ => unreachable!("rating_index must be 0-3"),
        }
    }

    /// Calculate elapsed days since last review
    fn elapsed_days_since_review(perf: &CommandPerformance, now: DateTime<Utc>) -> u32 {
        if perf.reps == 0 {
            0
        } else {
            (now - perf.last_review).num_days().max(0) as u32
        }
    }

    fn update_fsrs_state(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
        now: DateTime<Utc>,
    ) {
        let perf = self.stats.get(command).unwrap();

        // Convert performance to FSRS rating (0-3 index for again/hard/good/easy)
        let rating_index = Self::calculate_rating_index(duration, success, optimal_time);

        // Get current memory state and elapsed time
        let memory_state = perf.memory_state();
        let elapsed_days = Self::elapsed_days_since_review(perf, now);

        // Calculate next state from FSRS
        let next_state = self.calculate_next_fsrs_state(memory_state, elapsed_days, rating_index);

        // Update performance with new FSRS state
        let perf = self.stats.get_mut(command).unwrap();
        perf.update_from_next_state(next_state.memory, next_state.interval, rating_index, now);

        // Update retrievability (current recall probability)
        //
        // `decay` must be the raw (positive) model weight `w[20]`, not its negation:
        // `fsrs::current_retrievability`'s formula mirrors the crate's internal
        // `power_forgetting_curve`, which computes its own `decay = -w[20]` before use.
        // Passing a negative value here double-flips the sign, driving the curve's base
        // negative for perfectly ordinary inputs and producing NaN via `powf`. Read from
        // `FSRS_PARAMS[20]` (the same array `self.fsrs` above is constructed with, see
        // `with_clock`/`from_stats_with_clock`) rather than a same-named constant, so
        // this can't silently diverge if these parameters become configurable later.
        perf.retrievability = fsrs::current_retrievability(
            perf.memory_state().unwrap_or(MemoryState {
                stability: 0.0,
                difficulty: DEFAULT_DIFFICULTY,
            }),
            elapsed_days as f32,
            FSRS_PARAMS[20],
        );
    }

    fn calculate_rating_index(duration: Duration, success: bool, optimal_time: Duration) -> usize {
        if !success {
            return 0; // Again (complete failure)
        }

        let ratio = duration.as_secs_f64() / optimal_time.as_secs_f64();

        match ratio {
            r if r <= 1.5 => 3, // Easy - instant recall (< 1.5x optimal)
            r if r <= 3.0 => 2, // Good - normal recall (1.5-3x optimal)
            r if r <= 5.0 => 1, // Hard - struggled (3-5x optimal)
            _ => 0,             // Again - essentially forgot (> 5x optimal)
        }
    }

    pub fn stats_clone(&self) -> HashMap<String, CommandPerformance> {
        self.stats.clone()
    }

    pub fn from_stats(stats: HashMap<String, CommandPerformance>) -> Self {
        Self::from_stats_with_clock(stats, Arc::new(SystemClock))
    }

    pub fn from_stats_with_clock(
        stats: HashMap<String, CommandPerformance>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            stats,
            fsrs: FSRS::new(&FSRS_PARAMS).unwrap(),
            clock,
        }
    }

    pub fn performance(&self, command: &str) -> Option<&CommandPerformance> {
        self.stats.get(command)
    }

    /// Record an attempt and return mastery level change if any
    ///
    /// Returns Some((command, old_level, new_level)) if mastery level changed
    pub fn record_attempt_with_mastery_change(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
    ) -> Option<(String, MasteryLevel, MasteryLevel)> {
        // Get old mastery level (Beginner if new command)
        let old_level = self
            .stats
            .get(command)
            .map(|p| p.mastery_level)
            .unwrap_or(MasteryLevel::Beginner);

        // Record the attempt
        self.record_attempt(command, duration, success, optimal_time);

        // Get new mastery level
        let new_level = self.stats.get(command).map(|p| p.mastery_level)?;

        // Return change if level improved
        if new_level.tier_level() > old_level.tier_level() {
            Some((command.to_string(), old_level, new_level))
        } else {
            None
        }
    }

    pub fn weak_commands(&self) -> Vec<String> {
        self.stats
            .iter()
            .filter(|(_, perf)| {
                perf.difficulty > 7.0
                    || perf.success_rate() < 0.7
                    || perf.mastery_level == MasteryLevel::Beginner
                    || perf.lapses > 2
            })
            .map(|(cmd, _)| cmd.clone())
            .collect()
    }

    pub fn all_commands(&self) -> Vec<&str> {
        self.stats.keys().map(String::as_str).collect()
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::time::FakeClock;

    #[test]
    fn test_new_command_defaults() {
        let perf = CommandPerformance::new("x".to_string(), Utc::now());
        assert_eq!(perf.stability, 0.0);
        assert_eq!(perf.difficulty, DEFAULT_DIFFICULTY);
        assert_eq!(perf.reps, 0);
        assert_eq!(perf.state, CardState::New);
        assert_eq!(perf.mastery_level, MasteryLevel::Beginner);
        assert_eq!(perf.retrievability, 1.0);
        assert!(perf.memory_state().is_none());
    }

    /// Regression test: a `profile.json` written while the decay-sign bug (see
    /// `PerformanceTracker::update_fsrs_state`) was live could contain a `NaN`
    /// `retrievability`, which `serde_json` round-trips as `null`. Deserializing that
    /// must recover as `1.0`, not fail the whole `CommandPerformance` (and therefore
    /// the whole profile load).
    #[test]
    fn test_deserialize_null_retrievability_recovers_as_one() {
        let json = r#"{
            "command": "x",
            "stability": 5.0,
            "difficulty": 6.0,
            "state": "Review",
            "reps": 3,
            "lapses": 1,
            "attempts": 3,
            "successes": 2,
            "total_time": 3000,
            "avg_time": 1000,
            "last_review": "2026-01-15T12:00:00Z",
            "due": "2026-01-20T12:00:00Z",
            "scheduled_days": 5,
            "mastery_level": "Intermediate",
            "retrievability": null
        }"#;

        let perf: CommandPerformance = serde_json::from_str(json).unwrap();
        assert_eq!(perf.retrievability, 1.0);
        // Everything else deserializes normally -- only `retrievability` gets tolerant handling.
        assert_eq!(perf.command, "x");
        assert_eq!(perf.stability, 5.0);
    }

    #[test]
    fn test_perfect_performance_increases_stability() {
        let mut tracker = PerformanceTracker::new();

        // First attempt (perfect)
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        let perf = tracker.performance("x").unwrap();
        assert!(perf.stability > 0.0);
        assert_eq!(perf.reps, 1);
        assert!(matches!(
            perf.state,
            CardState::Learning | CardState::Review
        ));

        let first_stability = perf.stability;

        // Second attempt (perfect)
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        let perf = tracker.performance("x").unwrap();
        assert!(perf.stability > first_stability); // FSRS increases stability
        assert_eq!(perf.reps, 2);
    }

    #[test]
    fn test_failure_increases_lapses() {
        let mut tracker = PerformanceTracker::new();

        // Build up some stability
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        let lapses_before = tracker.performance("x").unwrap().lapses;

        // Fail
        tracker.record_attempt("x", Duration::from_secs(10), false, Duration::from_secs(1));

        let perf = tracker.performance("x").unwrap();
        // Lapse count increases when failing
        assert_eq!(perf.lapses, lapses_before + 1);
        assert!(matches!(
            perf.state,
            CardState::Relearning | CardState::Learning
        ));
    }

    #[test]
    fn test_difficulty_bounds() {
        let mut tracker = PerformanceTracker::new();

        // Spam failures - difficulty should stay within 0-10
        for _ in 0..100 {
            tracker.record_attempt("x", Duration::from_secs(10), false, Duration::from_secs(1));
        }

        let perf = tracker.performance("x").unwrap();
        assert!(perf.difficulty >= 0.0);
        assert!(perf.difficulty <= 10.0);
    }

    #[test]
    fn test_rating_conversion() {
        // Test performance → FSRS rating conversion
        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(1),
                true,
                Duration::from_secs(1)
            ),
            3 // Easy
        );

        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(2),
                true,
                Duration::from_secs(1)
            ),
            2 // Good
        );

        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(4),
                true,
                Duration::from_secs(1)
            ),
            1 // Hard
        );

        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(10),
                false,
                Duration::from_secs(1)
            ),
            0 // Again
        );
    }

    #[test]
    fn test_memory_state_conversion() {
        let mut perf = CommandPerformance::new("x".to_string(), Utc::now());
        assert!(perf.memory_state().is_none());

        perf.stability = 5.0;
        perf.difficulty = 6.0;
        perf.reps = 1;
        perf.state = CardState::Review;

        let memory = perf.memory_state().unwrap();
        assert_eq!(memory.stability, 5.0);
        assert_eq!(memory.difficulty, 6.0);
    }

    #[test]
    fn test_mastery_level_progression() {
        let mut perf = CommandPerformance::new("x".to_string(), Utc::now());
        assert_eq!(perf.mastery_level, MasteryLevel::Beginner);

        // Simulate progression through states
        perf.state = CardState::Review;
        perf.stability = 5.0;
        perf.update_mastery_level();
        assert_eq!(perf.mastery_level, MasteryLevel::Intermediate);

        perf.stability = 10.0;
        perf.update_mastery_level();
        assert_eq!(perf.mastery_level, MasteryLevel::Advanced);

        perf.stability = 35.0;
        perf.difficulty = 4.0;
        perf.update_mastery_level();
        assert_eq!(perf.mastery_level, MasteryLevel::Master);
    }

    #[test]
    fn test_weak_commands_detection() {
        let mut tracker = PerformanceTracker::new();

        // Add a weak command (low success rate)
        for _ in 0..10 {
            tracker.record_attempt(
                "weak",
                Duration::from_secs(10),
                false,
                Duration::from_secs(1),
            );
        }

        // Add a strong command
        for _ in 0..10 {
            tracker.record_attempt(
                "strong",
                Duration::from_secs(1),
                true,
                Duration::from_secs(1),
            );
        }

        let weak = tracker.weak_commands();
        assert!(weak.contains(&"weak".to_string()));
        // "strong" might still be beginner mastery level initially, so don't assert it's not weak
    }

    #[test]
    fn test_record_attempt_honors_injected_clock() {
        let clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
        let mut tracker = PerformanceTracker::with_clock(clock.clone());

        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        let first_last_review = tracker.performance("x").unwrap().last_review;
        assert_eq!(first_last_review, clock.now());

        clock.advance_days(3);
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        let second_last_review = tracker.performance("x").unwrap().last_review;
        assert_eq!(second_last_review, clock.now());
        assert_eq!(
            (second_last_review - first_last_review).num_days(),
            3,
            "elapsed_days_since_review must reflect the fake clock, not wall time"
        );
    }

    #[test]
    fn test_all_commands() {
        let mut tracker = PerformanceTracker::new();

        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("yy", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("p", Duration::from_secs(1), true, Duration::from_secs(1));

        let all = tracker.all_commands();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"x"));
        assert!(all.contains(&"yy"));
        assert!(all.contains(&"p"));
    }

    /// Strategy for one simulated review: (success, duration, optimal duration, days
    /// elapsed since the previous review). `elapsed_days` ranges up to a year so both
    /// properties exercise FSRS's primary scheduling input (`days_elapsed` in
    /// [`PerformanceTracker::calculate_next_fsrs_state`]), not just same-instant reviews.
    fn attempt_strategy() -> impl Strategy<Value = (bool, u64, u64, i64)> {
        (any::<bool>(), 1u64..90, 1u64..90, 0i64..365)
    }

    proptest! {
        #[test]
        fn prop_fsrs_state_transition_is_deterministic(
            attempts in prop::collection::vec(attempt_strategy(), 1..12),
        ) {
            let run = || {
                let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
                let mut tracker = PerformanceTracker::with_clock(clock.clone());
                for &(success, duration_secs, optimal_secs, elapsed_days) in &attempts {
                    clock.advance_days(elapsed_days);
                    tracker.record_attempt(
                        "x",
                        Duration::from_secs(duration_secs),
                        success,
                        Duration::from_secs(optimal_secs),
                    );
                }
                tracker.performance("x").unwrap().clone()
            };

            let first = run();
            let second = run();

            // Same starting state + same rating sequence + same simulated elapsed
            // time between reviews must always produce the same resulting FSRS
            // state -- no hidden randomness or wall-clock reads.
            prop_assert_eq!(first.stability, second.stability);
            prop_assert_eq!(first.difficulty, second.difficulty);
            prop_assert_eq!(first.state, second.state);
            prop_assert_eq!(first.reps, second.reps);
            prop_assert_eq!(first.lapses, second.lapses);
            prop_assert_eq!(first.scheduled_days, second.scheduled_days);
            prop_assert_eq!(first.due, second.due);
            prop_assert_eq!(first.retrievability, second.retrievability);
        }

        /// Bounds this crate's glue code doesn't itself enforce -- unlike `scheduled_days`
        /// (`.max(1.0)` at :182, cannot be zero by construction) or `due` (defined as
        /// `last_review + scheduled_days`, cannot precede it by construction) -- these
        /// come straight from the `fsrs` crate's own model output, so a regression in how
        /// this module drives that crate (wrong parameter order, unclamped passthrough,
        /// wrong decay sign) would show up here. Broader than `test_difficulty_bounds`
        /// above: exercises Hard/Good/Easy ratings and elapsed-day gaps up to a year, not
        /// just repeated same-instant failures.
        #[test]
        fn prop_fsrs_state_stays_in_bounds(
            attempts in prop::collection::vec(attempt_strategy(), 1..20),
        ) {
            let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
            let mut tracker = PerformanceTracker::with_clock(clock.clone());

            for &(success, duration_secs, optimal_secs, elapsed_days) in &attempts {
                clock.advance_days(elapsed_days);
                tracker.record_attempt(
                    "x",
                    Duration::from_secs(duration_secs),
                    success,
                    Duration::from_secs(optimal_secs),
                );

                let perf = tracker.performance("x").unwrap();
                // FSRS's own difficulty domain (fsrs::simulation::{D_MIN, D_MAX}).
                prop_assert!(
                    (1.0..=10.0).contains(&perf.difficulty),
                    "difficulty {} outside FSRS's [1, 10] domain",
                    perf.difficulty
                );
                // FSRS clamps stability to a strictly positive minimum
                // (fsrs::simulation::S_MIN); zero would make retrievability degenerate.
                prop_assert!(
                    perf.stability > 0.0,
                    "stability {} must be strictly positive",
                    perf.stability
                );
                prop_assert!(
                    (0.0..=1.0).contains(&perf.retrievability),
                    "retrievability {} outside the documented [0, 1] probability range",
                    perf.retrievability
                );
            }
        }
    }
}
