//! Achievement system and tracking

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::learning::{MasteryLevel, PerformanceTracker};

use super::UserProfile;

/// Unique achievement identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementId {
    // Streak achievements
    Streak7Days,
    Streak30Days,
    Streak90Days,
    Streak365Days,

    // Mastery achievements
    FirstPerfect,
    Perfect10,
    Perfect100,
    MasterOneCommand,
    MasterTenCommands,

    // Speed achievements
    SpeedDemon,  // Complete scenario in < 50% optimal time
    Speedrunner, // Complete 10 speed runs
    Flash,       // Complete scenario in < 25% optimal time

    // Exploration achievements
    JackOfAllTrades, // Use 20 different commands
    Specialist,      // Master a single command
    Polyglot,        // Complete scenarios in all difficulties

    // Milestone achievements
    Centurion, // Complete 100 scenarios
    Veteran,   // Complete 500 scenarios
    Legend,    // Complete 1000 scenarios
}

/// Achievement metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: AchievementId,
    pub name: String,
    pub description: String,
    pub unlocked_at: Option<DateTime<Utc>>,
}

impl Achievement {
    /// Create achievement metadata
    pub fn new(id: AchievementId) -> Self {
        let (name, description) = Self::metadata(id);
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            unlocked_at: None,
        }
    }

    /// Get achievement name and description
    fn metadata(id: AchievementId) -> (&'static str, &'static str) {
        match id {
            // Streaks
            AchievementId::Streak7Days => ("7-Day Warrior", "Practice every day for a week"),
            AchievementId::Streak30Days => ("Month Master", "Maintain a 30-day practice streak"),
            AchievementId::Streak90Days => {
                ("Quarter Champion", "Maintain a 90-day practice streak")
            }
            AchievementId::Streak365Days => ("Year Legend", "Maintain a 365-day practice streak"),

            // Mastery
            AchievementId::FirstPerfect => {
                ("First Perfect", "Complete a scenario with perfect score")
            }
            AchievementId::Perfect10 => ("Perfect 10", "Complete 10 scenarios perfectly"),
            AchievementId::Perfect100 => ("Perfectionist", "Complete 100 scenarios perfectly"),
            AchievementId::MasterOneCommand => ("Command Master", "Master a single command"),
            AchievementId::MasterTenCommands => ("Master of Many", "Master 10 different commands"),

            // Speed
            AchievementId::SpeedDemon => (
                "Speed Demon",
                "Complete a scenario in under 50% of optimal time",
            ),
            AchievementId::Speedrunner => ("Speedrunner", "Complete 10 speed challenges"),
            AchievementId::Flash => ("Flash", "Complete a scenario in under 25% of optimal time"),

            // Exploration
            AchievementId::JackOfAllTrades => ("Jack of All Trades", "Use 20 different commands"),
            AchievementId::Specialist => ("Specialist", "Master a single command to perfection"),
            AchievementId::Polyglot => ("Polyglot", "Complete scenarios of all difficulty levels"),

            // Milestones
            AchievementId::Centurion => ("Centurion", "Complete 100 scenarios"),
            AchievementId::Veteran => ("Veteran", "Complete 500 scenarios"),
            AchievementId::Legend => ("Legend", "Complete 1000 scenarios"),
        }
    }

    /// Mark achievement as unlocked
    pub fn unlock(&mut self) {
        self.unlocked_at = Some(Utc::now());
    }

    /// Check if unlocked
    pub fn is_unlocked(&self) -> bool {
        self.unlocked_at.is_some()
    }
}

/// Achievement engine that checks conditions
pub struct AchievementEngine;

