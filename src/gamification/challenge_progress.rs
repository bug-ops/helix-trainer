//! Daily challenge progress persisted with the profile
//!
//! Attempt limits and best-score tracking for the daily challenge; the
//! scenario selection itself lives in `minigame::challenge`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::constants::{CHALLENGE_MAX_ATTEMPTS, CHALLENGE_SCENARIO_COUNT};
use crate::time::PlayerDay;

/// The calendar day a daily challenge was last attempted.
///
/// `Unreadable` keeps a malformed stored date distinct from "never played" so
/// that it never refills the attempt allowance. It serializes as `null`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChallengeDay {
    /// No challenge has been attempted yet.
    #[default]
    NotStarted,
    /// The last attempt happened on this day.
    On(PlayerDay),
    /// The stored day could not be read; counters are kept and never reset.
    Unreadable,
}

impl Serialize for ChallengeDay {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::On(date) => date.serialize(serializer),
            Self::NotStarted | Self::Unreadable => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for ChallengeDay {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Buffer as a Value first so a mismatched shape is fully consumed before being discarded.
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_null() {
            return Ok(Self::NotStarted);
        }
        Ok(PlayerDay::deserialize(value).map_or(Self::Unreadable, Self::On))
    }
}

/// Tracks player's progress on daily challenges.
///
/// Persisted to `UserProfile` for cross-session tracking.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(from = "ChallengeProgressWire")]
pub struct ChallengeProgress {
    /// Day of the current/last challenge; serialized as `YYYY-MM-DD` or `null`.
    ///
    /// A malformed stored value, or a missing one with used attempts, loads as
    /// [`ChallengeDay::Unreadable`] so one bad field neither fails the profile
    /// load nor resets the attempt limit.
    pub(super) last_challenge_date: ChallengeDay,

    /// Attempts used today (0-3)
    pub(super) attempts_used_today: u8,

    /// Best score achieved today
    pub(super) best_score_today: u64,

    /// Best scenarios completed today (out of 10)
    pub(super) best_scenarios_today: u8,

    /// All-time best challenge score
    pub(super) all_time_best_score: u64,

    /// All-time best scenarios completed in a single challenge
    pub(super) all_time_best_scenarios: u8,

    /// Total challenges attempted (lifetime)
    pub(super) total_challenges_attempted: u32,

    /// Total challenges completed (all 10 scenarios)
    pub(super) total_challenges_completed: u32,
}

#[derive(Deserialize)]
struct ChallengeProgressWire {
    #[serde(default)]
    last_challenge_date: ChallengeDay,
    attempts_used_today: u8,
    best_score_today: u64,
    best_scenarios_today: u8,
    all_time_best_score: u64,
    all_time_best_scenarios: u8,
    total_challenges_attempted: u32,
    total_challenges_completed: u32,
}

impl From<ChallengeProgressWire> for ChallengeProgress {
    fn from(wire: ChallengeProgressWire) -> Self {
        let last_challenge_date = match wire.last_challenge_date {
            ChallengeDay::NotStarted if wire.attempts_used_today > 0 => ChallengeDay::Unreadable,
            day => day,
        };
        // A stored value that breaks a bound is repaired and logged, never rejected; the
        // attempts stay spent (fail-closed).
        let attempts_used_today = if wire.attempts_used_today > CHALLENGE_MAX_ATTEMPTS {
            tracing::warn!(
                stored = wire.attempts_used_today,
                max = CHALLENGE_MAX_ATTEMPTS,
                "Challenge attempts above the limit; clamped"
            );
            CHALLENGE_MAX_ATTEMPTS
        } else {
            wire.attempts_used_today
        };
        let all_time_best_score = repaired_max(
            "all_time_best_score",
            wire.all_time_best_score,
            wire.best_score_today,
        );
        let all_time_best_scenarios = repaired_max(
            "all_time_best_scenarios",
            wire.all_time_best_scenarios,
            wire.best_scenarios_today,
        );
        let total_challenges_attempted = repaired_max(
            "total_challenges_attempted",
            wire.total_challenges_attempted,
            wire.total_challenges_completed,
        );
        Self {
            last_challenge_date,
            attempts_used_today,
            best_score_today: wire.best_score_today,
            best_scenarios_today: wire.best_scenarios_today,
            all_time_best_score,
            all_time_best_scenarios,
            total_challenges_attempted,
            total_challenges_completed: wire.total_challenges_completed,
        }
    }
}

/// `stored` raised to `floor`, logging the repair when it had to move.
fn repaired_max<T: Ord + Copy + std::fmt::Display>(field: &str, stored: T, floor: T) -> T {
    if stored >= floor {
        return stored;
    }
    tracing::warn!(field, %stored, %floor, "Challenge progress below its bound; repaired");
    floor
}

impl ChallengeProgress {
    /// Day of the current or last challenge.
    #[must_use]
    pub fn last_challenge_date(&self) -> ChallengeDay {
        self.last_challenge_date
    }

