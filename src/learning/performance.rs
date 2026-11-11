use chrono::{DateTime, Utc};
use fsrs::{FSRS, MemoryState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use super::LearningError;
use super::reviewable::ReviewableCommand;
use super::traits::ProgressionTier;
use crate::time::{Clock, SystemClock};

const DEFAULT_DIFFICULTY: f32 = 5.0; // Mixle difficulty (0-10 scale)
const DEFAULT_DESIRED_RETENTION: f32 = 0.9; // 90% target retention

/// FSRS model parameters this tracker's [`FSRS`] instance is constructed with.
///
/// Hoisted so the decay value passed to `fsrs::current_retrievability` in
/// [`CommandPerformance::retrievability`] (`FSRS_PARAMS[20]`) is structurally tied
/// to whatever parameters actually built the tracker, instead of a same-named
/// constant that could silently diverge if these parameters ever become
/// user-configurable. The decay passed there must be the raw positive weight `w[20]`:
/// `fsrs::current_retrievability` negates it itself, and a pre-negated value drives the
/// curve's base negative and yields NaN.
const FSRS_PARAMS: [f32; 21] = fsrs::DEFAULT_PARAMETERS;

fn default_fsrs() -> FSRS {
    // Cannot fail: `FSRS_PARAMS` is the crate's own default parameter set.
    FSRS::new(&FSRS_PARAMS).expect("default FSRS parameters are valid")
}

/// Card state tracked by the application (FSRS doesn't track this)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardState {
    /// Never reviewed.
    New,
    /// Being learned for the first time.
    Learning,
    /// Graduated to spaced review.
    Review,
    /// Forgotten after graduating and being learned again.
    Relearning,
}

/// Persisted per-command FSRS data, keyed by command string.
///
/// The profile and the tracker exchange this named type instead of a bare `HashMap`, and
/// the on-disk JSON shape stays a plain object. Deserialization drops, rather than fails
/// on, cards whose id is not a canonical [`ReviewableCommand`] (saved before typed text
/// and arguments were excluded) and cards that fail validation (non-finite or
/// out-of-range FSRS values, `successes > attempts`); the dropped count is logged.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct PerformanceData(HashMap<String, CommandPerformance>);

impl<'de> Deserialize<'de> for PerformanceData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut cards = HashMap::with_capacity(raw.len());
        let mut not_reviewable = 0_usize;
        let mut invalid = Vec::new();
        for (command, value) in raw {
            if ReviewableCommand::from_card_id(&command).is_none() {
                not_reviewable += 1;
                continue;
            }
            let card = CardRecord::deserialize(value)
                .map_err(|e| e.to_string())
                .and_then(|record| {
                    CommandPerformance::from_record(command.clone(), record)
                        .map_err(|e| e.to_string())
                });
            match card {
                Ok(card) => {
                    cards.insert(command, card);
                }
                Err(reason) => invalid.push((command, reason)),
            }
        }
        if not_reviewable > 0 {
            tracing::warn!(
                dropped = not_reviewable,
                "dropped persisted cards that are not reviewable commands"
            );
        }
        if !invalid.is_empty() {
            tracing::warn!(dropped = invalid.len(), cards = ?invalid, "dropped invalid persisted cards");
        }
        Ok(Self(cards))
    }
}

impl PerformanceData {
    /// Look up the record for `command`.
    #[must_use]
    pub fn get(&self, command: &str) -> Option<&CommandPerformance> {
        self.0.get(command)
    }

    /// Whether a record exists for `command`.
    #[must_use]
    pub fn contains_key(&self, command: &str) -> bool {
        self.0.contains_key(command)
    }

    /// Number of tracked commands.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no command is tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over `(command, record)` pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CommandPerformance)> {
        self.0.iter()
    }
}

impl From<HashMap<String, CommandPerformance>> for PerformanceData {
    fn from(map: HashMap<String, CommandPerformance>) -> Self {
        Self(map)
    }
}

impl From<PerformanceData> for HashMap<String, CommandPerformance> {
    fn from(data: PerformanceData) -> Self {
        data.0
    }
}

/// FSRS memory stability in days: finite and non-negative.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Stability(f32);

impl Stability {
    fn new(value: f32) -> Result<Self, LearningError> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(LearningError::InvalidStability(f64::from(value)))
        }
    }
}