impl AchievementEngine {
    /// Check all achievement conditions and return newly unlocked achievements
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{AchievementEngine, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    ///
    /// let mut profile = UserProfile::new();
    /// profile.perfect_scenarios = 1;
    ///
    /// let tracker = PerformanceTracker::new();
    /// let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    ///
    /// // Should unlock FirstPerfect
    /// assert!(unlocked.iter().any(|id| matches!(id, helix_trainer::gamification::AchievementId::FirstPerfect)));
    /// ```
    pub fn check_achievements(
        profile: &UserProfile,
        tracker: &PerformanceTracker,
    ) -> Vec<AchievementId> {
        let mut unlocked = Vec::new();

        // Streak achievements
        Self::check_streak_achievements(profile, &mut unlocked);

        // Mastery achievements
        Self::check_mastery_achievements(profile, tracker, &mut unlocked);

        // Exploration achievements
        Self::check_exploration_achievements(profile, tracker, &mut unlocked);

        // Milestone achievements
        Self::check_milestone_achievements(profile, &mut unlocked);

        unlocked
    }

    /// Check achievement conditions and unlock any newly satisfied achievements on `profile`
    ///
    /// Returns the ids that were newly unlocked by this call, for the caller to surface
    /// (e.g. as notifications).
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{AchievementEngine, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    ///
    /// let mut profile = UserProfile::new();
    /// profile.perfect_scenarios = 1;
    ///
    /// let tracker = PerformanceTracker::new();
    /// let unlocked = AchievementEngine::check_and_unlock(&mut profile, &tracker);
    ///
    /// assert!(profile.has_achievement(&helix_trainer::gamification::AchievementId::FirstPerfect));
    /// assert_eq!(unlocked.len(), 1);
    /// ```
    pub fn check_and_unlock(
        profile: &mut UserProfile,
        tracker: &PerformanceTracker,
    ) -> Vec<AchievementId> {
        let newly_unlocked = Self::check_achievements(profile, tracker);
        for &id in &newly_unlocked {
            profile.unlock_achievement(id);
        }
        newly_unlocked
    }

    fn check_streak_achievements(profile: &UserProfile, unlocked: &mut Vec<AchievementId>) {
        let streak = profile.current_streak;

        if streak >= 365 && !profile.has_achievement(&AchievementId::Streak365Days) {
            unlocked.push(AchievementId::Streak365Days);
        }
        if streak >= 90 && !profile.has_achievement(&AchievementId::Streak90Days) {
            unlocked.push(AchievementId::Streak90Days);
        }
        if streak >= 30 && !profile.has_achievement(&AchievementId::Streak30Days) {
            unlocked.push(AchievementId::Streak30Days);
        }
        if streak >= 7 && !profile.has_achievement(&AchievementId::Streak7Days) {
            unlocked.push(AchievementId::Streak7Days);
        }
    }

    fn check_mastery_achievements(
        profile: &UserProfile,
        tracker: &PerformanceTracker,
        unlocked: &mut Vec<AchievementId>,
    ) {
        // Perfect scenario achievements
        let perfect_count = profile.perfect_scenarios;

        if perfect_count >= 100 && !profile.has_achievement(&AchievementId::Perfect100) {
            unlocked.push(AchievementId::Perfect100);
        }
        if perfect_count >= 10 && !profile.has_achievement(&AchievementId::Perfect10) {
            unlocked.push(AchievementId::Perfect10);
        }
        if perfect_count >= 1 && !profile.has_achievement(&AchievementId::FirstPerfect) {
            unlocked.push(AchievementId::FirstPerfect);
        }

        // Command mastery achievements
        let mastered_commands = tracker
            .all_commands()
            .iter()
            .filter(|cmd| {
                tracker
                    .get_performance(cmd)
                    .map(|perf| perf.mastery_level == MasteryLevel::Master)
                    .unwrap_or(false)
            })
            .count();

        if mastered_commands >= 10 && !profile.has_achievement(&AchievementId::MasterTenCommands) {
            unlocked.push(AchievementId::MasterTenCommands);
        }
        if mastered_commands >= 1 && !profile.has_achievement(&AchievementId::MasterOneCommand) {
            unlocked.push(AchievementId::MasterOneCommand);
        }

        // Specialist (same as MasterOneCommand, but kept separate for clarity)
        if mastered_commands >= 1 && !profile.has_achievement(&AchievementId::Specialist) {
            unlocked.push(AchievementId::Specialist);
        }
    }

    fn check_exploration_achievements(
        profile: &UserProfile,
        tracker: &PerformanceTracker,
        unlocked: &mut Vec<AchievementId>,
    ) {
        // Jack of all trades - 20 different commands used
        let commands_used = tracker.all_commands().len();

        if commands_used >= 20 && !profile.has_achievement(&AchievementId::JackOfAllTrades) {
            unlocked.push(AchievementId::JackOfAllTrades);
        }
    }

    fn check_milestone_achievements(profile: &UserProfile, unlocked: &mut Vec<AchievementId>) {
        let completed = profile.scenarios_completed;

        if completed >= 1000 && !profile.has_achievement(&AchievementId::Legend) {
            unlocked.push(AchievementId::Legend);
        }
        if completed >= 500 && !profile.has_achievement(&AchievementId::Veteran) {
            unlocked.push(AchievementId::Veteran);
        }
        if completed >= 100 && !profile.has_achievement(&AchievementId::Centurion) {
            unlocked.push(AchievementId::Centurion);
        }
    }

    /// Get all possible achievements
    pub fn all_achievements() -> Vec<Achievement> {
        use AchievementId::*;
        vec![
            // Streaks
            Streak7Days,
            Streak30Days,
            Streak90Days,
            Streak365Days,
            // Mastery
            FirstPerfect,
            Perfect10,
            Perfect100,
            MasterOneCommand,
            MasterTenCommands,
            // Speed
            SpeedDemon,
            Speedrunner,
            Flash,
            // Exploration
            JackOfAllTrades,
            Specialist,
            Polyglot,
            // Milestones
            Centurion,
            Veteran,
            Legend,
        ]
        .into_iter()
        .map(Achievement::new)
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_creation() {
        let achievement = Achievement::new(AchievementId::FirstPerfect);
        assert_eq!(achievement.id, AchievementId::FirstPerfect);
        assert_eq!(achievement.name, "First Perfect");
        assert!(!achievement.is_unlocked());
    }

    #[test]
    fn test_achievement_unlock() {
        let mut achievement = Achievement::new(AchievementId::FirstPerfect);
        assert!(!achievement.is_unlocked());

        achievement.unlock();
        assert!(achievement.is_unlocked());
        assert!(achievement.unlocked_at.is_some());
    }

    #[test]
    fn test_all_achievements_complete() {
        let achievements = AchievementEngine::all_achievements();
        assert_eq!(achievements.len(), 18); // Total number of achievements
    }

    #[test]
    fn test_streak_achievement_detection() {
        let tracker = PerformanceTracker::new();
        let mut profile = UserProfile::new();

        // No streak
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.is_empty());

        // 7-day streak
        profile.current_streak = 7;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Streak7Days));

        // Don't unlock again
        profile.unlock_achievement(AchievementId::Streak7Days);
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(!unlocked.contains(&AchievementId::Streak7Days));

        // 30-day streak
        profile.current_streak = 30;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Streak30Days));
    }

    #[test]
    fn test_perfect_scenario_achievements() {
        let tracker = PerformanceTracker::new();
        let mut profile = UserProfile::new();

        // First perfect
        profile.perfect_scenarios = 1;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::FirstPerfect));

        // 10 perfect
        profile.unlock_achievement(AchievementId::FirstPerfect);
        profile.perfect_scenarios = 10;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Perfect10));

        // 100 perfect
        profile.unlock_achievement(AchievementId::Perfect10);
        profile.perfect_scenarios = 100;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Perfect100));
    }

    #[test]
    fn test_milestone_achievements() {
        let tracker = PerformanceTracker::new();
        let mut profile = UserProfile::new();

        // 100 scenarios
        profile.scenarios_completed = 100;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Centurion));

        // 500 scenarios
        profile.unlock_achievement(AchievementId::Centurion);
        profile.scenarios_completed = 500;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Veteran));

        // 1000 scenarios
        profile.unlock_achievement(AchievementId::Veteran);
        profile.scenarios_completed = 1000;
        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::Legend));
    }

    /// Regression test: retroactively checking an existing profile (e.g. on load) must
    /// unlock every satisfied tier at once, not just the highest one. Previously an
    /// `else if` cascade meant a profile with `scenarios_completed = 600` unlocked only
    /// `Veteran` (>= 500) and could never also unlock `Centurion` (>= 100).
    #[test]
    fn test_milestone_achievements_unlock_all_satisfied_tiers_at_once() {
        let tracker = PerformanceTracker::new();
        let mut profile = UserProfile::new();
        profile.scenarios_completed = 600;

        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);

        assert!(unlocked.contains(&AchievementId::Centurion));
        assert!(unlocked.contains(&AchievementId::Veteran));
        assert!(!unlocked.contains(&AchievementId::Legend));
    }

    #[test]
    fn test_exploration_jack_of_all_trades() {
        let mut tracker = PerformanceTracker::new();
        let profile = UserProfile::new();

        // Use 20 different commands
        for i in 0..20 {
            let cmd = format!("cmd{}", i);
            tracker.record_attempt(
                &cmd,
                std::time::Duration::from_secs(1),
                true,
                std::time::Duration::from_secs(1),
            );
        }

        let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
        assert!(unlocked.contains(&AchievementId::JackOfAllTrades));
    }

    #[test]
    fn test_check_and_unlock_marks_profile_and_returns_ids() {
        let tracker = PerformanceTracker::new();
        let mut profile = UserProfile::new();
        profile.perfect_scenarios = 1;

        let unlocked = AchievementEngine::check_and_unlock(&mut profile, &tracker);

        assert_eq!(unlocked, vec![AchievementId::FirstPerfect]);
        assert!(profile.has_achievement(&AchievementId::FirstPerfect));

        // Calling again should not re-unlock the same achievement
        let unlocked_again = AchievementEngine::check_and_unlock(&mut profile, &tracker);
        assert!(unlocked_again.is_empty());
    }

    #[test]
    fn test_achievement_metadata_all_variants() {
        // Ensure all achievement IDs have metadata
        for achievement in AchievementEngine::all_achievements() {
            assert!(!achievement.name.is_empty());
            assert!(!achievement.description.is_empty());
        }
    }
}
