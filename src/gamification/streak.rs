//! Streak tracking and management

use chrono::{DateTime, Utc};

use super::{GamificationError, Result, UserProfile};
use crate::constants::{
    MILESTONE_7_DAY_XP, MILESTONE_30_DAY_XP, MILESTONE_90_DAY_XP, MILESTONE_365_DAY_XP,
    STREAK_MILESTONE_MONTH, STREAK_MILESTONE_QUARTER, STREAK_MILESTONE_WEEK, STREAK_MILESTONE_YEAR,
};

/// Streak change event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakChange {
    /// Streak continued (same day activity)
    Continued,
    /// Streak incremented (new day)
    Incremented { new_streak: u32 },
    /// Streak broken (missed day)
    Broken { was_streak: u32 },
    /// Streak protected by freeze
    Protected { used_freeze: bool },
}

/// Manages daily practice streaks
pub struct StreakManager;

impl StreakManager {
    /// Update streak based on current activity
    ///
    /// Checks if user has been active since last recorded activity:
    /// - Same day: No change
    /// - Next day (completed quest): Increment streak
    /// - Missed day: Break streak (or use freeze if available)
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{StreakManager, UserProfile, StreakChange};
    /// use helix_trainer::time::{Clock, FakeClock};
    ///
    /// let clock = FakeClock::at("2026-01-15T12:00:00Z");
    /// let mut profile = UserProfile::new_at(clock.now());
    /// profile.current_streak = 5;
    /// profile.complete_quest("test_quest".to_string());
    ///
    /// // Simulate next day activity
    /// clock.advance_days(1);
    ///
    /// let change = StreakManager::update_streak(&mut profile, clock.now());
    /// assert_eq!(change, StreakChange::Incremented { new_streak: 6 });
    /// assert_eq!(profile.current_streak, 6);
    /// ```
    pub fn update_streak(profile: &mut UserProfile, now: DateTime<Utc>) -> StreakChange {
        let last_active = profile.last_activity;

        let days_since_active = Self::days_between(last_active, now);

        let change = match days_since_active {
            0 => {
                // Same day - no change
                StreakChange::Continued
            }
            1 => {
                // Next day - increment if completed any quest today
                if !profile.completed_quests_today.is_empty() {
                    profile.current_streak += 1;
                    profile.longest_streak = profile.longest_streak.max(profile.current_streak);
                    StreakChange::Incremented {
                        new_streak: profile.current_streak,
                    }
                } else {
                    // Activity but no quest completion - don't increment
                    StreakChange::Continued
                }
            }
            _ => {
                // Missed day(s)
                let was_streak = profile.current_streak;

                if profile.streak_freeze_available {
                    // Use streak freeze
                    profile.streak_freeze_available = false;
                    StreakChange::Protected { used_freeze: true }
                } else {
                    // Break streak
                    profile.current_streak = 0;
                    StreakChange::Broken { was_streak }
                }
            }
        };

        profile.last_activity = now;
        change
    }

    /// Use streak freeze manually
    ///
    /// # Errors
    ///
    /// Returns error if freeze not available
    pub fn use_freeze(profile: &mut UserProfile) -> Result<()> {
        if !profile.streak_freeze_available {
            return Err(GamificationError::StreakFreezeUnavailable);
        }

        profile.streak_freeze_available = false;
        Ok(())
    }

    /// Grant streak freeze (earned by completing all of today's daily quests)
    pub fn grant_freeze(profile: &mut UserProfile) {
        profile.streak_freeze_available = true;
    }

    /// Check if user should be granted a freeze
    ///
    /// Eligible once every quest generated for today has been completed, and no
    /// freeze is currently held. A profile with no quests generated yet is never
    /// eligible.
    pub fn check_freeze_eligibility(profile: &UserProfile) -> bool {
        !profile.streak_freeze_available
            && !profile.daily_quests.is_empty()
            && profile.completed_quests_today.len() >= profile.daily_quests.len()
    }

    /// Calculate days between two dates
    fn days_between(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
        (end.date_naive() - start.date_naive()).num_days()
    }

    /// Check if streak milestone reached
    pub fn milestone_reached(streak: u32) -> Option<u32> {
        match streak {
            s if s == STREAK_MILESTONE_WEEK => Some(s),
            s if s == STREAK_MILESTONE_MONTH => Some(s),
            s if s == STREAK_MILESTONE_QUARTER => Some(s),
            s if s == STREAK_MILESTONE_YEAR => Some(s),
            _ => None,
        }
    }

