//! User profile and XP calculation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{AchievementId, Quest};
use crate::config::Difficulty;
use crate::config::keymap::fingerprint::StoredKeymapFingerprint;
use crate::constants::{
    MAX_PLAYER_LEVEL, MINIGAME_LEVEL_BONUS_XP, MINIGAME_SCENARIO_BASE_XP,
    MINIGAME_STREAK_BONUS_DIVISOR, MINIGAME_STREAK_BONUS_MAX_XP, MINIGAME_STREAK_XP_MULTIPLIER,
    MINIGAME_XP_PER_100_POINTS, REVIEW_SUCCESS_BONUS_MAX_XP, REVIEW_XP_PER_COMMAND,
    SCENARIO_BASE_XP_PER_100_POINTS, SCENARIO_FIRST_TODAY_BONUS_XP, SCENARIO_PERFECT_BONUS_DIVISOR,
    XP_LEVEL_FORMULA_BASE, XP_LEVEL_FORMULA_EXPONENT,
};
use crate::ids::QuestId;
use crate::learning::PerformanceData;
use crate::learning::ScenarioHistory;
use crate::numeric::{self, Rounding};
use crate::sound::SoundConfig;

/// An achievement id from disk that may be unknown to this build.
#[derive(Deserialize)]
#[serde(untagged)]
enum MaybeKnownAchievement {
    Known(AchievementId),
    Unknown(serde::de::IgnoredAny),
}

fn deserialize_known_achievements<'de, D>(
    deserializer: D,
) -> Result<HashSet<AchievementId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Vec::<MaybeKnownAchievement>::deserialize(deserializer)?;
    let total = raw.len();
    let known: HashSet<AchievementId> = raw
        .into_iter()
        .filter_map(|entry| match entry {
            MaybeKnownAchievement::Known(id) => Some(id),
            MaybeKnownAchievement::Unknown(_) => None,
        })
        .collect();
    if known.len() < total {
        tracing::warn!(
            "Ignored {} unknown achievement id(s) in profile",
            total.saturating_sub(known.len())
        );
    }
    Ok(known)
}

/// User profile with progression, streaks, and achievements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Current level
    pub level: u32,

    /// Total XP accumulated
    pub total_xp: u64,

    /// Current streak (consecutive days)
    pub current_streak: u32,

    /// Longest streak ever achieved
    pub longest_streak: u32,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Unlocked achievements
    ///
    /// Ids this build does not know (e.g. written by a newer version) are
    /// dropped on load instead of failing the whole profile.
    #[serde(deserialize_with = "deserialize_known_achievements")]
    pub achievements_unlocked: HashSet<AchievementId>,

    /// Active daily quests
    pub daily_quests: Vec<Quest>,

    /// Last time quests were refreshed
    pub last_quest_refresh: DateTime<Utc>,

    /// Completed quest IDs today
    pub completed_quests_today: HashSet<QuestId>,

    /// Streak freeze available (protects from one missed day)
    pub streak_freeze_available: bool,

    /// Total scenarios completed
    pub scenarios_completed: u32,

    /// Perfect scenario count
    pub perfect_scenarios: u32,

    /// Total commands executed
    pub commands_executed: u32,

    /// Scenario completion history and mastery tracking
    #[serde(default)]
    pub scenario_history: ScenarioHistory,

    /// Best score achieved in mini-game mode
    #[serde(default)]
    pub minigame_high_score: u64,

    /// Best streak achieved in mini-game mode
    #[serde(default)]
    pub minigame_best_streak: u32,

    /// Total number of mini-game sessions played
    #[serde(default)]
    pub minigame_games_played: u32,

    /// Sound configuration for audio feedback
    #[serde(default)]
    pub sound_config: SoundConfig,

    /// Challenge mode progress (daily puzzle tracking)
    #[serde(default)]
    pub challenge_progress: crate::gamification::ChallengeProgress,

    /// Survival mode best level reached
    #[serde(default)]
    pub survival_best_level: u32,

    /// Survival mode best scenarios completed in single run
    #[serde(default)]
    pub survival_best_scenarios: u32,

    /// Persisted FSRS/PerformanceTracker data
    #[serde(default)]
    pub performance_data: PerformanceData,

    /// Count of scenario completions finished in under
    /// `SPEED_DEMON_TIME_RATIO` of a
    /// scenario's time budget, in either Training or arcade mode. Powers
    /// `SpeedDemon`/`Speedrunner`.
    #[serde(default)]
    pub speed_run_count: u32,

    /// Count of scenario completions finished in under
    /// `FLASH_TIME_RATIO` of a scenario's
    /// time budget, in either Training or arcade mode. Powers `Flash`.
    #[serde(default)]
    pub flash_run_count: u32,

    /// Difficulty levels completed at least once, across any mode. Powers `Polyglot`.
    #[serde(default)]
    pub difficulties_completed: HashSet<Difficulty>,

    /// Fingerprint of the resolved keymap bindings active when review
    /// history was last recorded, or `None` if no custom keymap has ever
    /// been active for this profile. A pre-tag bare number loads as
    /// [`StoredKeymapFingerprint::Legacy`].
    ///
    /// Compared at startup against the currently resolved keymap: a
    /// mismatch means FSRS review history was built under a different key
    /// mapping than the one now active, which callers surface as a
    /// notification rather than silently trusting stale scheduling data.
    #[serde(
        default,
        deserialize_with = "crate::config::keymap::fingerprint::lenient_stored_fingerprint"
    )]
    pub keymap_fingerprint: Option<StoredKeymapFingerprint>,
}

