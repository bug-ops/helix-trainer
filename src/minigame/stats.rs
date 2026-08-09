//! Mini-game statistics tracking
//!
//! Tracks score, lives, multiplier, and streaks for the mini-game session.

use serde::{Deserialize, Serialize};

/// Change event for multiplier animations and notifications
#[derive(Debug, Clone, PartialEq)]
pub enum MultiplierChange {
    /// Multiplier increased
    Increased { from: f64, to: f64 },
    /// Multiplier decreased (after grace consumed)
    Decreased { from: f64, to: f64 },
    /// Reached a milestone streak (10, 25, 50)
    MilestoneReached { multiplier: f64 },
    /// Grace period used instead of reset
    GraceUsed,
}

/// Encapsulated multiplier state with grace mechanics
///
/// Manages streak-based multiplier with grace periods at milestones.
/// Grace allows one failure without losing the multiplier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiplierState {
    /// Current multiplier value (1.0 to 5.0)
    current: f64,
    /// Current streak count
    streak: u32,
    /// Best streak achieved
    best_streak: u32,
    /// Grace failures remaining (0-1)
    grace_remaining: u8,
    /// Recent change event for UI animation (not serialized)
    #[serde(skip)]
    recent_change: Option<MultiplierChange>,
}

impl MultiplierState {
    /// Create new multiplier state
    pub fn new() -> Self {
        Self {
            current: 1.0,
            streak: 0,
            best_streak: 0,
            grace_remaining: 0,
            recent_change: None,
        }
    }

    /// Get current multiplier value
    pub fn current(&self) -> f64 {
        self.current
    }

    /// Get current streak
    pub fn streak(&self) -> u32 {
        self.streak
    }

    /// Get best streak achieved
    pub fn best_streak(&self) -> u32 {
        self.best_streak
    }

    /// Get remaining grace count
    pub fn grace_remaining(&self) -> u8 {
        self.grace_remaining
    }

    /// Record a successful scenario completion
    ///
    /// Increases streak, updates multiplier, and grants grace at milestones.
    pub fn record_success(&mut self) {
        let old_multiplier = self.current;

        self.streak = self.streak.saturating_add(1);
        if self.streak > self.best_streak {
            self.best_streak = self.streak;
        }

        self.current = Self::calculate_multiplier(self.streak);

        // Grant grace at milestones (10, 25, 50)
        if matches!(self.streak, 10 | 25 | 50) {
            self.grace_remaining = 1;
            self.recent_change = Some(MultiplierChange::MilestoneReached {
                multiplier: self.current,
            });
        } else if (self.current - old_multiplier).abs() > f64::EPSILON {
            self.recent_change = Some(MultiplierChange::Increased {
                from: old_multiplier,
                to: self.current,
            });
        } else {
            self.recent_change = None;
        }
    }

    /// Record a scenario failure
    ///
    /// Uses grace if available, otherwise resets streak and multiplier.
    pub fn record_failure(&mut self) {
        if self.grace_remaining > 0 {
            self.grace_remaining -= 1;
            self.recent_change = Some(MultiplierChange::GraceUsed);
        } else {
            let old_multiplier = self.current;
            self.streak = 0;
            self.current = 1.0;

            if (old_multiplier - 1.0).abs() > f64::EPSILON {
                self.recent_change = Some(MultiplierChange::Decreased {
                    from: old_multiplier,
                    to: 1.0,
                });
            } else {
                self.recent_change = None;
            }
        }
    }

    /// Take the recent change event (consumes it)
    ///
    /// Returns the change event if one occurred since last call.
    pub fn take_change(&mut self) -> Option<MultiplierChange> {
        self.recent_change.take()
    }

    /// Get streak count needed for next multiplier tier
    ///
    /// Returns None if already at maximum tier.
    pub fn streak_for_next_tier(&self) -> Option<u32> {
        match self.streak {
            0..=2 => Some(3),
            3..=5 => Some(6),
            6..=9 => Some(10),
            10..=14 => Some(15),
            15..=24 => Some(25),
            25..=39 => Some(40),
            _ => None, // Already at max
        }
    }

