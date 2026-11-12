//! Daily challenge progress persisted with the profile
//!
//! Attempt limits and best-score tracking for the daily challenge; the
//! scenario selection itself lives in `minigame::challenge`.

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::constants::{CHALLENGE_MAX_ATTEMPTS, CHALLENGE_SCENARIO_COUNT};

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
    On(NaiveDate),
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
        Ok(NaiveDate::deserialize(value).map_or(Self::Unreadable, Self::On))
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
    pub last_challenge_date: ChallengeDay,

    /// Attempts used today (0-3)
    pub attempts_used_today: u8,

    /// Best score achieved today
    pub best_score_today: u64,

    /// Best scenarios completed today (out of 10)
    pub best_scenarios_today: u8,

    /// All-time best challenge score
    pub all_time_best_score: u64,

    /// All-time best scenarios completed in a single challenge
    pub all_time_best_scenarios: u8,

    /// Total challenges attempted (lifetime)
    pub total_challenges_attempted: u32,

    /// Total challenges completed (all 10 scenarios)
    pub total_challenges_completed: u32,
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
        Self {
            last_challenge_date,
            attempts_used_today: wire.attempts_used_today,
            best_score_today: wire.best_score_today,
            best_scenarios_today: wire.best_scenarios_today,
            all_time_best_score: wire.all_time_best_score,
            all_time_best_scenarios: wire.all_time_best_scenarios,
            total_challenges_attempted: wire.total_challenges_attempted,
            total_challenges_completed: wire.total_challenges_completed,
        }
    }
}

impl ChallengeProgress {
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
    pub fn can_attempt(&self, today: NaiveDate) -> bool {
        if self.is_new_day(today) {
            return true;
        }

        self.attempts_used_today < CHALLENGE_MAX_ATTEMPTS
    }

    /// Get remaining attempts for today
    pub fn attempts_remaining(&self, today: NaiveDate) -> u8 {
        if self.is_new_day(today) {
            return CHALLENGE_MAX_ATTEMPTS;
        }

        CHALLENGE_MAX_ATTEMPTS.saturating_sub(self.attempts_used_today)
    }

    /// Start a new attempt, resetting if new day
    pub fn start_attempt(&mut self, today: NaiveDate) {
        // Reset for new day
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

    /// Check whether the calendar day has rolled over since `last_challenge_date`
    fn is_new_day(&self, today: NaiveDate) -> bool {
        match self.last_challenge_date {
            ChallengeDay::NotStarted => true,
            ChallengeDay::On(day) => day != today,
            ChallengeDay::Unreadable => false,
        }
    }

    /// Counts a [`ChallengeDay::Unreadable`] day as `today`, keeping the used
    /// attempts and best-today score instead of refilling the allowance.
    pub fn settle_unreadable_day(&mut self, today: NaiveDate) {
        if self.last_challenge_date == ChallengeDay::Unreadable {
            self.last_challenge_date = ChallengeDay::On(today);
        }
    }

    /// Check if progress is for the given date
    pub fn is_today(&self, today: NaiveDate) -> bool {
        !self.is_new_day(today)
    }
}