impl UserProfile {
    /// Create a new user profile
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let profile = UserProfile::new();
    /// assert_eq!(profile.level, 1);
    /// assert_eq!(profile.total_xp, 0);
    /// assert_eq!(profile.current_streak, 0);
    /// ```
    pub fn new() -> Self {
        Self::new_at(Utc::now())
    }

    /// Create a new user profile with an explicit creation timestamp
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let profile = UserProfile::new_at(Utc::now());
    /// assert_eq!(profile.level, 1);
    /// ```
    pub fn new_at(now: DateTime<Utc>) -> Self {
        Self {
            level: 1,
            total_xp: 0,
            current_streak: 0,
            longest_streak: 0,
            last_activity: now,
            achievements_unlocked: HashSet::new(),
            daily_quests: Vec::new(),
            last_quest_refresh: now,
            completed_quests_today: HashSet::new(),
            streak_freeze_available: false,
            scenarios_completed: 0,
            perfect_scenarios: 0,
            commands_executed: 0,
            scenario_history: ScenarioHistory::new(),
            minigame_high_score: 0,
            minigame_best_streak: 0,
            minigame_games_played: 0,
            sound_config: SoundConfig::default(),
            challenge_progress: crate::gamification::ChallengeProgress::default(),
            survival_best_level: 0,
            survival_best_scenarios: 0,
            performance_data: PerformanceData::default(),
            speed_run_count: 0,
            flash_run_count: 0,
            difficulties_completed: HashSet::new(),
            keymap_fingerprint: None,
        }
    }

    /// Add XP and update level if needed
    ///
    /// Returns true if leveled up
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let mut profile = UserProfile::new();
    /// let leveled_up = profile.add_xp(100);
    /// assert!(leveled_up); // Leveled up from 1 to 2
    /// assert_eq!(profile.level, 2);
    /// ```
    pub fn add_xp(&mut self, xp: u64) -> bool {
        let old_level = self.level;
        self.total_xp = self.total_xp.saturating_add(xp);
        self.level = XPCalculator::level_from_xp(self.total_xp);
        self.level > old_level
    }

    /// Replace `level` with the level implied by `total_xp`.
    ///
    /// `total_xp` is the single source of truth; a stored `level` that
    /// disagrees with it (hand edit, downgrade) is silently corrected.
    pub fn recompute_level(&mut self) {
        self.level = XPCalculator::level_from_xp(self.total_xp);
    }

    /// Position of this profile inside its current level.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let mut profile = UserProfile::new();
    /// profile.add_xp(150);
    /// let progress = profile.level_progress();
    /// assert_eq!((progress.earned(), progress.span()), (50, 183));
    /// ```
    #[must_use]
    pub fn level_progress(&self) -> LevelProgress {
        LevelProgress::at(self.level, self.total_xp)
    }

    /// XP earned inside the current level, clamped to `[0, level span]`.
    ///
    /// Safe for any `(level, total_xp)` pair, including ones that are
    /// inconsistent with each other.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let mut profile = UserProfile::new();
    /// profile.add_xp(150);
    /// assert_eq!(profile.xp_in_level(), 50);
    /// ```
    pub fn xp_in_level(&self) -> u64 {
        self.level_progress().earned()
    }

