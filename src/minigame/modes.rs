//! Game mode configurations for mini-game sessions
//!
//! This module defines the different game modes available in the mini-game system:
//! - Arcade: Classic 60-second timed session with 3 lives
//! - Survival: Endless mode with 1 life and escalating difficulty
//! - Challenge: Daily puzzle with fixed scenarios and limited attempts

use std::time::Duration;

use super::DifficultyLevel;
use crate::constants::{
    CHALLENGE_LIVES_PER_ATTEMPT, CHALLENGE_MAX_ATTEMPTS, CHALLENGE_SCENARIO_COUNT,
    CHALLENGE_TIME_PER_SCENARIO, EXTRA_LIFE_SCORE_MILESTONE, SURVIVAL_BASE_TIME, SURVIVAL_MIN_TIME,
    SURVIVAL_TIME_DECREASE_PER_LEVEL,
};

/// Game mode configuration for mini-game sessions.
///
/// Each variant carries the specific configuration for that mode,
/// allowing the session to apply mode-specific rules uniformly.
#[derive(Debug, Clone)]
pub enum MiniGameMode {
    /// Classic arcade mode: 60-second timed session, 3 lives, score attack
    Arcade(ArcadeConfig),

    /// Survival mode: endless, 1 life, escalating per-scenario time pressure
    Survival(SurvivalConfig),

    /// Challenge mode: daily puzzle, fixed scenarios, limited attempts
    Challenge(ChallengeConfig),
}

impl MiniGameMode {
    /// Get the starting number of lives for this mode
    pub fn starting_lives(&self) -> u8 {
        match self {
            Self::Arcade(c) => c.starting_lives,
            Self::Survival(_) => 1,
            Self::Challenge(c) => c.lives_per_attempt,
        }
    }

    /// Get session duration if applicable
    ///
    /// Single source of truth for whether a mode has a session time limit —
    /// see [`Self::has_session_timer`], which derives from this.
    pub fn session_duration(&self) -> Option<Duration> {
        match self {
            Self::Arcade(c) => Some(c.session_duration),
            Self::Survival(_) | Self::Challenge(_) => None,
        }
    }

    /// Check if this mode has a session time limit
    pub fn has_session_timer(&self) -> bool {
        self.session_duration().is_some()
    }

    /// Check if this is Arcade mode
    pub fn is_arcade(&self) -> bool {
        match self {
            Self::Arcade(_) => true,
            Self::Survival(_) | Self::Challenge(_) => false,
        }
    }

    /// Check if this is Survival mode
    pub fn is_survival(&self) -> bool {
        match self {
            Self::Survival(_) => true,
            Self::Arcade(_) | Self::Challenge(_) => false,
        }
    }

    /// Check if this is Challenge mode
    pub fn is_challenge(&self) -> bool {
        match self {
            Self::Challenge(_) => true,
            Self::Arcade(_) | Self::Survival(_) => false,
        }
    }
}

impl Default for MiniGameMode {
    fn default() -> Self {
        Self::Arcade(ArcadeConfig::default())
    }
}

/// Configuration for Arcade mode (classic timed gameplay).
#[derive(Debug, Clone)]
pub struct ArcadeConfig {
    /// Total session duration (default: 60 seconds)
    pub session_duration: Duration,

    /// Starting number of lives (default: 3)
    pub starting_lives: u8,

    /// Maximum lives that can be accumulated (default: 5)
    pub max_lives: u8,

    /// Score milestone for extra life (default: 5000)
    pub extra_life_milestone: u64,
}

impl Default for ArcadeConfig {
    fn default() -> Self {
        Self {
            session_duration: Duration::from_mins(1),
            starting_lives: 3,
            max_lives: 5,
            extra_life_milestone: EXTRA_LIFE_SCORE_MILESTONE,
        }
    }
}

/// Configuration for Survival mode (endless with escalating difficulty).
///
/// In Survival mode:
/// - No session time limit (play until you fail)
/// - Single life (one timeout/failure = game over)
/// - Per-scenario time limit decreases as level increases
/// - Difficulty escalates continuously
#[derive(Debug, Clone)]
pub struct SurvivalConfig {
    /// Base time limit per scenario at level 1 (default: 10 seconds)
    pub base_time_per_scenario: Duration,

    /// Minimum time limit per scenario (default: 3 seconds)
    pub min_time_per_scenario: Duration,

    /// Time decrease per difficulty level (default: 500ms)
    pub time_decrease_per_level: Duration,