/// FSRS item difficulty: finite and inside `0..=10`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Difficulty(f32);

impl Difficulty {
    const DEFAULT: Self = Self(DEFAULT_DIFFICULTY);

    fn new(value: f32) -> Result<Self, LearningError> {
        if (0.0..=10.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(LearningError::InvalidDifficulty(f64::from(value)))
        }
    }
}

/// Per-command FSRS card plus practice statistics.
///
/// Fields are private so the FSRS update path is the only writer. Stability and
/// difficulty are range-checked newtypes, and everything derivable (average time, mastery
/// tier, retrievability) is computed on read instead of stored. The persisted shape is
/// `CardRecord`; cards are only built from disk through [`PerformanceData`]'s
/// deserializer, which validates them.
#[derive(Debug, Clone)]
pub struct CommandPerformance {
    command: String,
    stability: Stability,
    difficulty: Difficulty,
    state: CardState,
    reps: u32,
    lapses: u32,
    attempts: u32,
    successes: u32,
    total_time: Duration,
    last_review: DateTime<Utc>,
    due: DateTime<Utc>,
    scheduled_days: u32,
}

/// On-disk shape of a [`CommandPerformance`]. Keys that are derivable (`avg_time`,
/// `mastery_level`, `retrievability`) or redundant (`command`) are neither written nor
/// required; older files that still carry them load with those keys ignored.
#[derive(Serialize, Deserialize)]
struct CardRecord {
    stability: f32,
    difficulty: f32,
    state: CardState,
    reps: u32,
    lapses: u32,
    attempts: u32,
    successes: u32,
    #[serde(with = "duration_serde")]
    total_time: Duration,
    last_review: DateTime<Utc>,
    due: DateTime<Utc>,
    scheduled_days: u32,
}

impl From<&CommandPerformance> for CardRecord {
    fn from(perf: &CommandPerformance) -> Self {
        Self {
            stability: perf.stability.0,
            difficulty: perf.difficulty.0,
            state: perf.state,
            reps: perf.reps,
            lapses: perf.lapses,
            attempts: perf.attempts,
            successes: perf.successes,
            total_time: perf.total_time,
            last_review: perf.last_review,
            due: perf.due,
            scheduled_days: perf.scheduled_days,
        }
    }
}

impl Serialize for CommandPerformance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CardRecord::from(self).serialize(serializer)
    }
}