    /// Calculate multiplier based on streak
    pub fn calculate_multiplier(streak: u32) -> f64 {
        match streak {
            0..=2 => 1.0,
            3..=5 => 1.5,
            6..=9 => 2.0,
            10..=14 => 2.5,
            15..=24 => 3.0,
            25..=39 => 4.0,
            _ => 5.0, // 40+ is max
        }
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.current = 1.0;
        self.streak = 0;
        self.grace_remaining = 0;
        self.recent_change = None;
    }
}

impl Default for MultiplierState {
    fn default() -> Self {
        Self::new()
    }
}

/// Game statistics for mini-game mode
///
/// Tracks all metrics needed for arcade-style gameplay including score,
/// lives, combo multiplier, and streak tracking. The multiplier/streak
/// tier table lives solely in [`MultiplierState`], embedded here as the
/// single source of truth — read it through the `multiplier()`, `streak()`,
/// `best_streak()`, and `grace_remaining()` accessors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniGameStats {
    /// Current score
    pub score: u64,

    /// Remaining lives (0-5)
    lives: u8,

    /// Current difficulty level (1-10)
    level: u32,

    /// Total scenarios completed this session
    pub scenarios_completed: u32,

    /// Total scenarios failed this session
    pub scenarios_failed: u32,

    /// Multiplier/streak tier table (sole owner of streak state)
    multiplier_state: MultiplierState,
}

impl MiniGameStats {
    /// Create new stats with default starting values
    ///
    /// Starting state:
    /// - 3 lives
    /// - 1.0x multiplier
    /// - 0 score/streak
    /// - Level 1
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let stats = MiniGameStats::new();
    /// assert_eq!(stats.lives(), 3);
    /// assert_eq!(stats.multiplier(), 1.0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Create new stats with a specific number of starting lives
    ///
    /// Used by different game modes to configure starting lives:
    /// - Arcade: 3 lives
    /// - Survival: 1 life
    /// - Challenge: 3 lives
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let stats = MiniGameStats::new_with_lives(1);
    /// assert_eq!(stats.lives(), 1);
    /// assert_eq!(stats.multiplier(), 1.0);
    /// ```
    pub fn new_with_lives(lives: u8) -> Self {
        Self {
            lives,
            ..Self::default()
        }
    }

    /// Get remaining lives
    pub fn lives(&self) -> u8 {
        self.lives
    }

    /// Get current difficulty level
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Get current score multiplier
    pub fn multiplier(&self) -> f64 {
        self.multiplier_state.current()
    }

    /// Get current consecutive completions streak
    pub fn streak(&self) -> u32 {
        self.multiplier_state.streak()
    }

    /// Get best streak achieved this session
    pub fn best_streak(&self) -> u32 {
        self.multiplier_state.best_streak()
    }

    /// Get remaining grace failures (protects the multiplier from resetting)
    pub fn grace_remaining(&self) -> u8 {
        self.multiplier_state.grace_remaining()
    }

    /// Take the recent multiplier change event (consumes it)
    ///
    /// Returns the change event if one occurred since last call.
    pub fn take_multiplier_change(&mut self) -> Option<MultiplierChange> {
        self.multiplier_state.take_change()
    }

    /// Increase score by points with current multiplier applied
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// for _ in 0..6 {
    ///     stats.record_completion();
    /// }
    /// stats.add_score(100);
    /// assert_eq!(stats.score, 200);
    /// ```
    pub fn add_score(&mut self, points: u64) {
        let multiplied = (points as f64 * self.multiplier_state.current()) as u64;
        self.score = self.score.saturating_add(multiplied);
    }

    /// Lose a life
    ///
    /// Returns true if lives remain, false if game over.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// assert!(stats.lose_life());  // 2 lives remain
    /// assert!(stats.lose_life());  // 1 life remains
    /// assert!(!stats.lose_life()); // 0 lives, game over
    /// ```
    pub fn lose_life(&mut self) -> bool {
        if self.lives > 0 {
            self.lives -= 1;
        }
        self.lives > 0
    }