    /// Time decrease formula: linear vs exponential
    pub decrease_mode: TimeDecreaseMode,
}

/// How time limit decreases with level in Survival mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeDecreaseMode {
    /// Linear: time = base - (level * `decrease_per_level`)
    #[default]
    Linear,
    /// Exponential: time = base * 0.95^level (smoother curve)
    Exponential,
}

impl Default for SurvivalConfig {
    fn default() -> Self {
        Self {
            base_time_per_scenario: SURVIVAL_BASE_TIME,
            min_time_per_scenario: SURVIVAL_MIN_TIME,
            time_decrease_per_level: SURVIVAL_TIME_DECREASE_PER_LEVEL,
            decrease_mode: TimeDecreaseMode::Linear,
        }
    }
}

impl SurvivalConfig {
    /// Calculate time limit for a given difficulty level
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::minigame::{DifficultyLevel, SurvivalConfig};
    ///
    /// let config = SurvivalConfig::default();
    /// let time = config.time_limit_for_level(DifficultyLevel::MIN);
    /// assert_eq!(time.as_secs(), 10); // Base time at level 1
    ///
    /// let time = config.time_limit_for_level(DifficultyLevel::new(5).unwrap());
    /// assert!(time < config.base_time_per_scenario);
    /// ```
    pub fn time_limit_for_level(&self, level: DifficultyLevel) -> Duration {
        let steps = level.get().saturating_sub(1);
        match self.decrease_mode {
            TimeDecreaseMode::Linear => {
                let decrease = self.time_decrease_per_level * u32::from(steps);
                let time = self.base_time_per_scenario.saturating_sub(decrease);
                time.max(self.min_time_per_scenario)
            }
            TimeDecreaseMode::Exponential => {
                let factor = 0.95_f64.powi(i32::from(steps));
                let time_secs = self.base_time_per_scenario.as_secs_f64() * factor;
                let time = Duration::from_secs_f64(time_secs);
                time.max(self.min_time_per_scenario)
            }
        }
    }
}

/// Configuration for Challenge mode (daily puzzle).
///
/// In Challenge mode:
/// - Fixed set of scenarios selected by date seed
/// - Same puzzle for all players on the same day
/// - Limited attempts per day (default: 3)
/// - Standard difficulty (no adaptive scaling)
/// - Best score of the day is tracked
#[derive(Debug, Clone)]
pub struct ChallengeConfig {
    /// Number of scenarios in the daily challenge (default: 10)
    pub scenario_count: usize,

    /// Maximum attempts allowed per day (default: 3)
    pub max_attempts_per_day: u8,

    /// Lives per attempt (default: 3, like arcade)
    pub lives_per_attempt: u8,

    /// Time limit per scenario (fixed, no scaling)
    pub time_per_scenario: Duration,

    /// Seed for deterministic scenario selection
    pub seed: u64,

    /// The date this challenge is for (UTC)
    pub date: chrono::NaiveDate,
}