    /// Get XP progress to next level (0.0 - 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let mut profile = UserProfile::new();
    /// profile.add_xp(50); // 50/100 XP to level 2
    /// assert_eq!(profile.xp_progress(), 0.5);
    /// ```
    pub fn xp_progress(&self) -> f64 {
        self.level_progress().ratio()
    }

    /// Check if quest is completed today
    pub fn is_quest_completed(&self, quest_id: &QuestId) -> bool {
        self.completed_quests_today.contains(quest_id)
    }

    /// Mark quest as completed
    pub fn complete_quest(&mut self, quest_id: QuestId) {
        self.completed_quests_today.insert(quest_id);
    }

    /// Check if achievement is unlocked
    pub fn has_achievement(&self, achievement: &AchievementId) -> bool {
        self.achievements_unlocked.contains(achievement)
    }

    /// Unlock achievement
    ///
    /// Returns true if newly unlocked (was not already unlocked)
    pub fn unlock_achievement(&mut self, achievement: AchievementId) -> bool {
        self.achievements_unlocked.insert(achievement)
    }

    /// Reset daily quest state (called at midnight)
    pub fn reset_daily_quests(&mut self, now: DateTime<Utc>) {
        self.completed_quests_today.clear();
        self.daily_quests.clear();
        self.last_quest_refresh = now;
    }
}

impl Default for UserProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// XP earned inside a level together with the XP the level spans.
///
/// Every screen that shows level progress (label, percentage, gauge) derives it from this one
/// value so they cannot disagree. `earned` never exceeds `span`.
///
/// # Examples
///
/// ```
/// use helix_trainer::gamification::LevelProgress;
///
/// let progress = LevelProgress::at(22, 9822);
/// assert_eq!((progress.earned(), progress.span()), (199, 696));
/// assert_eq!(progress.percent(), 28);
///
/// let empty = LevelProgress::new(5, 0);
/// assert_eq!((empty.earned(), empty.span(), empty.ratio()), (0, 0, 1.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelProgress {
    earned: u64,
    span: u64,
}

impl LevelProgress {
    /// Builds a progress value, clamping `earned` to `span`.
    #[must_use]
    pub fn new(earned: u64, span: u64) -> Self {
        Self {
            earned: earned.min(span),
            span,
        }
    }

    /// Progress of `total_xp` inside `level`.
    ///
    /// The next level is `level + 1`, saturating so a profile at the level cap does not
    /// overflow (#523); a `total_xp` outside the level's range is clamped to `0` or the span.
    #[must_use]
    pub fn at(level: u32, total_xp: u64) -> Self {
        let start = XPCalculator::xp_for_level(level);
        let end = XPCalculator::xp_for_level(level.saturating_add(1));
        Self::new(total_xp.saturating_sub(start), end.saturating_sub(start))
    }

    /// XP earned inside the level.
    #[must_use]
    pub fn earned(self) -> u64 {
        self.earned
    }

    /// XP the level spans.
    #[must_use]
    pub fn span(self) -> u64 {
        self.span
    }

    /// Fraction of the level completed in `0.0..=1.0`; a zero-XP span counts as complete.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    pub fn ratio(self) -> f64 {
        if self.span == 0 {
            1.0
        } else {
            self.earned as f64 / self.span as f64
        }
    }

    /// Whole percentage of the level completed, rounded down, in `0..=100`.
    #[must_use]
    pub fn percent(self) -> u32 {
        crate::numeric::floor_percent(self.ratio())
    }
}

/// Components of a training-scenario XP award before mastery scaling
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XPComponents {
    /// XP derived from the scenario score
    pub base_xp: u64,
    /// Bonus for a perfect score
    pub perfect_bonus: u64,
    /// Bonus for the first completion of the day
    pub first_today_bonus: u64,
    /// Sum of the three components
    pub total_base_xp: u64,
}

/// XP calculation and leveling formulas
pub struct XPCalculator;