    /// Gain a life (from score milestones)
    ///
    /// Maximum 5 lives.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.gain_life();
    /// assert_eq!(stats.lives(), 4);
    /// ```
    pub fn gain_life(&mut self) {
        if self.lives < 5 {
            self.lives += 1;
        }
    }

    /// Check if game is over (no lives remaining)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// assert!(!stats.is_game_over());
    /// while stats.lose_life() {}
    /// assert!(stats.is_game_over());
    /// ```
    pub fn is_game_over(&self) -> bool {
        self.lives == 0
    }

    /// Record a scenario completion
    ///
    /// Advances the multiplier/streak tier table (see [`MultiplierState::record_success`])
    /// and increments the completion counter.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.record_completion();
    /// assert_eq!(stats.scenarios_completed, 1);
    /// assert_eq!(stats.streak(), 1);
    /// ```
    pub fn record_completion(&mut self) {
        self.multiplier_state.record_success();
        self.scenarios_completed = self.scenarios_completed.saturating_add(1);
    }

    /// Record a scenario failure
    ///
    /// Consumes grace if available, otherwise resets the streak/multiplier
    /// (see [`MultiplierState::record_failure`]), and increments the failure counter.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.record_failure();
    /// assert_eq!(stats.scenarios_failed, 1);
    /// ```
    pub fn record_failure(&mut self) {
        self.multiplier_state.record_failure();
        self.scenarios_failed = self.scenarios_failed.saturating_add(1);
    }

    /// Increase difficulty level
    ///
    /// Maximum level is 10.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.increase_level();
    /// assert_eq!(stats.level(), 2);
    /// ```
    pub fn increase_level(&mut self) {
        if self.level < 10 {
            self.level += 1;
        }
    }

    /// Decrease difficulty level
    ///
    /// Minimum level is 1.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.increase_level();
    /// stats.increase_level();
    /// stats.increase_level();
    /// stats.increase_level();
    /// stats.decrease_level();
    /// assert_eq!(stats.level(), 4);
    /// ```
    pub fn decrease_level(&mut self) {
        if self.level > 1 {
            self.level -= 1;
        }
    }
}

