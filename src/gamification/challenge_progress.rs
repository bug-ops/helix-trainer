//! Daily challenge progress persisted with the profile
//!
//! Attempt limits and best-score tracking for the daily challenge; the
//! scenario selection itself lives in `minigame::challenge`.

use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize};

use crate::constants::{CHALLENGE_MAX_ATTEMPTS, CHALLENGE_SCENARIO_COUNT};

/// Tracks player's progress on daily challenges.
///
/// Persisted to `UserProfile` for cross-session tracking.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChallengeProgress {
    /// Date of current/last challenge; serialized as `YYYY-MM-DD`.
    ///
    /// A malformed stored value loads as `None` so one bad field does not fail
    /// the whole profile load.
    #[serde(default, deserialize_with = "lenient_date")]
    pub last_challenge_date: Option<NaiveDate>,

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

fn lenient_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    // Buffer as a Value first so a mismatched shape is fully consumed before being discarded.
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(NaiveDate::deserialize(value).ok())
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
            self.last_challenge_date = Some(today);
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
        self.last_challenge_date != Some(today)
    }

    /// Check if progress is for the given date
    pub fn is_today(&self, today: NaiveDate) -> bool {
        !self.is_new_day(today)
    }
}