    /// Attempts used today; never above the attempt limit.
    #[must_use]
    pub fn attempts_used_today(&self) -> u8 {
        self.attempts_used_today
    }

    /// Best score achieved today.
    #[must_use]
    pub fn best_score_today(&self) -> u64 {
        self.best_score_today
    }

    /// Most scenarios completed in one challenge today.
    #[must_use]
    pub fn best_scenarios_today(&self) -> u8 {
        self.best_scenarios_today
    }

    /// All-time best challenge score; never below today's best.
    #[must_use]
    pub fn all_time_best_score(&self) -> u64 {
        self.all_time_best_score
    }

    /// All-time most scenarios completed in one challenge.
    #[must_use]
    pub fn all_time_best_scenarios(&self) -> u8 {
        self.all_time_best_scenarios
    }

    /// Challenges attempted in total; never below the completed count.
    #[must_use]
    pub fn total_challenges_attempted(&self) -> u32 {
        self.total_challenges_attempted
    }

    /// Challenges completed in total (all scenarios).
    #[must_use]
    pub fn total_challenges_completed(&self) -> u32 {
        self.total_challenges_completed
    }

    /// Create a new empty challenge progress.
    ///
    /// Note: This is identical to `Default::default()` but provides a conventional
    /// constructor for explicit initialization. Using `new()` signals intentional
    /// creation of an empty progress state, while `Default` is typically used
    /// for struct field initialization in derives.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if player can attempt today's challenge
    pub fn can_attempt(&self, today: PlayerDay) -> bool {
        if self.is_new_day(today) {
            return true;
        }

        self.attempts_used_today < CHALLENGE_MAX_ATTEMPTS
    }

    /// Get remaining attempts for today
    pub fn attempts_remaining(&self, today: PlayerDay) -> u8 {
        if self.is_new_day(today) {
            return CHALLENGE_MAX_ATTEMPTS;
        }

        CHALLENGE_MAX_ATTEMPTS.saturating_sub(self.attempts_used_today)
    }

    /// Start a new attempt, resetting if new day
    pub fn start_attempt(&mut self, today: PlayerDay) {
        // Reset for new day; a stored day at or after today is today (never rewound)
        if self.is_new_day(today) {
            self.last_challenge_date = ChallengeDay::On(today);
            self.attempts_used_today = 0;
            self.best_score_today = 0;
            self.best_scenarios_today = 0;
        }

        self.attempts_used_today = self.attempts_used_today.saturating_add(1);
        self.total_challenges_attempted = self.total_challenges_attempted.saturating_add(1);
    }

    /// Record attempt result
    pub fn record_result(&mut self, score: u64, scenarios_completed: u8) {
        // Update today's best
        if score > self.best_score_today {
            self.best_score_today = score;
        }
        if scenarios_completed > self.best_scenarios_today {
            self.best_scenarios_today = scenarios_completed;
        }

        // Update all-time best
        if score > self.all_time_best_score {
            self.all_time_best_score = score;
        }
        if scenarios_completed > self.all_time_best_scenarios {
            self.all_time_best_scenarios = scenarios_completed;
        }

        // Track completions
        if usize::from(scenarios_completed) >= CHALLENGE_SCENARIO_COUNT {
            self.total_challenges_completed = self.total_challenges_completed.saturating_add(1);
        }
    }

    /// Check whether today is strictly after `last_challenge_date`
    fn is_new_day(&self, today: PlayerDay) -> bool {
        match self.last_challenge_date {
            ChallengeDay::NotStarted => true,
            ChallengeDay::On(day) => today > day,
            ChallengeDay::Unreadable => false,
        }
    }

    /// Counts a [`ChallengeDay::Unreadable`] day as `today`, keeping the used
    /// attempts and best-today score instead of refilling the allowance.
    pub fn settle_unreadable_day(&mut self, today: PlayerDay) {
        if self.last_challenge_date == ChallengeDay::Unreadable {
            self.last_challenge_date = ChallengeDay::On(today);
        }
    }

    /// Check if progress is for the given date
    pub fn is_today(&self, today: PlayerDay) -> bool {
        !self.is_new_day(today)
    }
}
/// Direct writes of the bounded values, for tests that need a specific state.
#[cfg(any(test, feature = "test-util"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
impl ChallengeProgress {
    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_last_challenge_date(&mut self, value: ChallengeDay) {
        self.last_challenge_date = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_attempts_used_today(&mut self, value: u8) {
        self.attempts_used_today = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_best_score_today(&mut self, value: u64) {
        self.best_score_today = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_best_scenarios_today(&mut self, value: u8) {
        self.best_scenarios_today = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_all_time_best_score(&mut self, value: u64) {
        self.all_time_best_score = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_all_time_best_scenarios(&mut self, value: u8) {
        self.all_time_best_scenarios = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_total_challenges_attempted(&mut self, value: u32) {
        self.total_challenges_attempted = value;
    }

    /// Set the stored value, bypassing the bounds the recording methods keep.
    pub fn set_total_challenges_completed(&mut self, value: u32) {
        self.total_challenges_completed = value;
    }
}