/// Coarse mastery tier derived from a command's FSRS state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MasteryLevel {
    /// New or still being learned.
    Beginner,
    /// In review with short stability.
    Intermediate,
    /// In review with stability above a week.
    Advanced,
    /// Stable for over a month with low difficulty.
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
    /// Creates a record for a command seen for the first time at `now`.
    pub fn new(command: String, now: DateTime<Utc>) -> Self {
        Self {
            command,
            stability: Stability(0.0),
            difficulty: Difficulty::DEFAULT,
            state: CardState::New,
            reps: 0,
            lapses: 0,
            attempts: 0,
            successes: 0,
            total_time: Duration::ZERO,
            last_review: now,
            due: now,
            scheduled_days: 0,
        }
    }

    fn from_record(command: String, record: CardRecord) -> Result<Self, LearningError> {
        if record.successes > record.attempts {
            return Err(LearningError::InvalidCounters {
                attempts: record.attempts,
                successes: record.successes,
            });
        }
        Ok(Self {
            command,
            stability: Stability::new(record.stability)?,
            difficulty: Difficulty::new(record.difficulty)?,
            state: record.state,
            reps: record.reps,
            lapses: record.lapses,
            attempts: record.attempts,
            successes: record.successes,
            total_time: record.total_time,
            last_review: record.last_review,
            due: record.due,
            scheduled_days: record.scheduled_days,
        })
    }

    /// Command text this record tracks.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// FSRS memory stability in days.
    #[must_use]
    pub fn stability(&self) -> f32 {
        self.stability.0
    }

    /// FSRS item difficulty (0-10).
    #[must_use]
    pub fn difficulty(&self) -> f32 {
        self.difficulty.0
    }

    /// Application-managed card state.
    #[must_use]
    pub fn state(&self) -> CardState {
        self.state
    }

    /// Total successful repetitions scheduled by FSRS.
    #[must_use]
    pub fn reps(&self) -> u32 {
        self.reps
    }

    /// Number of times the command was forgotten.
    #[must_use]
    pub fn lapses(&self) -> u32 {
        self.lapses
    }

    /// Total recorded attempts.
    #[must_use]
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Total successful attempts.
    #[must_use]
    pub fn successes(&self) -> u32 {
        self.successes
    }

    /// Cumulative time spent across attempts.
    #[must_use]
    pub fn total_time(&self) -> Duration {
        self.total_time
    }

    /// Average time per attempt, derived from the totals.
    #[must_use]
    pub fn avg_time(&self) -> Duration {
        self.total_time / self.attempts.max(1)
    }

    /// Timestamp of the last review.
    #[must_use]
    pub fn last_review(&self) -> DateTime<Utc> {
        self.last_review
    }

    /// Timestamp when the command is next due.
    #[must_use]
    pub fn due(&self) -> DateTime<Utc> {
        self.due
    }

    /// Interval in days chosen by the last FSRS schedule.
    #[must_use]
    pub fn scheduled_days(&self) -> u32 {
        self.scheduled_days
    }

    /// Mastery tier, derived from the card state, stability and difficulty.
    #[must_use]
    pub fn mastery_level(&self) -> MasteryLevel {
        match self.state {
            CardState::New | CardState::Learning => MasteryLevel::Beginner,
            CardState::Review | CardState::Relearning => {
                match (self.stability.0, self.difficulty.0) {
                    (s, d) if s > 30.0 && d < 5.0 => MasteryLevel::Master,
                    (s, _) if s > 7.0 => MasteryLevel::Advanced,
                    _ => MasteryLevel::Intermediate,
                }
            }
        }
    }

    /// Recall probability (0.0-1.0) at `now`, derived from the memory state and the time
    /// since the last review. A card that was never reviewed has probability 1.0.
    #[must_use]
    pub fn retrievability(&self, now: DateTime<Utc>) -> f32 {
        let Some(memory) = self.memory_state() else {
            return 1.0;
        };
        if memory.stability <= 0.0 {
            return 0.0;
        }
        let elapsed_days = (now - self.last_review).num_days().max(0) as f32;
        fsrs::current_retrievability(memory, elapsed_days, FSRS_PARAMS[20])
    }

    /// Test-only: overwrite the FSRS difficulty to stage a scheduling scenario.
    #[cfg(test)]
    pub(crate) fn set_difficulty(&mut self, difficulty: f32) {
        self.difficulty = Difficulty::new(difficulty).expect("test difficulty is in range");
    }

    /// Test-only: overwrite the scheduled interval to stage a scheduling scenario.
    #[cfg(test)]
    pub(crate) fn set_scheduled_days(&mut self, scheduled_days: u32) {
        self.scheduled_days = scheduled_days;
    }

    /// Test-only: overwrite the due timestamp to stage a scheduling scenario.
    #[cfg(test)]
    pub(crate) fn set_due(&mut self, due: DateTime<Utc>) {
        self.due = due;
    }

    /// Fraction of attempts that succeeded; `0.0` before any attempt.
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }

    /// Get MemoryState for FSRS (None if New)
    pub fn memory_state(&self) -> Option<MemoryState> {
        if self.reps == 0 {
            None
        } else {
            Some(MemoryState {
                stability: self.stability.0,
                difficulty: self.difficulty.0,
            })
        }
    }

    /// Update from FSRS NextState and determine card state
    ///
    /// # Errors
    ///
    /// Returns [`LearningError::InvalidStability`] or [`LearningError::InvalidDifficulty`]
    /// when `memory` is out of domain; the card is left unchanged.
    pub fn update_from_next_state(
        &mut self,
        memory: MemoryState,
        interval: f32,
        rating_index: usize,
        now: DateTime<Utc>,
    ) -> Result<(), LearningError> {
        let stability = Stability::new(memory.stability)?;
        let difficulty = Difficulty::new(memory.difficulty)?;

        // Determine new state based on rating and current state
        self.state = match (self.state, rating_index) {
            // Again (0) - failed
            (_, 0) => {
                self.lapses = self.lapses.saturating_add(1);
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

        self.stability = stability;
        self.difficulty = difficulty;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "FSRS clamps intervals to its maximum interval, a few decades of days, and the value is at least 1"
        )]
        let scheduled_days = interval.round().max(1.0) as u32;
        self.scheduled_days = scheduled_days;
        self.last_review = now;

        // For first review (reps == 0), make command immediately due
        // This allows users to practice new commands right away
        // Subsequent reviews use FSRS scheduling with 1+ day intervals
        if self.reps == 0 {
            self.due = self.last_review; // Immediately due
        } else {
            self.due = self.last_review + chrono::Duration::days(self.scheduled_days as i64);
        }

        self.reps = self.reps.saturating_add(1);
        Ok(())
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
        serializer.serialize_u64(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
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
    /// Creates an empty tracker that reads time from the system clock.
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemClock))
    }

    /// Creates an empty tracker that reads time from `clock`, so tests can control it.
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self {
            stats: HashMap::new(),
            fsrs: default_fsrs(),
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

    /// The clock this tracker reads time from.
    ///
    /// Lets sibling components (such as the arcade prioritizer) share one injected
    /// time source instead of reading the system clock independently.
    #[must_use]
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }

    /// Record an attempt, logging an FSRS update failure instead of returning it.
    ///
    /// For call sites with no recovery path: the attempt is still counted and the schedule
    /// stays unchanged. See [`Self::try_record_attempt`] to observe the error.
    pub fn record_attempt(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
    ) {
        if let Err(error) = self.try_record_attempt(command, duration, success, optimal_time) {
            tracing::error!(
                command,
                %error,
                "FSRS update failed; attempt recorded without a schedule change"
            );
        }
    }

    /// Record an attempt and advance the command's FSRS schedule.
    ///
    /// # Errors
    ///
    /// Returns [`LearningError::FsrsError`] (or an invalid stability/difficulty error) when
    /// the FSRS update fails. The attempt counters and total time are still updated; the
    /// card's schedule, memory state and review count are not.
    pub fn try_record_attempt(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
    ) -> Result<(), LearningError> {
        // One `now` read shared by card creation, the elapsed-days calculation and the
        // persisted last_review timestamp.
        let now = self.clock.now();

        let perf = self
            .stats
            .entry(command.to_string())
            .or_insert_with(|| CommandPerformance::new(command.to_string(), now));

        perf.attempts = perf.attempts.saturating_add(1);
        if success {
            perf.successes = perf.successes.saturating_add(1);
        }
        perf.total_time = perf.total_time.saturating_add(duration);

        // Rating index 0-3 for again/hard/good/easy.
        let rating_index = Self::calculate_rating_index(duration, success, optimal_time);
        let elapsed_days = Self::elapsed_days_since_review(perf, now);
        let next_state =
            Self::next_item_state(&self.fsrs, perf.memory_state(), elapsed_days, rating_index)?;
        perf.update_from_next_state(next_state.memory, next_state.interval, rating_index, now)
    }

    fn next_item_state(
        fsrs: &FSRS,
        memory_state: Option<MemoryState>,
        elapsed_days: u32,
        rating_index: usize,
    ) -> Result<fsrs::ItemState, LearningError> {
        let next_states = fsrs
            .next_states(memory_state, DEFAULT_DESIRED_RETENTION, elapsed_days)
            .map_err(|error| LearningError::FsrsError(error.to_string()))?;

        Ok(match rating_index {
            0 => next_states.again,
            1 => next_states.hard,
            2 => next_states.good,
            3 => next_states.easy,
            _ => unreachable!("rating_index must be 0-3"),
        })
    }

    fn elapsed_days_since_review(perf: &CommandPerformance, now: DateTime<Utc>) -> u32 {
        if perf.reps == 0 {
            0
        } else {
            u32::try_from((now - perf.last_review).num_days().max(0)).unwrap_or(u32::MAX)
        }
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

    /// Clones the tracked data for persistence.
    pub fn stats_clone(&self) -> PerformanceData {
        PerformanceData(self.stats.clone())
    }

    /// Restores a tracker from persisted data using the system clock.
    pub fn from_stats(stats: PerformanceData) -> Self {
        Self::from_stats_with_clock(stats, Arc::new(SystemClock))
    }

    /// Restores a tracker from persisted data using `clock`.
    pub fn from_stats_with_clock(stats: PerformanceData, clock: Arc<dyn Clock>) -> Self {
        Self {
            stats: stats.into(),
            fsrs: default_fsrs(),
            clock,
        }
    }

    /// Looks up the record of a command, if it was ever attempted.
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
            .map(|p| p.mastery_level())
            .unwrap_or(MasteryLevel::Beginner);

        // Record the attempt
        self.record_attempt(command, duration, success, optimal_time);

        // Get new mastery level
        let new_level = self.stats.get(command).map(|p| p.mastery_level())?;

        // Return change if level improved
        if new_level.tier_level() > old_level.tier_level() {
            Some((command.to_string(), old_level, new_level))
        } else {
            None
        }
    }

    /// Commands that are hard, unreliable, still at beginner level or have many lapses.
    pub fn weak_commands(&self) -> Vec<String> {
        self.stats
            .iter()
            .filter(|(_, perf)| {
                perf.difficulty() > 7.0
                    || perf.success_rate() < 0.7
                    || perf.mastery_level() == MasteryLevel::Beginner
                    || perf.lapses > 2
            })
            .map(|(cmd, _)| cmd.clone())
            .collect()
    }

    /// Every command that has a record, in unspecified order.
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
    use std::assert_matches;

    fn card_json(stability: &str, difficulty: &str, attempts: u32, successes: u32) -> String {
        format!(
            r#"{{
                "stability": {stability},
                "difficulty": {difficulty},
                "state": "Review",
                "reps": 3,
                "lapses": 1,
                "attempts": {attempts},
                "successes": {successes},
                "total_time": 3000,
                "last_review": "2026-01-15T12:00:00Z",
                "due": "2026-01-20T12:00:00Z",
                "scheduled_days": 5
            }}"#
        )
    }

    fn load(cards: &[(&str, String)]) -> PerformanceData {
        let body: Vec<String> = cards
            .iter()
            .map(|(command, json)| format!("\"{command}\": {json}"))
            .collect();
        serde_json::from_str(&format!("{{{}}}", body.join(","))).unwrap()
    }

    #[test]
    fn test_new_command_defaults() {
        let now = Utc::now();
        let perf = CommandPerformance::new("x".to_string(), now);
        assert_eq!(perf.stability(), 0.0);
        assert_eq!(perf.difficulty(), DEFAULT_DIFFICULTY);
        assert_eq!(perf.reps(), 0);
        assert_eq!(perf.state(), CardState::New);
        assert_eq!(perf.mastery_level(), MasteryLevel::Beginner);
        assert_eq!(perf.retrievability(now), 1.0);
        assert_eq!(perf.avg_time(), Duration::ZERO);
        assert!(perf.memory_state().is_none());
    }

    #[test]
    fn legacy_derived_keys_are_ignored_on_load() {
        let legacy = r#"{
            "command": "x",
            "stability": 5.0,
            "difficulty": 6.0,
            "state": "Review",
            "reps": 3,
            "lapses": 1,
            "attempts": 3,
            "successes": 2,
            "total_time": 3000,
            "avg_time": 99999,
            "last_review": "2026-01-15T12:00:00Z",
            "due": "2026-01-20T12:00:00Z",
            "scheduled_days": 5,
            "mastery_level": "Master",
            "retrievability": null
        }"#;

        let data = load(&[("x", legacy.to_string())]);

        let perf = data.get("x").expect("legacy card loads");
        assert_eq!(perf.command(), "x");
        assert_eq!(perf.avg_time(), Duration::from_millis(1000));
        assert_eq!(perf.mastery_level(), MasteryLevel::Intermediate);
    }

    #[test]
    fn derived_values_are_not_persisted() {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt("h", Duration::from_secs(1), true, Duration::from_secs(1));

        let json = serde_json::to_value(tracker.stats_clone()).unwrap();

        let keys: std::collections::BTreeSet<&str> = json["h"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        for derived in ["command", "avg_time", "mastery_level", "retrievability"] {
            assert!(!keys.contains(derived), "{derived} must not be persisted");
        }
    }

    #[test]
    fn persisted_cards_round_trip() {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt("h", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("dd", Duration::from_secs(9), false, Duration::from_secs(1));

        let json = serde_json::to_string(&tracker.stats_clone()).unwrap();
        let data: PerformanceData = serde_json::from_str(&json).unwrap();

        for command in ["h", "dd"] {
            let before = tracker.performance(command).unwrap();
            let after = data.get(command).unwrap();
            assert_eq!(after.command(), command);
            assert_eq!(after.stability(), before.stability());
            assert_eq!(after.difficulty(), before.difficulty());
            assert_eq!(after.attempts(), before.attempts());
            assert_eq!(after.mastery_level(), before.mastery_level());
            assert_eq!(after.due(), before.due());
        }
    }

    #[test]
    fn invalid_cards_are_dropped_and_the_rest_load() {
        let data = load(&[
            ("h", card_json("5.0", "6.0", 3, 2)),
            ("j", card_json("5.0", "null", 3, 2)),
            ("k", card_json("-1.0", "6.0", 3, 2)),
            ("l", card_json("5.0", "11.0", 3, 2)),
            ("w", card_json("5.0", "6.0", 2, 3)),
            ("b", card_json("\"x\"", "6.0", 3, 2)),
        ]);

        assert_eq!(data.len(), 1);
        assert!(data.contains_key("h"));
    }

    #[test]
    fn review_after_loading_dropped_cards_does_not_panic() {
        let data = load(&[
            ("h", card_json("5.0", "6.0", 3, 2)),
            ("j", card_json("5.0", "null", 3, 2)),
        ]);
        let mut tracker = PerformanceTracker::from_stats(data);

        tracker.record_attempt("h", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("j", Duration::from_secs(1), true, Duration::from_secs(1));

        assert_eq!(tracker.performance("h").unwrap().attempts(), 4);
        assert_eq!(tracker.performance("j").unwrap().attempts(), 1);
    }

    #[test]
    fn mastery_avg_time_and_retrievability_are_derived_on_read() {
        let now: DateTime<Utc> = "2026-01-15T12:00:00Z".parse().unwrap();
        let data = load(&[("h", card_json("40.0", "4.0", 4, 4))]);
        let perf = data.get("h").unwrap();

        assert_eq!(perf.avg_time(), Duration::from_millis(750));
        assert_eq!(perf.mastery_level(), MasteryLevel::Master);
        assert!((perf.retrievability(now) - 1.0).abs() < 1e-6);
        let later = perf.retrievability(now + chrono::Duration::days(100));
        assert!((0.0..1.0).contains(&later));
    }

    #[test]
    fn failed_fsrs_update_counts_the_attempt_and_keeps_the_schedule() {
        let mut broken = CommandPerformance::new("x".to_string(), Utc::now());
        broken.reps = 2;
        broken.state = CardState::Review;
        broken.stability = Stability(f32::NAN);
        let mut tracker =
            PerformanceTracker::from_stats(HashMap::from([("x".into(), broken)]).into());

        let result =
            tracker.try_record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        assert_matches!(result, Err(LearningError::FsrsError(_)));
        let perf = tracker.performance("x").unwrap();
        assert_eq!(perf.attempts(), 1);
        assert_eq!(perf.successes(), 1);
        assert_eq!(perf.reps(), 2);
        assert_eq!(perf.state(), CardState::Review);
    }

    #[test]
    fn out_of_domain_memory_leaves_the_card_unchanged() {
        let now = Utc::now();
        let mut perf = CommandPerformance::new("x".to_string(), now);
        let memory = MemoryState {
            stability: f32::INFINITY,
            difficulty: 5.0,
        };

        let result = perf.update_from_next_state(memory, 3.0, 2, now);

        assert_matches!(result, Err(LearningError::InvalidStability(_)));
        assert_eq!(perf.reps(), 0);
        assert_eq!(perf.state(), CardState::New);
    }

    #[test]
    fn deserialize_drops_non_reviewable_cards_and_keeps_valid_history() {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt("h", Duration::from_secs(1), true, Duration::from_secs(1));
        let mut json = serde_json::to_value(tracker.stats_clone()).unwrap();
        let card = json["h"].clone();
        for junk in ["Escape", "3h", "f=", "!"] {
            json[junk] = card.clone();
        }

        let data: PerformanceData = serde_json::from_value(json).unwrap();

        assert_eq!(data.len(), 1);
        let kept = data.get("h").expect("valid card survives");
        assert_eq!(kept.attempts, 1);
        assert_eq!(kept.reps, 1);
    }

    #[test]
    fn test_perfect_performance_increases_stability() {
        let mut tracker = PerformanceTracker::new();

        // First attempt (perfect)
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        let perf = tracker.performance("x").unwrap();
        assert!(perf.stability() > 0.0);
        assert_eq!(perf.reps, 1);
        assert_matches!(perf.state, CardState::Learning | CardState::Review);

        let first_stability = perf.stability();

        // Second attempt (perfect)
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        let perf = tracker.performance("x").unwrap();
        assert!(perf.stability() > first_stability); // FSRS increases stability
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
        assert_matches!(perf.state, CardState::Relearning | CardState::Learning);
    }

    #[test]
    fn test_difficulty_bounds() {
        let mut tracker = PerformanceTracker::new();

        // Spam failures - difficulty should stay within 0-10
        for _ in 0..100 {
            tracker.record_attempt("x", Duration::from_secs(10), false, Duration::from_secs(1));
        }

        let perf = tracker.performance("x").unwrap();
        assert!(perf.difficulty() >= 0.0);
        assert!(perf.difficulty() <= 10.0);
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

        perf.stability = Stability(5.0);
        perf.difficulty = Difficulty(6.0);
        perf.reps = 1;
        perf.state = CardState::Review;

        let memory = perf.memory_state().unwrap();
        assert_eq!(memory.stability, 5.0);
        assert_eq!(memory.difficulty, 6.0);
    }

    #[test]
    fn test_mastery_level_progression() {
        let mut perf = CommandPerformance::new("x".to_string(), Utc::now());
        assert_eq!(perf.mastery_level(), MasteryLevel::Beginner);

        // Simulate progression through states
        perf.state = CardState::Review;
        perf.stability = Stability(5.0);
        assert_eq!(perf.mastery_level(), MasteryLevel::Intermediate);

        perf.stability = Stability(10.0);
        assert_eq!(perf.mastery_level(), MasteryLevel::Advanced);

        perf.stability = Stability(35.0);
        perf.difficulty = Difficulty(4.0);
        assert_eq!(perf.mastery_level(), MasteryLevel::Master);
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
            prop_assert_eq!(first.stability(), second.stability());
            prop_assert_eq!(first.difficulty(), second.difficulty());
            prop_assert_eq!(first.state, second.state);
            prop_assert_eq!(first.reps, second.reps);
            prop_assert_eq!(first.lapses, second.lapses);
            prop_assert_eq!(first.scheduled_days, second.scheduled_days);
            prop_assert_eq!(first.due, second.due);
            prop_assert_eq!(first.avg_time(), second.avg_time());
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
                    (1.0..=10.0).contains(&perf.difficulty()),
                    "difficulty {} outside FSRS's [1, 10] domain",
                    perf.difficulty()
                );
                // FSRS clamps stability to a strictly positive minimum
                // (fsrs::simulation::S_MIN); zero would make retrievability degenerate.
                prop_assert!(
                    perf.stability() > 0.0,
                    "stability {} must be strictly positive",
                    perf.stability()
                );
                prop_assert!(
                    (0.0..=1.0).contains(&perf.retrievability(clock.now() + chrono::Duration::days(elapsed_days))),
                    "retrievability {} outside the documented [0, 1] probability range",
                    perf.retrievability(clock.now() + chrono::Duration::days(elapsed_days))
                );
            }
        }
    }

    #[test]
    fn test_record_attempt_with_saturated_counters_does_not_panic() {
        let mut perf = CommandPerformance::new("x".to_string(), Utc::now());
        perf.attempts = u32::MAX;
        perf.successes = u32::MAX;
        perf.reps = u32::MAX;
        let mut tracker =
            PerformanceTracker::from_stats(HashMap::from([("x".to_string(), perf)]).into());

        tracker.record_attempt(
            "x",
            std::time::Duration::from_secs(1),
            true,
            std::time::Duration::from_secs(1),
        );

        let perf = tracker.performance("x").expect("entry exists");
        assert_eq!(perf.attempts, u32::MAX);
        assert_eq!(perf.successes, u32::MAX);
    }
}
