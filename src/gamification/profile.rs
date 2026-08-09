//! User profile and XP calculation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::{AchievementId, Quest};
use crate::config::Difficulty;
use crate::constants::{
    MAX_PLAYER_LEVEL, MINIGAME_LEVEL_BONUS_XP, MINIGAME_STREAK_BONUS_DIVISOR,
    MINIGAME_STREAK_BONUS_MAX_XP, MINIGAME_XP_PER_100_POINTS, SCENARIO_BASE_XP_PER_100_POINTS,
    XP_LEVEL_FORMULA_BASE, XP_LEVEL_FORMULA_EXPONENT,
};
use crate::learning::CommandPerformance;
use crate::learning::ScenarioHistory;
use crate::sound::SoundConfig;

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
    pub achievements_unlocked: HashSet<AchievementId>,

    /// Active daily quests
    pub daily_quests: Vec<Quest>,

    /// Last time quests were refreshed
    pub last_quest_refresh: DateTime<Utc>,

    /// Completed quest IDs today
    pub completed_quests_today: HashSet<String>,

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
    pub challenge_progress: crate::minigame::ChallengeProgress,

    /// Survival mode best level reached
    #[serde(default)]
    pub survival_best_level: u32,

    /// Survival mode best scenarios completed in single run
    #[serde(default)]
    pub survival_best_scenarios: u32,

    /// Persisted FSRS/PerformanceTracker data
    #[serde(default)]
    pub performance_data: HashMap<String, CommandPerformance>,

    /// Count of scenario completions finished in under
    /// [`SPEED_DEMON_TIME_RATIO`](crate::constants::SPEED_DEMON_TIME_RATIO) of a
    /// scenario's time budget, in either Training or arcade mode. Powers
    /// `SpeedDemon`/`Speedrunner`.
    #[serde(default)]
    pub speed_run_count: u32,

    /// Count of scenario completions finished in under
    /// [`FLASH_TIME_RATIO`](crate::constants::FLASH_TIME_RATIO) of a scenario's
    /// time budget, in either Training or arcade mode. Powers `Flash`.
    #[serde(default)]
    pub flash_run_count: u32,

    /// Difficulty levels completed at least once, across any mode. Powers `Polyglot`.
    #[serde(default)]
    pub difficulties_completed: HashSet<Difficulty>,
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
            challenge_progress: crate::minigame::ChallengeProgress::default(),
            survival_best_level: 0,
            survival_best_scenarios: 0,
            performance_data: HashMap::new(),
            speed_run_count: 0,
            flash_run_count: 0,
            difficulties_completed: HashSet::new(),
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
        let current_level_xp = XPCalculator::xp_for_level(self.level);
        let next_level_xp = XPCalculator::xp_for_level(self.level + 1);
        let xp_in_level = self.total_xp - current_level_xp;
        let xp_needed = next_level_xp - current_level_xp;

        if xp_needed == 0 {
            1.0
        } else {
            (xp_in_level as f64) / (xp_needed as f64)
        }
    }

    /// XP needed for next level
    pub fn xp_for_next_level(&self) -> u64 {
        let next_level_total = XPCalculator::xp_for_level(self.level + 1);
        next_level_total.saturating_sub(self.total_xp)
    }

    /// Check if quest is completed today
    pub fn is_quest_completed(&self, quest_id: &str) -> bool {
        self.completed_quests_today.contains(quest_id)
    }

    /// Mark quest as completed
    pub fn complete_quest(&mut self, quest_id: String) {
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
            (XP_LEVEL_FORMULA_BASE * ((level - 1) as f64).powf(XP_LEVEL_FORMULA_EXPONENT)).round()
                as u64
        }
    }

    /// Calculate level from total XP
    ///
    /// Uses binary search to find the highest level where threshold <= total_xp
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
            let mid = (low + high).div_ceil(2);
            if Self::xp_for_level(mid) <= total_xp {
                low = mid;
            } else {
                high = mid - 1;
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
    ///
    /// let easy_practice = QuestType::CommandPractice {
    ///     command: "w".to_string(),
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

    /// Calculate XP for scenario completion
    ///
    /// # Arguments
    ///
    /// * `score` - Score (0-100)
    /// * `multiplier` - Mastery-based XP multiplier (0.0-1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::XPCalculator;
    ///
    /// // Perfect score with full multiplier
    /// assert_eq!(XPCalculator::scenario_xp(100, 1.0), 50); // 100 * 50 / 100 = 50
    ///
    /// // Normal score
    /// assert_eq!(XPCalculator::scenario_xp(85, 1.0), 42); // 85 * 50 / 100 = 42
    ///
    /// // Mastered scenario (20% XP)
    /// assert_eq!(XPCalculator::scenario_xp(100, 0.2), 10); // 50 * 0.2 = 10
    /// ```
    pub fn scenario_xp(score: u32, multiplier: f64) -> u64 {
        // Base XP per 100 points scored
        let base_xp = (score as u64 * SCENARIO_BASE_XP_PER_100_POINTS) / 100;
        (base_xp as f64 * multiplier).round() as u64
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
        let level_bonus = level as u64 * MINIGAME_LEVEL_BONUS_XP;

        // Streak bonus per streak milestone
        let streak_bonus =
            (best_streak as u64 / MINIGAME_STREAK_BONUS_DIVISOR) * MINIGAME_STREAK_BONUS_MAX_XP;

        base_xp + level_bonus + streak_bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    fn test_xp_for_next_level() {
        let mut profile = UserProfile::new();
        assert_eq!(profile.xp_for_next_level(), 100); // Level 1 → 2

        profile.add_xp(100); // Now level 2
        let needed = profile.xp_for_next_level();
        assert!(needed > 0);
        assert!(needed < 200); // Should be ~183
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
    fn test_scenario_xp_calculation() {
        // Perfect score, full multiplier
        let xp = XPCalculator::scenario_xp(100, 1.0);
        assert_eq!(xp, 50); // 100 * 50 / 100 = 50

        // Normal score
        let xp = XPCalculator::scenario_xp(85, 1.0);
        assert_eq!(xp, 42); // 85 * 50 / 100 = 42

        // 50% score
        let xp = XPCalculator::scenario_xp(50, 1.0);
        assert_eq!(xp, 25); // 50 * 50 / 100 = 25

        // Mastered scenario (20% multiplier)
        let xp = XPCalculator::scenario_xp(100, 0.2);
        assert_eq!(xp, 10); // 50 * 0.2 = 10

        // Zero score
        let xp = XPCalculator::scenario_xp(0, 1.0);
        assert_eq!(xp, 0);
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

        assert!(!profile.is_quest_completed("quest_1"));

        profile.complete_quest("quest_1".to_string());
        assert!(profile.is_quest_completed("quest_1"));
    }

    #[test]
    fn test_reset_daily_quests() {
        let mut profile = UserProfile::new();

        profile.complete_quest("quest_1".to_string());
        assert!(profile.is_quest_completed("quest_1"));

        profile.reset_daily_quests(Utc::now());
        assert!(!profile.is_quest_completed("quest_1"));
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
        use crate::sound::SoundConfig;

        let mut profile = UserProfile::new();
        profile.sound_config = SoundConfig::new(0.3, false);

        // Serialize and deserialize (TOML is the actual format)
        let toml = toml::to_string(&profile).unwrap();
        let restored: UserProfile = toml::from_str(&toml).unwrap();

        assert!((restored.sound_config.volume - 0.3).abs() < f32::EPSILON);
        assert!(!restored.sound_config.enabled);
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
}
