//! User profile and XP calculation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::{AchievementId, Quest};
use crate::learning::ScenarioHistory;

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
        let now = Utc::now();
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
    pub fn reset_daily_quests(&mut self) {
        self.completed_quests_today.clear();
        self.daily_quests.clear();
        self.last_quest_refresh = Utc::now();
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
    /// assert_eq!(XPCalculator::xp_for_level(3), 282);    // Level 3 starts at ~282
    /// ```
    pub fn xp_for_level(level: u32) -> u64 {
        if level <= 1 {
            0
        } else {
            (100.0 * ((level - 1) as f64).powf(1.5)) as u64
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
    /// assert_eq!(XPCalculator::level_from_xp(282), 3);
    /// ```
    pub fn level_from_xp(total_xp: u64) -> u32 {
        // Binary search for level
        let mut low = 1u32;
        let mut high = 100u32; // Arbitrary max level

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
    ///     command: "dd".to_string(),
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
        // Base XP: 50 per 100 points
        let base_xp = (score as u64 * 50) / 100;
        (base_xp as f64 * multiplier) as u64
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
        assert_eq!(XPCalculator::xp_for_level(3), 282); // 100 * 2^1.5 = 282.8

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
        assert_eq!(XPCalculator::level_from_xp(282), 3);
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

        profile.reset_daily_quests();
        assert!(!profile.is_quest_completed("quest_1"));
        assert!(profile.daily_quests.is_empty());
    }
}