impl XPCalculator {
    /// Total XP required to reach a given level (minimum XP threshold)
    ///
    /// Formula: `100 * (level-1)^1.5` for level >= 2, 0 for level 1
    ///
    /// This gives XP thresholds:
    /// - Level 1: 0 XP
    /// - Level 2: 100 XP
    /// - Level 3: 283 XP
    /// - Level 5: 900 XP
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// assert_eq!(XPCalculator::xp_for_level(1), 0);      // Level 1 starts at 0
    /// assert_eq!(XPCalculator::xp_for_level(2), 100);    // Level 2 starts at 100
    /// assert_eq!(XPCalculator::xp_for_level(3), 283);    // Level 3 starts at 283 (rounded)
    /// ```
    pub fn xp_for_level(level: u32) -> u64 {
        if level <= 1 {
            0
        } else {
            numeric::to_u64(
                XP_LEVEL_FORMULA_BASE
                    * (f64::from(level.saturating_sub(1))).powf(XP_LEVEL_FORMULA_EXPONENT),
                Rounding::Nearest,
            )
        }
    }

    /// Calculate level from total XP
    ///
    /// Uses binary search to find the highest level where threshold <= `total_xp`
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// assert_eq!(XPCalculator::level_from_xp(0), 1);
    /// assert_eq!(XPCalculator::level_from_xp(99), 1);
    /// assert_eq!(XPCalculator::level_from_xp(100), 2);
    /// assert_eq!(XPCalculator::level_from_xp(283), 3);
    /// ```
    pub fn level_from_xp(total_xp: u64) -> u32 {
        // Binary search for level
        let mut low = 1u32;
        let mut high = MAX_PLAYER_LEVEL;

        while low < high {
            let mid = low.saturating_add(high).div_ceil(2);
            if Self::xp_for_level(mid) <= total_xp {
                low = mid;
            } else {
                high = mid.saturating_sub(1);
            }
        }

        low.max(1)
    }

    /// Calculate XP reward for quest completion
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{QuestDifficulty, QuestType, XPCalculator};
    /// use helix_trainer::ids::NormalizedCommand;
    ///
    /// let easy_practice = QuestType::CommandPractice {
    ///     command: NormalizedCommand::from_raw("w").unwrap(),
    ///     target: 5,
    ///     current: 0,
    /// };
    /// assert_eq!(XPCalculator::quest_xp_reward(&easy_practice, QuestDifficulty::Easy), 25);
    ///
    /// let hard_scenario = QuestType::ScenarioCompletion {
    ///     target: 3,
    ///     current: 0,
    /// };
    /// assert_eq!(XPCalculator::quest_xp_reward(&hard_scenario, QuestDifficulty::Hard), 150);
    /// ```
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    pub fn quest_xp_reward(
        quest_type: &super::QuestType,
        difficulty: super::QuestDifficulty,
    ) -> u32 {
        use super::{QuestDifficulty, QuestType};

        match quest_type {
            QuestType::CommandPractice { .. } => match difficulty {
                QuestDifficulty::Easy => 25,
                QuestDifficulty::Medium => 50,
                QuestDifficulty::Hard => 100,
            },
            QuestType::ScenarioCompletion { .. } => match difficulty {
                QuestDifficulty::Easy => 30,
                QuestDifficulty::Medium => 75,
                QuestDifficulty::Hard => 150,
            },
            QuestType::SpeedRun { .. } => match difficulty {
                QuestDifficulty::Easy => 50,
                QuestDifficulty::Medium => 100,
                QuestDifficulty::Hard => 200,
            },
            QuestType::TimeInvested { .. } => match difficulty {
                QuestDifficulty::Easy => 25,
                QuestDifficulty::Medium => 50,
                QuestDifficulty::Hard => 25, // No hard variant
            },
            QuestType::Exploration { .. } => match difficulty {
                QuestDifficulty::Easy => 40,
                QuestDifficulty::Medium => 80,
                QuestDifficulty::Hard => 160,
            },
        }
    }

    /// XP components for a completed training scenario, before mastery scaling
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// let xp = XPCalculator::training_xp_components(100, true, true);
    /// assert_eq!(xp.total_base_xp, 50 + 10 + 10);
    /// ```
    #[must_use]
    pub fn training_xp_components(
        score: u32,
        is_perfect: bool,
        is_first_today: bool,
    ) -> XPComponents {
        let base_xp = (u64::from(score) * SCENARIO_BASE_XP_PER_100_POINTS) / 100;
        let perfect_bonus = if is_perfect {
            base_xp / SCENARIO_PERFECT_BONUS_DIVISOR
        } else {
            0
        };
        let first_today_bonus = if is_first_today {
            SCENARIO_FIRST_TODAY_BONUS_XP
        } else {
            0
        };
        let total_base_xp = base_xp
            .saturating_add(perfect_bonus)
            .saturating_add(first_today_bonus);

        XPComponents {
            base_xp,
            perfect_bonus,
            first_today_bonus,
            total_base_xp,
        }
    }

