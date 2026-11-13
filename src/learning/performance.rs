use chrono::{DateTime, Utc};
use fsrs::{FSRS, MemoryState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use super::LearningError;
use super::traits::ProgressionTier;
use crate::constants::PER_ACTION_BUDGET;
use crate::ids::CommandId;
use crate::time::{Clock, SystemClock};

const DEFAULT_DIFFICULTY: f32 = 5.0; // Mixle difficulty (0-10 scale)
const DEFAULT_DESIRED_RETENTION: f32 = 0.9; // 90% target retention

/// FSRS-6 parameter table. The 21-element array type makes an accidental switch to the
/// 34-element FSRS-7 table a compile error; `fsrs` 7 selects the model by parameter count.
const FSRS_PARAMS: [f32; 21] = fsrs::FSRS6_DEFAULT_PARAMETERS;

static MODEL: LazyLock<FSRS> = LazyLock::new(default_fsrs);

#[expect(
    clippy::expect_used,
    reason = "`FSRS_PARAMS` is the crate's own FSRS-6 default parameter set, so construction cannot fail"
)]
fn default_fsrs() -> FSRS {
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

/// How one attempt at a command ended
///
/// # Examples
///
/// ```
/// use helix_trainer::learning::AttemptOutcome;
///
/// assert!(AttemptOutcome::Succeeded.is_succeeded());
/// assert!(!AttemptOutcome::Failed.is_succeeded());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttemptOutcome {
    /// The command was executed or recalled correctly.
    Succeeded,
    /// The attempt failed or timed out.
    Failed,
}

impl AttemptOutcome {
    /// Whether the attempt succeeded.
    #[must_use]
    pub fn is_succeeded(self) -> bool {
        self == Self::Succeeded
    }
}

/// A command whose mastery level rose after an attempt
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasteryChange {
    /// The command that leveled up.
    pub command: CommandId,
    /// The level before the attempt.
    pub from: MasteryLevel,
    /// The level after the attempt.
    pub to: MasteryLevel,
}

/// FSRS rating of one attempt, ordered from worst to best
///
/// # Examples
///
/// ```
/// use helix_trainer::learning::AttemptOutcome::{Failed, Succeeded};
/// use helix_trainer::learning::performance::{Rating, ReferenceTime};
/// use std::time::Duration;
///
/// let reference = ReferenceTime::SINGLE_ACTION;
/// assert_eq!(Rating::for_attempt(Succeeded, Duration::from_secs(2), reference), Rating::Easy);
/// assert_eq!(Rating::for_attempt(Succeeded, Duration::from_secs(20), reference), Rating::Hard);
/// assert_eq!(Rating::for_attempt(Failed, Duration::ZERO, reference), Rating::Again);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rating {
    /// Failed attempt.
    Again,
    /// Succeeded slowly (more than 3x the reference time).
    Hard,
    /// Succeeded at a normal pace (1.5x to 3x the reference time).
    Good,
    /// Succeeded quickly (up to 1.5x the reference time).
    Easy,
}

impl Rating {
    const EASY_MAX_RATIO: f64 = 1.5;
    const GOOD_MAX_RATIO: f64 = 3.0;

    /// Rates an attempt: a failure is [`Rating::Again`]; a success is never `Again` and gets
    /// worse as `duration` grows relative to `reference`.
    pub fn for_attempt(
        outcome: AttemptOutcome,
        duration: Duration,
        reference: ReferenceTime,
    ) -> Self {
        if !outcome.is_succeeded() {
            return Self::Again;
        }
        let ratio = reference.ratio(duration);
        if ratio <= Self::EASY_MAX_RATIO {
            Self::Easy
        } else if ratio <= Self::GOOD_MAX_RATIO {
            Self::Good
        } else {
            Self::Hard
        }
    }
}

/// Expected time for an attempt, the yardstick a duration is rated against
///
/// Independent of the measured time, so the rating depends on how quickly the player answered.
///
/// # Examples
///
/// ```
/// use helix_trainer::learning::performance::ReferenceTime;
/// use std::num::NonZeroUsize;
/// use std::time::Duration;
///
/// let three_actions = ReferenceTime::for_optimal_actions(NonZeroUsize::new(3).unwrap());
/// assert_eq!(three_actions.as_duration(), Duration::from_secs(9));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceTime(Duration);

impl ReferenceTime {
    /// The reference for a single action, used for review answers.
    pub const SINGLE_ACTION: Self = Self(PER_ACTION_BUDGET);

    /// Wraps an explicit expected duration.
    pub const fn new(expected: Duration) -> Self {
        Self(expected)
    }

    /// The reference for a scenario solved in `optimal_actions` actions: that many action
    /// budgets, saturating on overflow.
    pub fn for_optimal_actions(optimal_actions: NonZeroUsize) -> Self {
        let count = u32::try_from(optimal_actions.get()).unwrap_or(u32::MAX);
        Self(PER_ACTION_BUDGET.saturating_mul(count))
    }

