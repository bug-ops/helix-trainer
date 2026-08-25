//! Streak tracking and management

use chrono::{DateTime, Utc};

use super::{GamificationError, Result, UserProfile};
use crate::constants::{
    MILESTONE_7_DAY_XP, MILESTONE_30_DAY_XP, MILESTONE_90_DAY_XP, MILESTONE_365_DAY_XP,
    STREAK_FREEZE_MAX_GAP_DAYS, STREAK_MILESTONE_MONTH, STREAK_MILESTONE_QUARTER,
    STREAK_MILESTONE_WEEK, STREAK_MILESTONE_YEAR,
};

/// Streak change event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakChange {
    /// Streak continued (same day activity)
    Continued,
    /// Streak incremented (new day)
    Incremented { new_streak: u32 },
    /// Streak broken (missed day)
    ///
    /// `freeze_could_not_cover_gap` is true exactly when a freeze was held
    /// but the gap exceeded [`STREAK_FREEZE_MAX_GAP_DAYS`] (so the freeze
    /// was left unconsumed, held for next time); false for every other
    /// break reason (no freeze held, or a non-positive/backwards-clock
    /// gap). Only meaningful when `was_streak > 0` - callers only surface
    /// this reason when there was an actual streak at stake.
    Broken {
        was_streak: u32,
        freeze_could_not_cover_gap: bool,
    },
    /// Streak protected by freeze
    Protected,
}

/// Manages daily practice streaks
pub struct StreakManager;

impl StreakManager {
    /// Update streak based on current activity
    ///
    /// Checks if user has been active since last recorded activity:
    /// - Same day: No change
    /// - Next day (completed quest): Increment streak
    /// - Missed day(s) within [`STREAK_FREEZE_MAX_GAP_DAYS`]: Break streak, or use
    ///   freeze if available and the prior streak was non-zero
    /// - Missed day(s) beyond [`STREAK_FREEZE_MAX_GAP_DAYS`]: Break streak, even if
    ///   a freeze is available - it only protects a brief lapse (e.g. a
    ///   Friday-to-Monday weekend gap), not an arbitrary absence
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
                let freeze_available_before = profile.streak_freeze_available;

