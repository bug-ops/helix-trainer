//! Mini-game statistics tracking
//!
//! Tracks score, lives, multiplier, and streaks for the mini-game session.

use serde::{Deserialize, Serialize};

/// Game statistics for mini-game mode
///
/// Tracks all metrics needed for arcade-style gameplay including score,
/// lives, combo multiplier, and streak tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniGameStats {
    /// Current score
    pub score: u64,

    /// Remaining lives (0-5)
    pub lives: u8,

    /// Current score multiplier (1.0 to 5.0)
    pub multiplier: f64,

    /// Current consecutive completions streak
    pub streak: u32,

    /// Current difficulty level (1-10)
    pub level: u32,

    /// Total scenarios completed this session
    pub scenarios_completed: u32,

    /// Total scenarios failed this session
    pub scenarios_failed: u32,

    /// Best streak achieved this session
    pub best_streak: u32,
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
    /// assert_eq!(stats.lives, 3);
    /// assert_eq!(stats.multiplier, 1.0);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Increase score by points with current multiplier applied
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.multiplier = 2.0;
    /// stats.add_score(100);
    /// assert_eq!(stats.score, 200);
    /// ```
    pub fn add_score(&mut self, points: u64) {
        let multiplied = (points as f64 * self.multiplier) as u64;
        self.score = self.score.saturating_add(multiplied);
    }

    /// Increase streak and update multiplier
    ///
    /// Multiplier tiers:
    /// - 0-2: 1.0x
    /// - 3-5: 1.5x
    /// - 6-9: 2.0x
    /// - 10-14: 2.5x
    /// - 15-24: 3.0x
    /// - 25-39: 4.0x
    /// - 40+: 5.0x (max)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.increase_streak();
    /// assert_eq!(stats.streak, 1);
    /// assert_eq!(stats.multiplier, 1.0);
    ///
    /// for _ in 0..5 {
    ///     stats.increase_streak();
    /// }
    /// assert_eq!(stats.streak, 6);
    /// assert_eq!(stats.multiplier, 2.0);
    /// ```
    pub fn increase_streak(&mut self) {
        self.streak = self.streak.saturating_add(1);
        if self.streak > self.best_streak {
            self.best_streak = self.streak;
        }
        self.update_multiplier();
    }

    /// Reset streak and multiplier (on failure/timeout)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.streak = 10;
    /// stats.multiplier = 2.5;
    /// stats.reset_streak();
    /// assert_eq!(stats.streak, 0);
    /// assert_eq!(stats.multiplier, 1.0);
    /// ```
    pub fn reset_streak(&mut self) {
        self.streak = 0;
        self.multiplier = 1.0;
    }

    /// Update multiplier based on current streak
    fn update_multiplier(&mut self) {
        self.multiplier = Self::calculate_multiplier(self.streak);
    }

    /// Calculate multiplier based on streak
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// assert_eq!(MiniGameStats::calculate_multiplier(0), 1.0);
    /// assert_eq!(MiniGameStats::calculate_multiplier(3), 1.5);
    /// assert_eq!(MiniGameStats::calculate_multiplier(10), 2.5);
    /// assert_eq!(MiniGameStats::calculate_multiplier(40), 5.0);
    /// ```
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
    /// assert_eq!(stats.lives, 4);
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
    /// stats.lives = 0;
    /// assert!(stats.is_game_over());
    /// ```
    pub fn is_game_over(&self) -> bool {
        self.lives == 0
    }

    /// Record a scenario completion
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameStats;
    ///
    /// let mut stats = MiniGameStats::new();
    /// stats.record_completion();
    /// assert_eq!(stats.scenarios_completed, 1);
    /// ```
    pub fn record_completion(&mut self) {
        self.scenarios_completed = self.scenarios_completed.saturating_add(1);
    }

    /// Record a scenario failure
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
    /// assert_eq!(stats.level, 2);
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
    /// stats.level = 5;
    /// stats.decrease_level();
    /// assert_eq!(stats.level, 4);
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
            multiplier: 1.0,
            streak: 0,
            level: 1,
            scenarios_completed: 0,
            scenarios_failed: 0,
            best_streak: 0,
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
        assert_eq!(stats.lives, 3);
        assert_eq!(stats.multiplier, 1.0);
        assert_eq!(stats.streak, 0);
        assert_eq!(stats.level, 1);
    }

    #[test]
    fn test_add_score_with_multiplier() {
        let mut stats = MiniGameStats::new();
        stats.add_score(100);
        assert_eq!(stats.score, 100);

        stats.multiplier = 2.0;
        stats.add_score(100);
        assert_eq!(stats.score, 300); // 100 + (100 * 2.0)
    }

    #[test]
    fn test_streak_increases_multiplier() {
        let mut stats = MiniGameStats::new();

        // 0-2: 1.0x
        assert_eq!(stats.multiplier, 1.0);

        // 3-5: 1.5x
        for _ in 0..3 {
            stats.increase_streak();
        }
        assert_eq!(stats.streak, 3);
        assert_eq!(stats.multiplier, 1.5);

        // 6-9: 2.0x
        for _ in 0..3 {
            stats.increase_streak();
        }
        assert_eq!(stats.streak, 6);
        assert_eq!(stats.multiplier, 2.0);

        // 10-14: 2.5x
        for _ in 0..4 {
            stats.increase_streak();
        }
        assert_eq!(stats.streak, 10);
        assert_eq!(stats.multiplier, 2.5);
    }

    #[test]
    fn test_reset_streak() {
        let mut stats = MiniGameStats::new();
        stats.streak = 10;
        stats.multiplier = 2.5;
        stats.best_streak = 10;

        stats.reset_streak();
        assert_eq!(stats.streak, 0);
        assert_eq!(stats.multiplier, 1.0);
        assert_eq!(stats.best_streak, 10); // Best streak preserved
    }

    #[test]
    fn test_lose_life() {
        let mut stats = MiniGameStats::new();
        assert_eq!(stats.lives, 3);

        assert!(stats.lose_life()); // 2 remain
        assert_eq!(stats.lives, 2);

        assert!(stats.lose_life()); // 1 remains
        assert_eq!(stats.lives, 1);

        assert!(!stats.lose_life()); // 0 remain, game over
        assert_eq!(stats.lives, 0);
        assert!(stats.is_game_over());
    }

    #[test]
    fn test_gain_life() {
        let mut stats = MiniGameStats::new();
        stats.lives = 2;

        stats.gain_life();
        assert_eq!(stats.lives, 3);

        stats.gain_life();
        stats.gain_life();
        assert_eq!(stats.lives, 5); // Max 5

        stats.gain_life();
        assert_eq!(stats.lives, 5); // Still 5
    }

    #[test]
    fn test_calculate_multiplier() {
        assert_eq!(MiniGameStats::calculate_multiplier(0), 1.0);
        assert_eq!(MiniGameStats::calculate_multiplier(2), 1.0);
        assert_eq!(MiniGameStats::calculate_multiplier(3), 1.5);
        assert_eq!(MiniGameStats::calculate_multiplier(5), 1.5);
        assert_eq!(MiniGameStats::calculate_multiplier(6), 2.0);
        assert_eq!(MiniGameStats::calculate_multiplier(9), 2.0);
        assert_eq!(MiniGameStats::calculate_multiplier(10), 2.5);
        assert_eq!(MiniGameStats::calculate_multiplier(14), 2.5);
        assert_eq!(MiniGameStats::calculate_multiplier(15), 3.0);
        assert_eq!(MiniGameStats::calculate_multiplier(24), 3.0);
        assert_eq!(MiniGameStats::calculate_multiplier(25), 4.0);
        assert_eq!(MiniGameStats::calculate_multiplier(39), 4.0);
        assert_eq!(MiniGameStats::calculate_multiplier(40), 5.0);
        assert_eq!(MiniGameStats::calculate_multiplier(100), 5.0);
    }

    #[test]
    fn test_record_completion_and_failure() {
        let mut stats = MiniGameStats::new();

        stats.record_completion();
        assert_eq!(stats.scenarios_completed, 1);

        stats.record_failure();
        assert_eq!(stats.scenarios_failed, 1);
    }

    #[test]
    fn test_level_adjustment() {
        let mut stats = MiniGameStats::new();
        assert_eq!(stats.level, 1);

        stats.increase_level();
        assert_eq!(stats.level, 2);

        stats.decrease_level();
        assert_eq!(stats.level, 1);

        stats.decrease_level();
        assert_eq!(stats.level, 1); // Min 1

        for _ in 0..20 {
            stats.increase_level();
        }
        assert_eq!(stats.level, 10); // Max 10
    }

    #[test]
    fn test_best_streak_tracking() {
        let mut stats = MiniGameStats::new();

        for _ in 0..5 {
            stats.increase_streak();
        }
        assert_eq!(stats.best_streak, 5);

        stats.reset_streak();
        assert_eq!(stats.best_streak, 5);

        for _ in 0..3 {
            stats.increase_streak();
        }
        assert_eq!(stats.best_streak, 5); // Still 5

        for _ in 0..3 {
            stats.increase_streak();
        }
        assert_eq!(stats.best_streak, 6); // New record
    }
}