    /// The expected duration.
    pub const fn as_duration(self) -> Duration {
        self.0
    }

    /// `duration` divided by the reference; zero for a zero duration, infinite when only the
    /// reference is zero.
    fn ratio(self, duration: Duration) -> f64 {
        if duration.is_zero() {
            0.0
        } else {
            duration.as_secs_f64() / self.0.as_secs_f64()
        }
    }
}

/// Persisted per-command FSRS data, keyed by review-card id.
///
/// The profile and the tracker exchange this named type instead of a bare `HashMap`, and
/// the on-disk JSON shape stays a plain object. Deserialization drops, rather than fails
/// on, cards whose id is not a canonical [`CommandId`] (saved before typed text
/// and arguments were excluded) and cards that fail validation (non-finite or
/// out-of-range FSRS values, `successes > attempts`); the dropped count is logged.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct PerformanceData(HashMap<CommandId, CommandPerformance>);

impl<'de> Deserialize<'de> for PerformanceData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = HashMap::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut cards = HashMap::with_capacity(raw.len());
        let mut not_reviewable = 0_usize;
        let mut invalid = Vec::new();
        for (key, value) in raw {
            let Some(command) = CommandId::from_card_id(&key) else {
                not_reviewable = not_reviewable.saturating_add(1);
                continue;
            };
            let card = CardRecord::deserialize(value)
                .map_err(|e| e.to_string())
                .and_then(|record| {
                    CommandPerformance::from_record(&record).map_err(|e| e.to_string())
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
    pub fn get(&self, command: &CommandId) -> Option<&CommandPerformance> {
        self.0.get(command)
    }

    /// Whether a record exists for `command`.
    #[must_use]
    pub fn contains_key(&self, command: &CommandId) -> bool {
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
    pub fn iter(&self) -> impl Iterator<Item = (&CommandId, &CommandPerformance)> {
        self.0.iter()
    }
}

impl From<HashMap<CommandId, CommandPerformance>> for PerformanceData {
    fn from(map: HashMap<CommandId, CommandPerformance>) -> Self {
        Self(map)
    }
}

#[expect(
    clippy::implicit_hasher,
    reason = "PerformanceData wraps the default-hasher map, so only that map type can be produced"
)]
impl From<PerformanceData> for HashMap<CommandId, CommandPerformance> {
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
/// `mastery_level`, `retrievability`) or redundant (`command`, the map key) are neither
/// written nor required; older files that still carry them load with those keys ignored.
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
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
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

    fn from_record(record: &CardRecord) -> Result<Self, LearningError> {
        if record.successes > record.attempts {
            return Err(LearningError::InvalidCounters {
                attempts: record.attempts,
                successes: record.successes,
            });
        }
        Ok(Self {
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
        self.total_time
            .checked_div(self.attempts.max(1))
            .unwrap_or_default()
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
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    pub fn retrievability(&self, now: DateTime<Utc>) -> f32 {
        let Some(memory) = self.memory_state() else {
            return 1.0;
        };
        if memory.stability <= 0.0 {
            return 0.0;
        }
        let elapsed_days = now
            .signed_duration_since(self.last_review)
            .num_days()
            .max(0) as f32;
        MODEL.current_retrievability(memory, elapsed_days)
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
            f64::from(self.successes) / f64::from(self.attempts)
        }
    }

    /// Get `MemoryState` for FSRS (None if New)
    pub fn memory_state(&self) -> Option<MemoryState> {
        if self.reps == 0 {
            None
        } else {
            Some(MemoryState {
                stability: self.stability.0,
                difficulty: self.difficulty.0,
                stability_fast: self.stability.0,
            })
        }
    }

    /// Update from FSRS `NextState` and determine card state
    ///
    /// # Errors
    ///
    /// Returns [`LearningError::InvalidStability`] or [`LearningError::InvalidDifficulty`]
    /// when `memory` is out of domain; the card is left unchanged.
    pub fn update_from_next_state(
        &mut self,
        memory: MemoryState,
        interval: f32,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Result<(), LearningError> {
        let stability = Stability::new(memory.stability)?;
        let difficulty = Difficulty::new(memory.difficulty)?;

        // Determine new state based on rating and current state
        self.state = match (self.state, rating) {
            (_, Rating::Again) => {
                self.lapses = self.lapses.saturating_add(1);
                if self.reps > 0 {
                    CardState::Relearning
                } else {
                    CardState::Learning
                }
            }
            (CardState::New | CardState::Learning, Rating::Hard) => CardState::Learning,
            (CardState::Relearning, Rating::Hard) => CardState::Relearning,
            (CardState::Review, Rating::Hard) => CardState::Review,
            (CardState::New | CardState::Learning, Rating::Good | Rating::Easy) => {
                if interval >= 1.0 {
                    CardState::Review
                } else {
                    CardState::Learning
                }
            }
            (CardState::Relearning | CardState::Review, Rating::Good | Rating::Easy) => {
                CardState::Review
            }
        };

        self.stability = stability;
        self.difficulty = difficulty;
        let scheduled_days =
            crate::numeric::to_u32(f64::from(interval), crate::numeric::Rounding::Nearest).max(1);
        self.scheduled_days = scheduled_days;
        self.last_review = now;

        // For first review (reps == 0), make command immediately due
        // This allows users to practice new commands right away
        // Subsequent reviews use FSRS scheduling with 1+ day intervals
        if self.reps == 0 {
            self.due = self.last_review; // Immediately due
        } else {
            self.due = self
                .last_review
                .checked_add_signed(chrono::Duration::days(i64::from(self.scheduled_days)))
                .unwrap_or(DateTime::<Utc>::MAX_UTC);
        }

        self.reps = self.reps.saturating_add(1);
        Ok(())
    }
}

// Helper module for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub(crate) fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
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
    stats: HashMap<CommandId, CommandPerformance>,
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
    /// Used by [`crate::ui::state::ProgressState::with_clocks`] to keep this tracker and its
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

    /// Record an attempt rated against an expected duration, logging an FSRS update failure
    /// instead of returning it.
    ///
    /// For call sites with no recovery path: the attempt is still counted and the schedule
    /// stays unchanged. See [`Self::try_record_attempt`] to observe the error.
    pub fn record_attempt(
        &mut self,
        command: &CommandId,
        duration: Duration,
        outcome: AttemptOutcome,
        optimal_time: Duration,
    ) {
        self.record_attempt_against(command, duration, outcome, ReferenceTime::new(optimal_time));
    }

    /// Like [`Self::record_attempt`] with a typed [`ReferenceTime`].
    pub fn record_attempt_against(
        &mut self,
        command: &CommandId,
        duration: Duration,
        outcome: AttemptOutcome,
        reference: ReferenceTime,
    ) {
        if let Err(error) = self.try_record_attempt(command, duration, outcome, reference) {
            tracing::error!(
                %command,
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
        command: &CommandId,
        duration: Duration,
        outcome: AttemptOutcome,
        reference: ReferenceTime,
    ) -> Result<(), LearningError> {
        // One `now` read shared by card creation, the elapsed-days calculation and the
        // persisted last_review timestamp.
        let now = self.clock.now();

        let perf = self
            .stats
            .entry(command.clone())
            .or_insert_with(|| CommandPerformance::new(now));

        perf.attempts = perf.attempts.saturating_add(1);
        if outcome.is_succeeded() {
            perf.successes = perf.successes.saturating_add(1);
        }
        perf.total_time = perf.total_time.saturating_add(duration);

        let rating = Rating::for_attempt(outcome, duration, reference);
        let elapsed_days = Self::elapsed_days_since_review(perf, now);
        let next_state =
            Self::next_item_state(&self.fsrs, perf.memory_state(), elapsed_days, rating)?;
        perf.update_from_next_state(next_state.memory, next_state.interval, rating, now)
    }

    fn next_item_state(
        fsrs: &FSRS,
        memory_state: Option<MemoryState>,
        elapsed_days: u32,
        rating: Rating,
    ) -> Result<fsrs::ItemState, LearningError> {
        let next_states = fsrs
            .next_states(memory_state, DEFAULT_DESIRED_RETENTION, elapsed_days)
            .map_err(LearningError::FsrsError)?;

        Ok(match rating {
            Rating::Again => next_states.again,
            Rating::Hard => next_states.hard,
            Rating::Good => next_states.good,
            Rating::Easy => next_states.easy,
        })
    }

    fn elapsed_days_since_review(perf: &CommandPerformance, now: DateTime<Utc>) -> u32 {
        if perf.reps == 0 {
            0
        } else {
            u32::try_from(
                now.signed_duration_since(perf.last_review)
                    .num_days()
                    .max(0),
            )
            .unwrap_or(u32::MAX)
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
    pub fn performance(&self, command: &CommandId) -> Option<&CommandPerformance> {
        self.stats.get(command)
    }

    /// Record an attempt and return mastery level change if any
    ///
    /// Returns the [`MasteryChange`] if the mastery level rose
    pub fn record_attempt_with_mastery_change(
        &mut self,
        command: &CommandId,
        duration: Duration,
        outcome: AttemptOutcome,
        reference: ReferenceTime,
    ) -> Option<MasteryChange> {
        // Get old mastery level (Beginner if new command)
        let old_level = self
            .stats
            .get(command)
            .map_or(MasteryLevel::Beginner, CommandPerformance::mastery_level);

        // Record the attempt
        self.record_attempt_against(command, duration, outcome, reference);

        // Get new mastery level
        let new_level = self
            .stats
            .get(command)
            .map(CommandPerformance::mastery_level)?;

        // Return change if level improved
        if new_level.tier_level() > old_level.tier_level() {
            Some(MasteryChange {
                command: command.clone(),
                from: old_level,
                to: new_level,
            })
        } else {
            None
        }
    }

    /// Commands that are hard, unreliable, still at beginner level or have many lapses.
    pub fn weak_commands(&self) -> Vec<CommandId> {
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
    pub fn all_commands(&self) -> Vec<&CommandId> {
        self.stats.keys().collect()
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
    use crate::testing::cid;
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
        let perf = CommandPerformance::new(now);
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

        let perf = data.get(&cid("x")).expect("legacy card loads");
        assert_eq!(perf.avg_time(), Duration::from_millis(1000));
        assert_eq!(perf.mastery_level(), MasteryLevel::Intermediate);
    }

    #[test]
    fn derived_values_are_not_persisted() {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt(
            &cid("h"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );

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
        tracker.record_attempt(
            &cid("h"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("dd"),
            Duration::from_secs(9),
            AttemptOutcome::Failed,
            Duration::from_secs(1),
        );

        let json = serde_json::to_string(&tracker.stats_clone()).unwrap();
        let data: PerformanceData = serde_json::from_str(&json).unwrap();

        for command in ["h", "dd"].map(cid) {
            let before = tracker.performance(&command).unwrap();
            let after = data.get(&command).unwrap();
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
        assert!(data.contains_key(&cid("h")));
    }

    #[test]
    fn due_date_saturates_instead_of_overflowing() {
        let mut card = CommandPerformance::new(Utc::now());
        card.reps = 1;
        let memory = MemoryState {
            stability: 5.0,
            difficulty: 5.0,
            stability_fast: 5.0,
        };

        card.update_from_next_state(memory, 36_500.0, Rating::Good, DateTime::<Utc>::MAX_UTC)
            .unwrap();

        assert_eq!(card.due(), DateTime::<Utc>::MAX_UTC);
        assert_eq!(card.reps(), 2);
    }

    #[test]
    fn review_after_loading_dropped_cards_does_not_panic() {
        let data = load(&[
            ("h", card_json("5.0", "6.0", 3, 2)),
            ("j", card_json("5.0", "null", 3, 2)),
        ]);
        let mut tracker = PerformanceTracker::from_stats(data);

        tracker.record_attempt(
            &cid("h"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("j"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );

        assert_eq!(tracker.performance(&cid("h")).unwrap().attempts(), 4);
        assert_eq!(tracker.performance(&cid("j")).unwrap().attempts(), 1);
    }

    #[test]
    fn mastery_avg_time_and_retrievability_are_derived_on_read() {
        let now: DateTime<Utc> = "2026-01-15T12:00:00Z".parse().unwrap();
        let data = load(&[("h", card_json("40.0", "4.0", 4, 4))]);
        let perf = data.get(&cid("h")).unwrap();

        assert_eq!(perf.avg_time(), Duration::from_millis(750));
        assert_eq!(perf.mastery_level(), MasteryLevel::Master);
        assert!((perf.retrievability(now) - 1.0).abs() < 1e-6);
        let later = perf.retrievability(now + chrono::Duration::days(100));
        assert!((0.0..1.0).contains(&later));
    }

    #[test]
    fn failed_fsrs_update_counts_the_attempt_and_keeps_the_schedule() {
        let mut broken = CommandPerformance::new(Utc::now());
        broken.reps = 2;
        broken.state = CardState::Review;
        broken.stability = Stability(f32::NAN);
        let mut tracker =
            PerformanceTracker::from_stats(HashMap::from([(cid("x"), broken)]).into());

        let result = tracker.try_record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            ReferenceTime::SINGLE_ACTION,
        );

        assert_matches!(result, Err(LearningError::FsrsError(_)));
        let error = result.unwrap_err();
        assert!(
            std::error::Error::source(&error).is_some(),
            "the fsrs error must stay attached as the source"
        );
        let perf = tracker.performance(&cid("x")).unwrap();
        assert_eq!(perf.attempts(), 1);
        assert_eq!(perf.successes(), 1);
        assert_eq!(perf.reps(), 2);
        assert_eq!(perf.state(), CardState::Review);
    }

    #[test]
    fn out_of_domain_memory_leaves_the_card_unchanged() {
        let now = Utc::now();
        let mut perf = CommandPerformance::new(now);
        let memory = MemoryState {
            stability: f32::INFINITY,
            difficulty: 5.0,
            stability_fast: f32::INFINITY,
        };

        let result = perf.update_from_next_state(memory, 3.0, Rating::Good, now);

        assert_matches!(result, Err(LearningError::InvalidStability(_)));
        assert_eq!(perf.reps(), 0);
        assert_eq!(perf.state(), CardState::New);
    }

    #[test]
    fn deserialize_drops_non_reviewable_cards_and_keeps_valid_history() {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt(
            &cid("h"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        let mut json = serde_json::to_value(tracker.stats_clone()).unwrap();
        let card = json["h"].clone();
        for junk in ["Escape", "3h", "f=", "!"] {
            json[junk] = card.clone();
        }

        let data: PerformanceData = serde_json::from_value(json).unwrap();

        assert_eq!(data.len(), 1);
        let kept = data.get(&cid("h")).expect("valid card survives");
        assert_eq!(kept.attempts, 1);
        assert_eq!(kept.reps, 1);
    }

    #[test]
    fn test_perfect_performance_increases_stability() {
        let mut tracker = PerformanceTracker::new();

        // First attempt (perfect)
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );

        let perf = tracker.performance(&cid("x")).unwrap();
        assert!(perf.stability() > 0.0);
        assert_eq!(perf.reps, 1);
        assert_matches!(perf.state, CardState::Learning | CardState::Review);

        let first_stability = perf.stability();

        // Second attempt (perfect)
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );

        let perf = tracker.performance(&cid("x")).unwrap();
        assert!(perf.stability() > first_stability); // FSRS increases stability
        assert_eq!(perf.reps, 2);
    }

    #[test]
    fn test_failure_increases_lapses() {
        let mut tracker = PerformanceTracker::new();

        // Build up some stability
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );

        let lapses_before = tracker.performance(&cid("x")).unwrap().lapses;

        // Fail
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(10),
            AttemptOutcome::Failed,
            Duration::from_secs(1),
        );

        let perf = tracker.performance(&cid("x")).unwrap();
        // Lapse count increases when failing
        assert_eq!(perf.lapses, lapses_before + 1);
        assert_matches!(perf.state, CardState::Relearning | CardState::Learning);
    }

    #[test]
    fn test_difficulty_bounds() {
        let mut tracker = PerformanceTracker::new();

        // Spam failures - difficulty should stay within 0-10
        for _ in 0..100 {
            tracker.record_attempt(
                &cid("x"),
                Duration::from_secs(10),
                AttemptOutcome::Failed,
                Duration::from_secs(1),
            );
        }

        let perf = tracker.performance(&cid("x")).unwrap();
        assert!(perf.difficulty() >= 0.0);
        assert!(perf.difficulty() <= 10.0);
    }

    fn rate(outcome: AttemptOutcome, secs: f64) -> Rating {
        Rating::for_attempt(
            outcome,
            Duration::from_secs_f64(secs),
            ReferenceTime::new(Duration::from_secs(1)),
        )
    }

    #[test]
    fn test_rating_thresholds_are_inclusive() {
        assert_eq!(rate(AttemptOutcome::Succeeded, 0.0), Rating::Easy);
        assert_eq!(rate(AttemptOutcome::Succeeded, 1.5), Rating::Easy);
        assert_eq!(rate(AttemptOutcome::Succeeded, 1.6), Rating::Good);
        assert_eq!(rate(AttemptOutcome::Succeeded, 3.0), Rating::Good);
        assert_eq!(rate(AttemptOutcome::Succeeded, 3.1), Rating::Hard);
        assert_eq!(rate(AttemptOutcome::Succeeded, 3600.0), Rating::Hard);
    }

    #[test]
    fn test_failure_is_always_again() {
        assert_eq!(rate(AttemptOutcome::Failed, 0.0), Rating::Again);
        assert_eq!(rate(AttemptOutcome::Failed, 1.0), Rating::Again);
        assert_eq!(rate(AttemptOutcome::Failed, 1000.0), Rating::Again);
    }

    #[test]
    fn test_slow_success_is_not_again() {
        // Regression for #550: more than 5x the reference time used to rate a success Again.
        assert_eq!(rate(AttemptOutcome::Succeeded, 6.0), Rating::Hard);
    }

    #[test]
    fn test_zero_reference_time_does_not_divide_by_zero() {
        let zero = ReferenceTime::new(Duration::ZERO);
        assert_eq!(
            Rating::for_attempt(AttemptOutcome::Succeeded, Duration::ZERO, zero),
            Rating::Easy
        );
        assert_eq!(
            Rating::for_attempt(AttemptOutcome::Succeeded, Duration::from_secs(1), zero),
            Rating::Hard
        );
    }

    #[test]
    fn test_reference_time_scales_with_optimal_actions() {
        let one = ReferenceTime::for_optimal_actions(NonZeroUsize::MIN);
        let four = ReferenceTime::for_optimal_actions(NonZeroUsize::new(4).unwrap());

        assert_eq!(one, ReferenceTime::SINGLE_ACTION);
        assert_eq!(four.as_duration(), PER_ACTION_BUDGET * 4);
        assert_eq!(
            ReferenceTime::for_optimal_actions(NonZeroUsize::MAX).as_duration(),
            PER_ACTION_BUDGET.saturating_mul(u32::MAX)
        );
    }

    #[test]
    fn test_training_rating_depends_on_actual_time() {
        // Regression for #550: the reference no longer derives from the measured time, so
        // a slow completion rates worse than a quick one.
        let reference = ReferenceTime::for_optimal_actions(NonZeroUsize::new(2).unwrap());
        let quick =
            Rating::for_attempt(AttemptOutcome::Succeeded, Duration::from_secs(6), reference);
        let slow = Rating::for_attempt(
            AttemptOutcome::Succeeded,
            Duration::from_secs(60),
            reference,
        );
        assert_eq!(quick, Rating::Easy);
        assert_eq!(slow, Rating::Hard);
    }

    #[test]
    fn test_memory_state_conversion() {
        let mut perf = CommandPerformance::new(Utc::now());
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
        let mut perf = CommandPerformance::new(Utc::now());
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
                &cid("weak"),
                Duration::from_secs(10),
                AttemptOutcome::Failed,
                Duration::from_secs(1),
            );
        }

        // Add a strong command
        for _ in 0..10 {
            tracker.record_attempt(
                &cid("strong"),
                Duration::from_secs(1),
                AttemptOutcome::Succeeded,
                Duration::from_secs(1),
            );
        }

        let weak = tracker.weak_commands();
        assert!(weak.contains(&cid("weak")));
        // "strong" might still be beginner mastery level initially, so don't assert it's not weak
    }

    #[test]
    fn test_record_attempt_honors_injected_clock() {
        let clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
        let mut tracker = PerformanceTracker::with_clock(clock.clone());

        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        let first_last_review = tracker.performance(&cid("x")).unwrap().last_review;
        assert_eq!(first_last_review, clock.now());

        clock.advance_days(3);
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        let second_last_review = tracker.performance(&cid("x")).unwrap().last_review;
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

        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("yy"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("p"),
            Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );

        let all = tracker.all_commands();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&&cid("x")));
        assert!(all.contains(&&cid("yy")));
        assert!(all.contains(&&cid("p")));
    }

    fn solid_card() -> CommandPerformance {
        let mut card = CommandPerformance::new(Utc::now());
        card.state = CardState::Review;
        card.stability = Stability(40.0);
        card.difficulty = Difficulty(4.0);
        card.reps = 5;
        card.attempts = 10;
        card.successes = 10;
        card
    }

    fn is_weak(card: CommandPerformance) -> bool {
        let tracker = PerformanceTracker::from_stats(HashMap::from([(cid("x"), card)]).into());
        tracker.weak_commands().contains(&cid("x"))
    }

    #[test]
    fn weak_commands_skips_a_solid_card() {
        assert!(!is_weak(solid_card()));
    }

    #[test]
    fn weak_commands_difficulty_threshold_is_exclusive() {
        let mut card = solid_card();
        card.difficulty = Difficulty(7.0);
        assert!(!is_weak(card.clone()));
        card.difficulty = Difficulty(7.5);
        assert!(is_weak(card));
    }

    #[test]
    fn weak_commands_success_rate_threshold_is_exclusive() {
        let mut card = solid_card();
        card.successes = 7;
        assert!(!is_weak(card.clone()));
        card.successes = 6;
        assert!(is_weak(card));
    }

    #[test]
    fn weak_commands_flags_beginner_level() {
        let mut card = solid_card();
        card.state = CardState::Learning;
        assert!(is_weak(card));
    }

    #[test]
    fn weak_commands_lapse_threshold_is_exclusive() {
        let mut card = solid_card();
        card.lapses = 2;
        assert!(!is_weak(card.clone()));
        card.lapses = 3;
        assert!(is_weak(card));
    }

    #[test]
    fn mastery_thresholds_are_exclusive() {
        let mut card = solid_card();
        card.stability = Stability(30.0);
        assert_eq!(card.mastery_level(), MasteryLevel::Advanced);
        card.stability = Stability(30.5);
        assert_eq!(card.mastery_level(), MasteryLevel::Master);
        card.difficulty = Difficulty(5.0);
        assert_eq!(card.mastery_level(), MasteryLevel::Advanced);
        card.stability = Stability(7.0);
        assert_eq!(card.mastery_level(), MasteryLevel::Intermediate);
        card.stability = Stability(7.5);
        assert_eq!(card.mastery_level(), MasteryLevel::Advanced);
    }

    #[test]
    fn mastery_level_presentation_and_tiers() {
        let levels = [
            (MasteryLevel::Beginner, "🔰", 0, false),
            (MasteryLevel::Intermediate, "📚", 1, false),
            (MasteryLevel::Advanced, "⭐", 2, false),
            (MasteryLevel::Master, "🏆", 3, true),
        ];
        for (level, emoji, tier, max) in levels {
            assert_eq!(level.emoji(), emoji);
            assert_eq!(level.tier_level(), tier);
            assert_eq!(level.is_max_tier(), max);
        }
    }

    #[test]
    fn performance_data_accessors_reflect_contents() {
        let empty = PerformanceData::default();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(!empty.contains_key(&cid("h")));
        assert_eq!(empty.iter().count(), 0);

        let data = load(&[
            ("h", card_json("5.0", "6.0", 3, 2)),
            ("j", card_json("5.0", "6.0", 3, 2)),
        ]);
        assert!(!data.is_empty());
        assert_eq!(data.len(), 2);
        assert!(data.contains_key(&cid("h")));
        assert!(!data.contains_key(&cid("k")));
        assert_eq!(data.iter().count(), 2);
    }

    #[test]
    fn loaded_card_exposes_persisted_counters() {
        let data = load(&[("h", card_json("5.0", "6.0", 4, 3))]);
        let perf = data.get(&cid("h")).unwrap();
        assert_eq!(perf.lapses(), 1);
        assert_eq!(perf.total_time(), Duration::from_secs(3));
        assert_eq!(perf.attempts(), 4);
        assert_eq!(perf.successes(), 3);
    }

    #[test]
    fn record_attempt_accumulates_counters_and_time() {
        let mut tracker = PerformanceTracker::new();
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(2),
            AttemptOutcome::Succeeded,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(3),
            AttemptOutcome::Failed,
            Duration::from_secs(1),
        );

        let perf = tracker.performance(&cid("x")).unwrap();
        assert_eq!(perf.attempts(), 2);
        assert_eq!(perf.successes(), 1);
        assert_eq!(perf.total_time(), Duration::from_secs(5));
        assert_eq!(perf.avg_time(), Duration::from_millis(2500));
    }

    fn memory() -> MemoryState {
        MemoryState {
            stability: 5.0,
            difficulty: 5.0,
            stability_fast: 5.0,
        }
    }

    #[test]
    fn again_counts_a_lapse_and_picks_learning_or_relearning() {
        let now = Utc::now();
        let mut fresh = CommandPerformance::new(now);
        fresh
            .update_from_next_state(memory(), 1.0, Rating::Again, now)
            .unwrap();
        assert_eq!(fresh.state(), CardState::Learning);
        assert_eq!(fresh.lapses(), 1);

        let mut seasoned = solid_card();
        seasoned
            .update_from_next_state(memory(), 1.0, Rating::Again, now)
            .unwrap();
        assert_eq!(seasoned.state(), CardState::Relearning);
        assert_eq!(seasoned.lapses(), 1);
        assert_eq!(seasoned.reps(), 6);
    }

    #[test]
    fn good_review_needs_a_day_of_interval_to_leave_learning() {
        let now = Utc::now();
        let mut short = CommandPerformance::new(now);
        short
            .update_from_next_state(memory(), 0.5, Rating::Good, now)
            .unwrap();
        assert_eq!(short.state(), CardState::Learning);

        let mut long = CommandPerformance::new(now);
        long.update_from_next_state(memory(), 1.0, Rating::Good, now)
            .unwrap();
        assert_eq!(long.state(), CardState::Review);
    }

    #[test]
    fn hard_keeps_the_current_state_family() {
        let now = Utc::now();
        for (from, to) in [
            (CardState::New, CardState::Learning),
            (CardState::Learning, CardState::Learning),
            (CardState::Relearning, CardState::Relearning),
            (CardState::Review, CardState::Review),
        ] {
            let mut card = CommandPerformance::new(now);
            card.state = from;
            card.update_from_next_state(memory(), 3.0, Rating::Hard, now)
                .unwrap();
            assert_eq!(card.state(), to, "from {from:?}");
        }
    }

    #[test]
    fn update_schedules_the_due_date_from_the_interval() {
        let now: DateTime<Utc> = "2026-01-15T12:00:00Z".parse().unwrap();
        let mut first = CommandPerformance::new(now);
        first
            .update_from_next_state(memory(), 4.0, Rating::Good, now)
            .unwrap();
        assert_eq!(first.due(), now);
        assert_eq!(first.scheduled_days(), 4);

        let mut again = solid_card();
        again
            .update_from_next_state(memory(), 4.4, Rating::Good, now)
            .unwrap();
        assert_eq!(again.scheduled_days(), 4);
        assert_eq!(again.due(), now + chrono::Duration::days(4));
        assert_eq!(again.last_review(), now);

        let mut tiny = solid_card();
        tiny.update_from_next_state(memory(), 0.1, Rating::Good, now)
            .unwrap();
        assert_eq!(tiny.scheduled_days(), 1);
    }

    #[test]
    fn elapsed_days_counts_whole_days_only_after_the_first_review() {
        let now: DateTime<Utc> = "2026-01-15T12:00:00Z".parse().unwrap();
        let mut card = CommandPerformance::new(now - chrono::Duration::days(3));
        assert_eq!(PerformanceTracker::elapsed_days_since_review(&card, now), 0);

        card.reps = 1;
        assert_eq!(PerformanceTracker::elapsed_days_since_review(&card, now), 3);
        assert_eq!(
            PerformanceTracker::elapsed_days_since_review(&card, now - chrono::Duration::days(10)),
            0
        );
    }

    #[test]
    fn mastery_change_is_reported_only_on_improvement() {
        let mut low = solid_card();
        low.stability = Stability(3.0);
        low.reps = 1;
        let mut tracker = PerformanceTracker::from_stats(HashMap::from([(cid("l"), low)]).into());
        let unchanged = tracker.record_attempt_with_mastery_change(
            &cid("l"),
            Duration::from_secs(60),
            AttemptOutcome::Failed,
            ReferenceTime::SINGLE_ACTION,
        );
        assert!(unchanged.is_none());

        let mut tracker = PerformanceTracker::new();
        let promoted = (0..6).find_map(|_| {
            tracker.record_attempt_with_mastery_change(
                &cid("x"),
                Duration::from_millis(100),
                AttemptOutcome::Succeeded,
                ReferenceTime::SINGLE_ACTION,
            )
        });
        let change = promoted.expect("repeated quick successes promote the card");
        assert_eq!(change.command, cid("x"));
        assert!(change.to.tier_level() > change.from.tier_level());
    }

    /// Strategy for one simulated review: (success, duration, optimal duration, days
    /// elapsed since the previous review). `elapsed_days` ranges up to a year so both
    /// properties exercise FSRS's primary scheduling input (`days_elapsed` in
    /// [`PerformanceTracker::calculate_next_fsrs_state`]), not just same-instant reviews.
    fn outcome_strategy() -> impl Strategy<Value = AttemptOutcome> {
        prop_oneof![
            Just(AttemptOutcome::Succeeded),
            Just(AttemptOutcome::Failed)
        ]
    }

    fn attempt_strategy() -> impl Strategy<Value = (AttemptOutcome, u64, u64, i64)> {
        (outcome_strategy(), 1u64..90, 1u64..90, 0i64..365)
    }

    proptest! {
        #[test]
        fn prop_rating_is_monotonic_in_duration(
            outcome in outcome_strategy(),
            reference_ms in 0u64..100_000,
            a_ms in 0u64..10_000_000,
            b_ms in 0u64..10_000_000,
        ) {
            let reference = ReferenceTime::new(Duration::from_millis(reference_ms));
            let (shorter, longer) = (a_ms.min(b_ms), a_ms.max(b_ms));

            let quick = Rating::for_attempt(outcome, Duration::from_millis(shorter), reference);
            let slow = Rating::for_attempt(outcome, Duration::from_millis(longer), reference);

            prop_assert!(slow <= quick, "{slow:?} after {longer} ms beat {quick:?} after {shorter} ms");
        }

        #[test]
        fn prop_success_is_never_again(
            reference_ms in 0u64..100_000,
            duration_ms in any::<u64>(),
        ) {
            let rating = Rating::for_attempt(
                AttemptOutcome::Succeeded,
                Duration::from_millis(duration_ms),
                ReferenceTime::new(Duration::from_millis(reference_ms)),
            );

            prop_assert_ne!(rating, Rating::Again);
        }

        #[test]
        fn prop_fsrs_state_transition_is_deterministic(
            attempts in prop::collection::vec(attempt_strategy(), 1..12),
        ) {
            let run = || {
                let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
                let mut tracker = PerformanceTracker::with_clock(clock.clone());
                for &(outcome, duration_secs, optimal_secs, elapsed_days) in &attempts {
                    clock.advance_days(elapsed_days);
                    tracker.record_attempt(&cid("x"),
                        Duration::from_secs(duration_secs),
                        outcome,
                        Duration::from_secs(optimal_secs),
                    );
                }
                tracker.performance(&cid("x")).unwrap().clone()
            };

            let first = run();
            let second = run();

            // Same starting state + same rating sequence + same simulated elapsed
            // time between reviews must always produce the same resulting FSRS
            // state -- no hidden randomness or wall-clock reads.
            prop_assert_eq!(first.stability().to_bits(), second.stability().to_bits());
            prop_assert_eq!(first.difficulty().to_bits(), second.difficulty().to_bits());
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

            for &(outcome, duration_secs, optimal_secs, elapsed_days) in &attempts {
                clock.advance_days(elapsed_days);
                tracker.record_attempt(&cid("x"),
                    Duration::from_secs(duration_secs),
                    outcome,
                    Duration::from_secs(optimal_secs),
                );

                let perf = tracker.performance(&cid("x")).unwrap();
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
        let mut perf = CommandPerformance::new(Utc::now());
        perf.attempts = u32::MAX;
        perf.successes = u32::MAX;
        perf.reps = u32::MAX;
        let mut tracker = PerformanceTracker::from_stats(HashMap::from([(cid("x"), perf)]).into());

        tracker.record_attempt(
            &cid("x"),
            std::time::Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            std::time::Duration::from_secs(1),
        );

        let perf = tracker.performance(&cid("x")).expect("entry exists");
        assert_eq!(perf.attempts, u32::MAX);
        assert_eq!(perf.successes, u32::MAX);
    }
}