    /// XP for one arcade scenario completion at the given streak
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// assert_eq!(XPCalculator::arcade_scenario_xp(1), 15);
    /// assert_eq!(XPCalculator::arcade_scenario_xp(5), 23);
    /// ```
    #[must_use]
    pub fn arcade_scenario_xp(streak: u32) -> u64 {
        MINIGAME_SCENARIO_BASE_XP.saturating_add(
            u64::from(streak.saturating_sub(1)).saturating_mul(MINIGAME_STREAK_XP_MULTIPLIER),
        )
    }

    /// XP for a finished review session: a flat amount per completed command plus a
    /// bonus proportional to the share of commands recalled successfully
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// assert_eq!(XPCalculator::review_session_xp(5, 3), 62);
    /// assert_eq!(XPCalculator::review_session_xp(0, 0), 0);
    /// ```
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    pub fn review_session_xp(completed: usize, recalled: usize) -> u64 {
        let success_rate = if completed > 0 {
            recalled as f64 / completed as f64
        } else {
            0.0
        };
        (completed as u64)
            .saturating_mul(REVIEW_XP_PER_COMMAND)
            .saturating_add(numeric::to_u64(
                success_rate * REVIEW_SUCCESS_BONUS_MAX_XP,
                Rounding::Floor,
            ))
    }