    /// Calculate XP bonus for streak milestone
    pub fn milestone_xp_bonus(streak: u32) -> u64 {
        match streak {
            s if s == STREAK_MILESTONE_WEEK => MILESTONE_7_DAY_XP,
            s if s == STREAK_MILESTONE_MONTH => MILESTONE_30_DAY_XP,
            s if s == STREAK_MILESTONE_QUARTER => MILESTONE_90_DAY_XP,
            s if s == STREAK_MILESTONE_YEAR => MILESTONE_365_DAY_XP,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::{Clock, FakeClock};
    use chrono::Duration;

    #[test]
    fn test_streak_same_day_continues() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 5;

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(change, StreakChange::Continued);
        assert_eq!(profile.current_streak, 5);
    }

    #[test]
    fn test_streak_increments_next_day() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 5;
        profile.complete_quest("test_quest".to_string());
        clock.advance_days(1);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(change, StreakChange::Incremented { new_streak: 6 });
        assert_eq!(profile.current_streak, 6);
    }

    #[test]
    fn test_streak_doesnt_increment_without_quest() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 5;
        clock.advance_days(1);
        // No quest completed

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(change, StreakChange::Continued);
        assert_eq!(profile.current_streak, 5);
    }

    #[test]
    fn test_streak_breaks_when_missed() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 10;
        clock.advance_days(2);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(change, StreakChange::Broken { was_streak: 10 });
        assert_eq!(profile.current_streak, 0);
    }

    #[test]
    fn test_streak_freeze_protects() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 10;
        profile.streak_freeze_available = true;
        clock.advance_days(2);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(change, StreakChange::Protected { used_freeze: true });
        assert_eq!(profile.current_streak, 10); // Streak preserved
        assert!(!profile.streak_freeze_available); // Freeze consumed
    }

    #[test]
    fn test_longest_streak_updates() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 5;
        profile.longest_streak = 5;
        profile.complete_quest("test".to_string());
        clock.advance_days(1);

        StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(profile.longest_streak, 6);
    }

    #[test]
    fn test_use_freeze_manually() {
        let mut profile = UserProfile::new();
        profile.streak_freeze_available = true;

        let result = StreakManager::use_freeze(&mut profile);
        assert!(result.is_ok());
        assert!(!profile.streak_freeze_available);

        // Try again - should fail
        let result = StreakManager::use_freeze(&mut profile);
        assert!(result.is_err());
    }

    #[test]
    fn test_grant_freeze() {
        let mut profile = UserProfile::new();
        assert!(!profile.streak_freeze_available);

        StreakManager::grant_freeze(&mut profile);
        assert!(profile.streak_freeze_available);
    }

    #[test]
    fn test_freeze_eligibility() {
        use crate::gamification::{Quest, QuestDifficulty, QuestType};

        let mut profile = UserProfile::new();

        // Not eligible - no quests generated for today
        assert!(!StreakManager::check_freeze_eligibility(&profile));

        for i in 0..3 {
            profile.daily_quests.push(Quest::new(
                format!("quest_{}", i),
                QuestType::CommandPractice {
                    command: "x".to_string(),
                    target: 1,
                    current: 0,
                },
                format!("Quest {}", i),
                QuestDifficulty::Easy,
            ));
        }

        // Not eligible - quests generated but none completed yet
        assert!(!StreakManager::check_freeze_eligibility(&profile));

        // Not eligible - already has freeze, even if all quests are completed
        profile.streak_freeze_available = true;
        for i in 0..3 {
            profile.complete_quest(format!("quest_{}", i));
        }
        assert!(!StreakManager::check_freeze_eligibility(&profile));

        // Eligible - all of today's quests completed, no freeze held
        profile.streak_freeze_available = false;
        assert!(StreakManager::check_freeze_eligibility(&profile));
    }

    #[test]
    fn test_milestone_detection() {
        assert_eq!(StreakManager::milestone_reached(7), Some(7));
        assert_eq!(StreakManager::milestone_reached(30), Some(30));
        assert_eq!(StreakManager::milestone_reached(90), Some(90));
        assert_eq!(StreakManager::milestone_reached(365), Some(365));
        assert_eq!(StreakManager::milestone_reached(15), None);
    }

    #[test]
    fn test_milestone_xp_bonus() {
        assert_eq!(StreakManager::milestone_xp_bonus(7), 50);
        assert_eq!(StreakManager::milestone_xp_bonus(30), 200);
        assert_eq!(StreakManager::milestone_xp_bonus(90), 500);
        assert_eq!(StreakManager::milestone_xp_bonus(365), 2000);
        assert_eq!(StreakManager::milestone_xp_bonus(15), 0);
    }

    #[test]
    fn test_days_between() {
        let now = Utc::now();
        let yesterday = now - Duration::days(1);
        let week_ago = now - Duration::days(7);

        assert_eq!(StreakManager::days_between(now, now), 0);
        assert_eq!(StreakManager::days_between(yesterday, now), 1);
        assert_eq!(StreakManager::days_between(week_ago, now), 7);
    }
}