impl ChallengeConfig {
    /// Create a challenge config for a specific date
    pub fn for_date(date: chrono::NaiveDate) -> Self {
        // Seed = days since Unix epoch, ensuring same seed for same date
        #[expect(
            clippy::cast_sign_loss,
            reason = "pre-1970 dates wrap to a different but still deterministic seed"
        )]
        let seed = date.and_time(chrono::NaiveTime::MIN).and_utc().timestamp() as u64;

        Self {
            scenario_count: CHALLENGE_SCENARIO_COUNT,
            max_attempts_per_day: CHALLENGE_MAX_ATTEMPTS,
            lives_per_attempt: CHALLENGE_LIVES_PER_ATTEMPT,
            time_per_scenario: CHALLENGE_TIME_PER_SCENARIO,
            seed,
            date,
        }
    }

    /// Check if this challenge is for the given date
    pub fn is_today(&self, today: chrono::NaiveDate) -> bool {
        self.date == today
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(n: u8) -> DifficultyLevel {
        DifficultyLevel::new(n).unwrap()
    }

    #[test]
    fn test_arcade_mode_defaults() {
        let mode = MiniGameMode::default();
        assert!(mode.is_arcade());
        assert_eq!(mode.starting_lives(), 3);
        assert!(mode.has_session_timer());
        assert_eq!(mode.session_duration(), Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_survival_mode_config() {
        let mode = MiniGameMode::Survival(SurvivalConfig::default());
        assert!(mode.is_survival());
        assert_eq!(mode.starting_lives(), 1);
        assert!(!mode.has_session_timer());
        assert_eq!(mode.session_duration(), None);
    }

    #[test]
    fn test_challenge_mode_config() {
        let mode =
            MiniGameMode::Challenge(ChallengeConfig::for_date(chrono::Utc::now().date_naive()));
        assert!(mode.is_challenge());
        assert_eq!(mode.starting_lives(), 3);
        assert!(!mode.has_session_timer());
        assert_eq!(mode.session_duration(), None);
    }

    #[test]
    fn test_survival_time_decrease_linear() {
        let config = SurvivalConfig::default();

        // Level 1 should have base time
        let time = config.time_limit_for_level(level(1));
        assert_eq!(time, SURVIVAL_BASE_TIME);

        // Level 2 should be base - 500ms
        let time = config.time_limit_for_level(level(2));
        assert_eq!(
            time,
            SURVIVAL_BASE_TIME.saturating_sub(Duration::from_millis(500))
        );

        // Level 3 should be base - 1000ms
        let time = config.time_limit_for_level(level(3));
        assert_eq!(
            time,
            SURVIVAL_BASE_TIME.saturating_sub(Duration::from_millis(1000))
        );
    }

    #[test]
    fn test_survival_time_decrease_exponential() {
        let config = SurvivalConfig {
            decrease_mode: TimeDecreaseMode::Exponential,
            ..SurvivalConfig::default()
        };

        // Level 1 should have base time
        let time = config.time_limit_for_level(level(1));
        assert_eq!(time, config.base_time_per_scenario);

        // Level 2 should be 95% of base
        let time = config.time_limit_for_level(level(2));
        let expected_secs = config.base_time_per_scenario.as_secs_f64() * 0.95;
        assert!((time.as_secs_f64() - expected_secs).abs() < 0.01);

        // Higher levels should continue decreasing
        let time_l5 = config.time_limit_for_level(level(5));
        let time_l10 = config.time_limit_for_level(level(10));
        assert!(time_l5 < time);
        assert!(time_l10 < time_l5);
    }

    #[test]
    fn test_survival_min_time_clamping() {
        let config = SurvivalConfig {
            min_time_per_scenario: SURVIVAL_BASE_TIME,
            ..SurvivalConfig::default()
        };
        assert_eq!(
            config.time_limit_for_level(DifficultyLevel::MAX),
            config.min_time_per_scenario
        );

        let exp_config = SurvivalConfig {
            decrease_mode: TimeDecreaseMode::Exponential,
            ..config
        };
        assert_eq!(
            exp_config.time_limit_for_level(DifficultyLevel::MAX),
            exp_config.min_time_per_scenario
        );
    }

    #[test]
    fn test_challenge_seed_consistency() {
        let date = chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let config1 = ChallengeConfig::for_date(date);
        let config2 = ChallengeConfig::for_date(date);

        assert_eq!(config1.seed, config2.seed);
        assert_eq!(config1.date, config2.date);
    }

    #[test]
    fn test_challenge_different_days_different_seeds() {
        let date1 = chrono::NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let date2 = chrono::NaiveDate::from_ymd_opt(2025, 1, 16).unwrap();

        let config1 = ChallengeConfig::for_date(date1);
        let config2 = ChallengeConfig::for_date(date2);

        assert_ne!(config1.seed, config2.seed);
    }

    #[test]
    fn test_arcade_config_defaults() {
        let config = ArcadeConfig::default();
        assert_eq!(config.session_duration, Duration::from_secs(60));
        assert_eq!(config.starting_lives, 3);
        assert_eq!(config.max_lives, 5);
        assert_eq!(config.extra_life_milestone, EXTRA_LIFE_SCORE_MILESTONE);
    }

    #[test]
    fn test_challenge_is_today() {
        let today = chrono::Utc::now().date_naive();
        let config = ChallengeConfig::for_date(today);
        assert!(config.is_today(today));

        let past = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let past_config = ChallengeConfig::for_date(past);
        assert!(!past_config.is_today(today));
    }

    #[test]
    fn test_survival_time_at_max_level_never_below_minimum() {
        for decrease_mode in [TimeDecreaseMode::Linear, TimeDecreaseMode::Exponential] {
            let config = SurvivalConfig {
                decrease_mode,
                ..SurvivalConfig::default()
            };
            let time = config.time_limit_for_level(DifficultyLevel::MAX);
            assert!(time >= config.min_time_per_scenario);
            assert!(time < config.time_limit_for_level(DifficultyLevel::MIN));
        }
    }
}