                if Self::try_consume_freeze(profile, days_since_active) {
                    StreakChange::Protected
                } else {
                    // Break streak (no-op if already 0 - nothing was there to protect)
                    profile.current_streak = 0;
                    StreakChange::Broken {
                        was_streak,
                        freeze_could_not_cover_gap: freeze_available_before
                            && days_since_active > STREAK_FREEZE_MAX_GAP_DAYS,
                    }
                }
            }
        };

        profile.last_activity = now;
        change
    }

    /// Use streak freeze manually to cover a gap of `days_since_active` days
    ///
    /// Applies the same coverage policy as the freeze branch of
    /// [`Self::update_streak`]: the freeze only protects a non-zero streak
    /// across a gap in `2..=STREAK_FREEZE_MAX_GAP_DAYS`. Fails loud rather
    /// than silently no-oping when the gap is out of that range - the
    /// freeze is left untouched (held for next time), consistent with
    /// `update_streak`.
    ///
    /// # Errors
    ///
    /// Returns an error if no freeze is available, if there is no active
    /// streak to protect, or if `days_since_active` falls outside the
    /// coverable range.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{StreakManager, UserProfile};
    ///
    /// let mut profile = UserProfile::new();
    /// profile.current_streak = 5;
    /// StreakManager::grant_freeze(&mut profile);
    ///
    /// // A 2-day gap is within the freeze's coverage.
    /// StreakManager::use_freeze(&mut profile, 2).unwrap();
    /// assert!(!profile.streak_freeze_available);
    /// ```
    pub fn use_freeze(profile: &mut UserProfile, days_since_active: i64) -> Result<()> {
        if !profile.streak_freeze_available {
            return Err(GamificationError::StreakFreezeUnavailable);
        }

        if profile.current_streak == 0 {
            return Err(GamificationError::StreakFreezeNothingToProtect);
        }

        if Self::try_consume_freeze(profile, days_since_active) {
            Ok(())
        } else {
            Err(GamificationError::StreakFreezeGapOutOfRange {
                days_since_active,
                max_gap_days: STREAK_FREEZE_MAX_GAP_DAYS,
            })
        }
    }

    /// Try to consume a streak freeze to cover a gap of `days_since_active`
    /// days.
    ///
    /// Shared coverage policy for [`Self::update_streak`] and
    /// [`Self::use_freeze`]: the freeze only applies to a non-zero streak
    /// across a gap in `2..=STREAK_FREEZE_MAX_GAP_DAYS` (a brief lapse, not
    /// an arbitrary absence). The lower bound also rejects a negative gap
    /// from a backwards clock/NTP correction, which would otherwise satisfy
    /// a `<=`-only upper bound. Returns `true` and consumes the freeze if it
    /// applied; otherwise leaves the freeze untouched and returns `false`.
    fn try_consume_freeze(profile: &mut UserProfile, days_since_active: i64) -> bool {
        if profile.current_streak > 0
            && profile.streak_freeze_available
            && (2..=STREAK_FREEZE_MAX_GAP_DAYS).contains(&days_since_active)
        {
            profile.streak_freeze_available = false;
            true
        } else {
            false
        }
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
    use std::assert_matches;

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
        assert_eq!(
            change,
            StreakChange::Broken {
                was_streak: 10,
                freeze_could_not_cover_gap: false
            }
        );
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
        assert_eq!(change, StreakChange::Protected);
        assert_eq!(profile.current_streak, 10); // Streak preserved
        assert!(!profile.streak_freeze_available); // Freeze consumed
    }

    /// Regression test for #325: a streak freeze must not protect a gap longer
    /// than [`STREAK_FREEZE_MAX_GAP_DAYS`] - it should break normally, and the
    /// freeze must remain available since it wasn't actually usable here.
    #[test]
    fn test_streak_freeze_does_not_cover_long_gap() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 10;
        profile.streak_freeze_available = true;
        clock.advance_days(STREAK_FREEZE_MAX_GAP_DAYS + 1);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(
            change,
            StreakChange::Broken {
                was_streak: 10,
                freeze_could_not_cover_gap: true
            }
        );
        assert_eq!(profile.current_streak, 0);
        assert!(
            profile.streak_freeze_available,
            "freeze must not be consumed when the gap exceeds the coverage cap"
        );
    }

    /// Boundary case: a gap of exactly `STREAK_FREEZE_MAX_GAP_DAYS` must still be
    /// covered by the freeze (the cap is inclusive, per the `..=` range in
    /// `update_streak`).
    #[test]
    fn test_streak_freeze_protects_at_max_gap_boundary() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 10;
        profile.streak_freeze_available = true;
        clock.advance_days(STREAK_FREEZE_MAX_GAP_DAYS);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(change, StreakChange::Protected);
        assert_eq!(profile.current_streak, 10);
        assert!(!profile.streak_freeze_available);
    }

    /// A gap beyond the cap with no freeze held must behave exactly as it did
    /// before this fix (freeze-unavailable path was never gap-length-gated).
    #[test]
    fn test_streak_breaks_beyond_cap_without_freeze() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 10;
        profile.streak_freeze_available = false;
        clock.advance_days(STREAK_FREEZE_MAX_GAP_DAYS + 1);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(
            change,
            StreakChange::Broken {
                was_streak: 10,
                freeze_could_not_cover_gap: false
            }
        );
        assert_eq!(profile.current_streak, 0);
    }

    /// A gap beyond the cap with `current_streak` already 0: the #319 zero-streak
    /// guard and the new gap-length guard must not conflict - freeze stays held
    /// (nothing to protect either way) and the streak stays 0.
    #[test]
    fn test_streak_freeze_untouched_beyond_cap_when_streak_already_zero() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 0;
        profile.streak_freeze_available = true;
        clock.advance_days(STREAK_FREEZE_MAX_GAP_DAYS + 1);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(
            change,
            StreakChange::Broken {
                was_streak: 0,
                freeze_could_not_cover_gap: true
            }
        );
        assert_eq!(profile.current_streak, 0);
        assert!(
            profile.streak_freeze_available,
            "freeze must not be consumed when there was no streak to protect"
        );
    }

    /// Regression test for a backwards-clock edge case: a negative
    /// `days_since_active` (e.g. an NTP correction moving the clock backwards)
    /// must not be treated as a coverable gap - the lower bound of the freeze
    /// window guards against a bare `<=` upper-bound check silently consuming
    /// the freeze and returning `Protected` for a gap that never happened.
    /// `freeze_could_not_cover_gap` must also be `false` here: the user was
    /// never actually "away longer than the freeze covers" - a negative gap
    /// isn't a long gap at all, so the notification copy for that case
    /// would be misleading if this were `true`.
    #[test]
    fn test_streak_freeze_not_consumed_on_negative_gap() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 10;
        profile.streak_freeze_available = true;
        // Simulate a backwards clock: last_activity is after `now`.
        profile.last_activity = clock.now() + Duration::days(1);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(
            change,
            StreakChange::Broken {
                was_streak: 10,
                freeze_could_not_cover_gap: false
            }
        );
        assert_eq!(profile.current_streak, 0);
        assert!(
            profile.streak_freeze_available,
            "freeze must not be consumed on a negative (backwards-clock) gap"
        );
    }

    /// Regression test for #319: a streak freeze must not be consumed (nor
    /// `StreakChange::Protected` returned) when `current_streak` is already 0 - there
    /// is nothing to protect, so this should behave like an ordinary (no-op) break.
    #[test]
    fn test_streak_freeze_not_consumed_when_streak_already_zero() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 0;
        profile.streak_freeze_available = true;
        clock.advance_days(2);

        let change = StreakManager::update_streak(&mut profile, clock.now());
        assert_eq!(
            change,
            StreakChange::Broken {
                was_streak: 0,
                freeze_could_not_cover_gap: false
            }
        );
        assert_eq!(profile.current_streak, 0);
        assert!(
            profile.streak_freeze_available,
            "freeze must not be consumed when there was no streak to protect"
        );
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
        profile.current_streak = 10;
        profile.streak_freeze_available = true;

        let result = StreakManager::use_freeze(&mut profile, 2);
        assert!(result.is_ok());
        assert!(!profile.streak_freeze_available);

        // Try again - should fail, no freeze available
        let result = StreakManager::use_freeze(&mut profile, 2);
        assert!(result.is_err());
    }

    /// `use_freeze` must reject an out-of-range gap the same way
    /// `update_streak` does - the freeze is left unconsumed (an `Err` is
    /// returned) rather than silently no-oping or panicking.
    #[test]
    fn test_use_freeze_rejects_gap_beyond_cap() {
        let mut profile = UserProfile::new();
        profile.current_streak = 10;
        profile.streak_freeze_available = true;

        let result = StreakManager::use_freeze(&mut profile, STREAK_FREEZE_MAX_GAP_DAYS + 1);
        assert_matches!(
            result,
            Err(GamificationError::StreakFreezeGapOutOfRange { .. })
        );
        assert!(
            profile.streak_freeze_available,
            "freeze must remain held when the gap exceeds the coverage cap"
        );
    }

    /// Regression test for #346 follow-up (N1): with no active streak, the
    /// gap is irrelevant - a gap that would otherwise be in range must still
    /// be rejected, and with a distinct error from `StreakFreezeGapOutOfRange`
    /// so the failure reason doesn't misname an in-range gap as the cause.
    #[test]
    fn test_use_freeze_rejects_zero_streak_with_distinct_reason() {
        let mut profile = UserProfile::new();
        profile.current_streak = 0;
        profile.streak_freeze_available = true;

        let result = StreakManager::use_freeze(&mut profile, 2);
        assert_matches!(result, Err(GamificationError::StreakFreezeNothingToProtect));
        assert!(
            profile.streak_freeze_available,
            "freeze must remain held when there is no streak to protect"
        );
    }

    /// `use_freeze` and `update_streak` must agree at the exact boundary
    /// `STREAK_FREEZE_MAX_GAP_DAYS` (the cap is inclusive), mirroring
    /// `test_streak_freeze_protects_at_max_gap_boundary` for `update_streak`.
    #[test]
    fn test_use_freeze_accepts_gap_at_max_boundary() {
        let mut profile = UserProfile::new();
        profile.current_streak = 10;
        profile.streak_freeze_available = true;

        let result = StreakManager::use_freeze(&mut profile, STREAK_FREEZE_MAX_GAP_DAYS);
        assert!(result.is_ok());
        assert!(!profile.streak_freeze_available);
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