impl Default for MiniGameStats {
    fn default() -> Self {
        Self {
            score: 0,
            lives: 3,
            level: 1,
            scenarios_completed: 0,
            scenarios_failed: 0,
            multiplier_state: MultiplierState::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stats() {
        let stats = MiniGameStats::new();
        assert_eq!(stats.score, 0);
        assert_eq!(stats.lives(), 3);
        assert_eq!(stats.multiplier(), 1.0);
        assert_eq!(stats.streak(), 0);
        assert_eq!(stats.level(), 1);
    }

    #[test]
    fn test_add_score_with_multiplier() {
        let mut stats = MiniGameStats::new();
        stats.add_score(100);
        assert_eq!(stats.score, 100);

        for _ in 0..6 {
            stats.record_completion();
        }
        assert_eq!(stats.multiplier(), 2.0);
        stats.add_score(100);
        assert_eq!(stats.score, 300); // 100 + (100 * 2.0)
    }

    #[test]
    fn test_lose_life() {
        let mut stats = MiniGameStats::new();
        assert_eq!(stats.lives(), 3);

        assert!(stats.lose_life()); // 2 remain
        assert_eq!(stats.lives(), 2);

        assert!(stats.lose_life()); // 1 remains
        assert_eq!(stats.lives(), 1);

        assert!(!stats.lose_life()); // 0 remain, game over
        assert_eq!(stats.lives(), 0);
        assert!(stats.is_game_over());
    }

    #[test]
    fn test_gain_life() {
        let mut stats = MiniGameStats::new();
        assert!(stats.lose_life()); // 3 -> 2

        stats.gain_life();
        assert_eq!(stats.lives(), 3);

        stats.gain_life();
        stats.gain_life();
        assert_eq!(stats.lives(), 5); // Max 5

        stats.gain_life();
        assert_eq!(stats.lives(), 5); // Still 5
    }

    #[test]
    fn test_record_completion_and_failure() {
        let mut stats = MiniGameStats::new();

        stats.record_completion();
        assert_eq!(stats.scenarios_completed, 1);
        assert_eq!(stats.streak(), 1);

        stats.record_failure();
        assert_eq!(stats.scenarios_failed, 1);
        assert_eq!(stats.streak(), 0);
    }

    #[test]
    fn test_level_adjustment() {
        let mut stats = MiniGameStats::new();
        assert_eq!(stats.level(), 1);

        stats.increase_level();
        assert_eq!(stats.level(), 2);

        stats.decrease_level();
        assert_eq!(stats.level(), 1);

        stats.decrease_level();
        assert_eq!(stats.level(), 1); // Min 1

        for _ in 0..20 {
            stats.increase_level();
        }
        assert_eq!(stats.level(), 10); // Max 10
    }

    #[test]
    fn test_new_with_lives() {
        let stats = MiniGameStats::new_with_lives(1);
        assert_eq!(stats.lives(), 1);
        assert_eq!(stats.score, 0);
        assert_eq!(stats.multiplier(), 1.0);
        assert_eq!(stats.level(), 1);

        let stats = MiniGameStats::new_with_lives(5);
        assert_eq!(stats.lives(), 5);
    }

    // CR-009: Test new_with_lives boundary values
    #[test]
    fn test_new_with_lives_boundary_values() {
        // Zero lives - immediately game over
        let stats_zero = MiniGameStats::new_with_lives(0);
        assert_eq!(stats_zero.lives(), 0);
        assert!(stats_zero.is_game_over());

        // Default arcade lives
        let stats_three = MiniGameStats::new_with_lives(3);
        assert_eq!(stats_three.lives(), 3);
        assert!(!stats_three.is_game_over());

        // Maximum u8 value
        let stats_max = MiniGameStats::new_with_lives(u8::MAX);
        assert_eq!(stats_max.lives(), u8::MAX);
        assert!(!stats_max.is_game_over());
    }

    mod multiplier_state_tests {
        use super::*;

        #[test]
        fn test_new_state() {
            let state = MultiplierState::new();
            assert!((state.current() - 1.0).abs() < f64::EPSILON);
            assert_eq!(state.streak(), 0);
            assert_eq!(state.best_streak(), 0);
            assert_eq!(state.grace_remaining(), 0);
        }

        #[test]
        fn test_record_success_increases_streak() {
            let mut state = MultiplierState::new();
            state.record_success();
            assert_eq!(state.streak(), 1);
            state.record_success();
            assert_eq!(state.streak(), 2);
        }

        #[test]
        fn test_record_success_updates_multiplier() {
            let mut state = MultiplierState::new();

            for _ in 0..3 {
                state.record_success();
            }
            assert!((state.current() - 1.5).abs() < f64::EPSILON);

            for _ in 0..3 {
                state.record_success();
            }
            assert!((state.current() - 2.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_record_success_updates_best_streak() {
            let mut state = MultiplierState::new();
            for _ in 0..5 {
                state.record_success();
            }
            assert_eq!(state.best_streak(), 5);

            state.record_failure();
            assert_eq!(state.best_streak(), 5);

            for _ in 0..3 {
                state.record_success();
            }
            assert_eq!(state.best_streak(), 5); // Shorter rebuild does not overwrite

            for _ in 0..3 {
                state.record_success();
            }
            assert_eq!(state.best_streak(), 6);
        }

        #[test]
        fn test_milestone_grants_grace() {
            let mut state = MultiplierState::new();
            for _ in 0..10 {
                state.record_success();
            }
            assert_eq!(state.grace_remaining(), 1);
            assert!(matches!(
                state.take_change(),
                Some(MultiplierChange::MilestoneReached { .. })
            ));
        }

        #[test]
        fn test_grace_protects_from_reset() {
            let mut state = MultiplierState::new();
            for _ in 0..10 {
                state.record_success();
            }
            assert_eq!(state.grace_remaining(), 1);
            let multiplier_before = state.current();

            state.record_failure();
            assert_eq!(state.grace_remaining(), 0);
            assert!((state.current() - multiplier_before).abs() < f64::EPSILON);
            assert!(matches!(
                state.take_change(),
                Some(MultiplierChange::GraceUsed)
            ));
        }

        #[test]
        fn test_failure_without_grace_resets() {
            let mut state = MultiplierState::new();
            for _ in 0..5 {
                state.record_success();
            }
            let old_mult = state.current();
            assert!(old_mult > 1.0);

            state.record_failure();
            assert!((state.current() - 1.0).abs() < f64::EPSILON);
            assert_eq!(state.streak(), 0);
            assert!(matches!(
                state.take_change(),
                Some(MultiplierChange::Decreased { from, to }) if (from - old_mult).abs() < f64::EPSILON && (to - 1.0).abs() < f64::EPSILON
            ));
        }

        #[test]
        fn test_take_change_consumes() {
            let mut state = MultiplierState::new();
            for _ in 0..3 {
                state.record_success();
            }
            assert!(state.take_change().is_some());
            assert!(state.take_change().is_none());
        }

        #[test]
        fn test_streak_for_next_tier() {
            let mut state = MultiplierState::new();
            assert_eq!(state.streak_for_next_tier(), Some(3));

            for _ in 0..3 {
                state.record_success();
            }
            assert_eq!(state.streak_for_next_tier(), Some(6));

            for _ in 0..3 {
                state.record_success();
            }
            assert_eq!(state.streak_for_next_tier(), Some(10));

            for _ in 0..34 {
                state.record_success();
            }
            assert_eq!(state.streak_for_next_tier(), None);
        }

        #[test]
        fn test_multiplier_tiers() {
            assert!((MultiplierState::calculate_multiplier(0) - 1.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(2) - 1.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(3) - 1.5).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(5) - 1.5).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(6) - 2.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(9) - 2.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(10) - 2.5).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(14) - 2.5).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(15) - 3.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(24) - 3.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(25) - 4.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(39) - 4.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(40) - 5.0).abs() < f64::EPSILON);
            assert!((MultiplierState::calculate_multiplier(100) - 5.0).abs() < f64::EPSILON);
        }

        #[test]
        fn test_milestone_25_grants_grace() {
            let mut state = MultiplierState::new();
            for _ in 0..25 {
                state.record_success();
            }
            assert_eq!(state.grace_remaining(), 1);
        }

        #[test]
        fn test_milestone_50_grants_grace() {
            let mut state = MultiplierState::new();
            for _ in 0..50 {
                state.record_success();
            }
            assert_eq!(state.grace_remaining(), 1);
        }

        #[test]
        fn test_reset() {
            let mut state = MultiplierState::new();
            for _ in 0..15 {
                state.record_success();
            }

            state.reset();
            assert!((state.current() - 1.0).abs() < f64::EPSILON);
            assert_eq!(state.streak(), 0);
            assert_eq!(state.grace_remaining(), 0);
        }

        #[test]
        fn test_change_on_multiplier_increase() {
            let mut state = MultiplierState::new();
            state.record_success();
            state.record_success();
            state.take_change();

            state.record_success();
            assert!(matches!(
                state.take_change(),
                Some(MultiplierChange::Increased { from, to }) if (from - 1.0).abs() < f64::EPSILON && (to - 1.5).abs() < f64::EPSILON
            ));
        }

        #[test]
        fn test_no_change_when_multiplier_same() {
            let mut state = MultiplierState::new();
            state.record_success();
            state.take_change();
            state.record_success();
            assert!(state.take_change().is_none());
        }
    }
}