    /// Calculate XP earned from mini-game session
    ///
    /// # Arguments
    ///
    /// * `score` - Total score achieved
    /// * `level` - Final difficulty level reached
    /// * `best_streak` - Best streak achieved
    ///
    /// # Formula
    ///
    /// - Base XP: 1 XP per 100 points scored
    /// - Level bonus: 10 XP per difficulty level reached
    /// - Streak bonus: 15 XP per 5 streak achieved
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// // Score 5000, level 3, streak 10
    /// let xp = XPCalculator::minigame_xp(5000, 3, 10);
    /// assert_eq!(xp, 50 + 30 + 30); // 50 (score) + 30 (level) + 30 (streak) = 110
    ///
    /// // Score 10000, level 5, streak 25
    /// let xp = XPCalculator::minigame_xp(10000, 5, 25);
    /// assert_eq!(xp, 100 + 50 + 75); // 100 + 50 + 75 = 225
    /// ```
    pub fn minigame_xp(score: u64, level: u32, best_streak: u32) -> u64 {
        // Base XP per 100 points scored
        let base_xp = score / MINIGAME_XP_PER_100_POINTS;

        // Level bonus per level reached
        let level_bonus = u64::from(level).saturating_mul(MINIGAME_LEVEL_BONUS_XP);

        // Streak bonus per streak milestone
        let streak_bonus = (u64::from(best_streak) / MINIGAME_STREAK_BONUS_DIVISOR)
            .saturating_mul(MINIGAME_STREAK_BONUS_MAX_XP);

        base_xp
            .saturating_add(level_bonus)
            .saturating_add(streak_bonus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::qid;

    #[test]
    fn profile_with_unknown_keymap_fingerprint_tag_loads_without_fingerprint() {
        let mut json = serde_json::to_value(UserProfile::new()).unwrap();
        json["keymap_fingerprint"] = serde_json::json!({"Sha256V2": "abc"});
        let profile: UserProfile = serde_json::from_str(&json.to_string()).unwrap();
        assert_eq!(profile.keymap_fingerprint, None);

        json["keymap_fingerprint"] = serde_json::json!("garbage");
        assert!(serde_json::from_str::<UserProfile>(&json.to_string()).is_err());
    }

    /// Golden values derived from the pre-#485 inline formulas; they pin current
    /// behavior so the move into `XPCalculator` cannot change any award.
    #[test]
    fn golden_training_xp_components() {
        let cases: [(u32, bool, bool, [u64; 4]); 6] = [
            (100, true, false, [50, 10, 0, 60]),
            (100, true, true, [50, 10, 10, 70]),
            (85, false, false, [42, 0, 0, 42]),
            (85, false, true, [42, 0, 10, 52]),
            (0, false, false, [0, 0, 0, 0]),
            (30, true, false, [15, 3, 0, 18]),
        ];
        for (score, perfect, first, [base, bonus, today, total]) in cases {
            let xp = XPCalculator::training_xp_components(score, perfect, first);
            assert_eq!(
                (
                    xp.base_xp,
                    xp.perfect_bonus,
                    xp.first_today_bonus,
                    xp.total_base_xp
                ),
                (base, bonus, today, total),
                "score={score} perfect={perfect} first={first}"
            );
        }
    }

    #[test]
    fn golden_arcade_scenario_xp() {
        for (streak, expected) in [(0, 15), (1, 15), (2, 17), (5, 23), (10, 33)] {
            assert_eq!(
                XPCalculator::arcade_scenario_xp(streak),
                expected,
                "streak={streak}"
            );
        }
    }

    #[test]
    fn golden_review_session_xp() {
        let cases = [
            (0, 0, 0),
            (1, 1, 30),
            (1, 0, 10),
            (5, 3, 62),
            (7, 4, 81),
            (10, 10, 120),
            (3, 1, 36),
            (6, 5, 76),
        ];
        for (completed, recalled, expected) in cases {
            assert_eq!(
                XPCalculator::review_session_xp(completed, recalled),
                expected,
                "completed={completed} recalled={recalled}"
            );
        }
    }

    #[test]
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn test_new_profile_defaults() {
        let profile = UserProfile::new();
        assert_eq!(profile.level, 1);
        assert_eq!(profile.total_xp, 0);
        assert_eq!(profile.current_streak, 0);
        assert_eq!(profile.longest_streak, 0);
        assert!(profile.achievements_unlocked.is_empty());
        assert!(profile.daily_quests.is_empty());
        assert!(!profile.streak_freeze_available);
    }

    #[test]
    fn test_add_xp_levels_up() {
        let mut profile = UserProfile::new();

        // Add 100 XP - should level up from 1 to 2
        let leveled_up = profile.add_xp(100);
        assert!(leveled_up);
        assert_eq!(profile.level, 2);
        assert_eq!(profile.total_xp, 100);

        // Add more XP without leveling
        let leveled_up = profile.add_xp(50);
        assert!(!leveled_up);
        assert_eq!(profile.level, 2);
        assert_eq!(profile.total_xp, 150);
    }

    #[test]
    fn test_xp_progress() {
        let mut profile = UserProfile::new();

        // 50 out of 100 XP to level 2
        profile.add_xp(50);
        assert_eq!(profile.xp_progress(), 0.5);

        // Complete level 2
        profile.add_xp(50);
        assert_eq!(profile.xp_progress(), 0.0); // Reset at new level
    }

    #[test]
    fn test_xp_calculator_level_formula() {
        assert_eq!(XPCalculator::xp_for_level(1), 0);
        assert_eq!(XPCalculator::xp_for_level(2), 100);
        assert_eq!(XPCalculator::xp_for_level(3), 283); // 100 * 2^1.5 = 282.8 → 283 (rounded)

        // Level 5 should be 100 * 4^1.5 = 100 * 8 = 800
        let level5_xp = XPCalculator::xp_for_level(5);
        assert_eq!(level5_xp, 800);
    }

    #[test]
    fn test_xp_calculator_level_from_xp() {
        assert_eq!(XPCalculator::level_from_xp(0), 1);
        assert_eq!(XPCalculator::level_from_xp(99), 1); // Just below threshold
        assert_eq!(XPCalculator::level_from_xp(100), 2);
        assert_eq!(XPCalculator::level_from_xp(101), 2); // Just above threshold
        assert_eq!(XPCalculator::level_from_xp(283), 3); // 283 with rounding
        assert_eq!(XPCalculator::level_from_xp(800), 5);
    }

    #[test]
    fn test_xp_calculator_roundtrip() {
        for level in 1..20 {
            let xp = XPCalculator::xp_for_level(level);
            let calculated_level = XPCalculator::level_from_xp(xp);
            assert_eq!(calculated_level, level);
        }
    }

    #[test]
    fn test_achievement_tracking() {
        let mut profile = UserProfile::new();

        assert!(!profile.has_achievement(&AchievementId::FirstPerfect));

        let newly_unlocked = profile.unlock_achievement(AchievementId::FirstPerfect);
        assert!(newly_unlocked);
        assert!(profile.has_achievement(&AchievementId::FirstPerfect));

        // Try unlocking again
        let newly_unlocked = profile.unlock_achievement(AchievementId::FirstPerfect);
        assert!(!newly_unlocked); // Already had it
    }

    #[test]
    fn test_quest_completion_tracking() {
        let mut profile = UserProfile::new();

        assert!(!profile.is_quest_completed(&qid("quest_1")));

        profile.complete_quest(qid("quest_1"));
        assert!(profile.is_quest_completed(&qid("quest_1")));
    }

    #[test]
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn test_reset_daily_quests() {
        let mut profile = UserProfile::new();

        profile.complete_quest(qid("quest_1"));
        assert!(profile.is_quest_completed(&qid("quest_1")));

        profile.reset_daily_quests(Utc::now());
        assert!(!profile.is_quest_completed(&qid("quest_1")));
        assert!(profile.daily_quests.is_empty());
    }

    #[test]
    fn test_minigame_xp_calculation() {
        // Low score, low level, low streak
        let xp = XPCalculator::minigame_xp(1000, 1, 2);
        assert_eq!(xp, 20); // 10 (score) + 10 (level) + 0 (streak < 5) = 20

        // Medium score, medium level, medium streak
        let xp = XPCalculator::minigame_xp(5000, 3, 10);
        assert_eq!(xp, 110); // 50 + 30 + 30 = 110

        // High score, high level, high streak
        let xp = XPCalculator::minigame_xp(10000, 5, 25);
        assert_eq!(xp, 225); // 100 + 50 + 75 = 225

        // Perfect game example
        let xp = XPCalculator::minigame_xp(50000, 10, 100);
        assert_eq!(xp, 900); // 500 + 100 + 300 = 900

        // Edge case: zero score
        let xp = XPCalculator::minigame_xp(0, 1, 0);
        assert_eq!(xp, 10); // 10 (level only)
    }

    #[test]
    fn test_minigame_high_score_tracking() {
        let mut profile = UserProfile::new();
        assert_eq!(profile.minigame_high_score, 0);
        assert_eq!(profile.minigame_best_streak, 0);

        profile.minigame_high_score = 5000;
        profile.minigame_best_streak = 15;

        assert_eq!(profile.minigame_high_score, 5000);
        assert_eq!(profile.minigame_best_streak, 15);
    }

    #[test]
    fn test_minigame_games_played_default() {
        let profile = UserProfile::new();
        assert_eq!(profile.minigame_games_played, 0);
    }

    #[test]
    fn test_minigame_games_played_saturates_at_max() {
        let mut profile = UserProfile::new();
        profile.minigame_games_played = u32::MAX;
        profile.minigame_games_played = profile.minigame_games_played.saturating_add(1);
        assert_eq!(profile.minigame_games_played, u32::MAX);
    }

    #[test]
    fn test_sound_config_persisted_in_profile() {
        use crate::sound::{SoundConfig, Volume};

        let mut profile = UserProfile::new();
        profile.sound_config = SoundConfig::new(Volume::new(0.3), false);

        // Serialize and deserialize (TOML is the actual format)
        let toml = toml::to_string(&profile).unwrap();
        let restored: UserProfile = toml::from_str(&toml).unwrap();

        assert!((restored.sound_config.volume().get() - 0.3).abs() < f32::EPSILON);
        assert!(!restored.sound_config.enabled());
    }

    // CR-016: Test backward compatibility with old profile format (missing game mode fields)
    #[test]
    fn test_profile_game_mode_fields_backward_compat() {
        // Create a full profile, serialize it, then verify game mode fields can be absent
        let mut profile = UserProfile::new();
        profile.level = 5;
        profile.total_xp = 1000;
        profile.current_streak = 3;
        profile.survival_best_level = 10;
        profile.survival_best_scenarios = 25;

        // Serialize
        let toml_str = toml::to_string(&profile).unwrap();
        assert!(toml_str.contains("survival_best_level"));

        // Now deserialize back
        let restored: UserProfile = toml::from_str(&toml_str).unwrap();

        // Verify original fields are loaded
        assert_eq!(restored.level, 5);
        assert_eq!(restored.total_xp, 1000);
        assert_eq!(restored.current_streak, 3);

        // Verify new game mode fields are loaded correctly
        assert_eq!(restored.survival_best_level, 10);
        assert_eq!(restored.survival_best_scenarios, 25);

        // Verify defaults work for challenge_progress (via #[serde(default)])
        assert_eq!(restored.challenge_progress.attempts_used_today, 0);
    }

    /// Regression test for #376: a non-default `challenge_progress` must survive
    /// a real `ProfileStorage::save`/`load` round-trip (the actual persisted
    /// format, JSON via `profile.json`), not just a bare TOML round-trip of
    /// `ChallengeProgress` in isolation.
    #[test]
    fn test_challenge_progress_persists_through_profile_storage_round_trip() {
        use crate::gamification::ProfileStorage;

        let mut profile = UserProfile::new();
        let today = chrono::Utc::now().date_naive();
        profile.challenge_progress.start_attempt(today);
        profile.challenge_progress.start_attempt(today);
        profile.challenge_progress.record_result(4200, 7);

        let storage = ProfileStorage::for_test();
        storage.save(&profile).unwrap();
        let restored = storage.load().unwrap();

        assert_eq!(restored.challenge_progress.attempts_used_today, 2);
        assert_eq!(restored.challenge_progress.total_challenges_attempted, 2);
        assert_eq!(restored.challenge_progress.best_score_today, 4200);
        assert_eq!(restored.challenge_progress.best_scenarios_today, 7);
        assert_eq!(restored.challenge_progress.all_time_best_score, 4200);
        assert_eq!(restored.challenge_progress.all_time_best_scenarios, 7);
    }

    /// Regression test for #450: `level` far above what `total_xp` supports
    /// must not panic `xp_progress`/`xp_in_level` (debug overflow) or leave
    /// the bar out of range.
    #[test]
    fn test_xp_progress_inconsistent_level_never_panics_or_leaves_range() {
        for (level, total_xp) in [
            (50, 0),
            (u32::MAX, 0),
            (u32::MAX, u64::MAX),
            (1, 1_000_000),
            (0, 0),
            (1, 0),
        ] {
            let mut profile = UserProfile::new();
            profile.level = level;
            profile.total_xp = total_xp;

            let progress = profile.xp_progress();
            assert!(
                (0.0..=1.0).contains(&progress),
                "({level}, {total_xp}) -> {progress}"
            );
            let span = XPCalculator::xp_for_level(level.saturating_add(1))
                .saturating_sub(XPCalculator::xp_for_level(level));
            assert!(profile.xp_in_level() <= span);
        }
    }

    #[test]
    fn test_xp_progress_unchanged_for_consistent_profile() {
        let mut profile = UserProfile::new();
        profile.add_xp(150);
        assert_eq!(profile.level, 2);
        assert_eq!(profile.xp_in_level(), 50);
        assert!((profile.xp_progress() - 50.0 / 183.0).abs() < 1e-9);
    }

    #[test]
    fn test_recompute_level_follows_total_xp() {
        let mut profile = UserProfile::new();
        profile.level = 50;
        profile.recompute_level();
        assert_eq!(profile.level, 1);

        profile.total_xp = 1_000_000;
        profile.recompute_level();
        assert_eq!(profile.level, XPCalculator::level_from_xp(1_000_000));
    }

    #[test]
    fn level_progress_matches_every_screen_example() {
        let at = |level, total_xp| {
            let progress = LevelProgress::at(level, total_xp);
            (progress.earned(), progress.span(), progress.percent())
        };
        assert_eq!(at(1, 50), (50, 100, 50));
        assert_eq!(at(22, 9822), (199, 696, 28));
        assert_eq!(at(2, 100), (0, 183, 0));
        assert_eq!(at(100, 1_000_000), (1496, 1496, 100));
    }

    #[test]
    fn level_progress_span_zero_is_full() {
        let progress = LevelProgress::new(7, 0);
        assert_eq!((progress.earned(), progress.span()), (0, 0));
        assert_eq!(progress.ratio(), 1.0);
        assert_eq!(progress.percent(), 100);
    }

    proptest::proptest! {
        #[test]
        fn level_progress_stays_within_span(level in proptest::num::u32::ANY, total_xp in proptest::num::u64::ANY) {
            let progress = LevelProgress::at(level, total_xp);
            proptest::prop_assert!(progress.earned() <= progress.span());
            proptest::prop_assert!((0.0..=1.0).contains(&progress.ratio()));
            proptest::prop_assert!(progress.percent() <= 100);
        }
    }
}
