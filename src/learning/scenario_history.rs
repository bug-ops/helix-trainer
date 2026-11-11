//! Scenario completion tracking and mastery system
//!
//! Prevents XP farming by tracking scenario mastery and scaling rewards.
//! Implements a three-tier mastery system: Learning → Proficient → Mastered

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::constants::{
    MASTERY_FACTOR_LEARNING, MASTERY_FACTOR_MASTERED, MASTERY_FACTOR_PROFICIENT,
    REPEAT_PENALTY_HEAVY, REPEAT_PENALTY_LIGHT, REPEAT_PENALTY_NONE,
};
use crate::ids::ScenarioId;
use crate::numeric::{Rounding, scale_u64};

/// Maximum recursion depth for repeat command protection
const MAX_REPEAT_DEPTH: u32 = 100;

/// Maximum number of scenarios to track in history (prevents unbounded growth)
/// Current application has ~20 scenarios, this allows room for 500x growth
const MAX_SCENARIOS_TRACKED: usize = 10_000;

/// Scales base XP by a mastery and repeat multiplier, rounding to the nearest whole XP.
pub(crate) fn scale_xp(base_xp: u64, multiplier: f64) -> u64 {
    scale_u64(base_xp, multiplier, Rounding::Nearest)
}

/// Mastery level for a scenario based on performance history
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioMastery {
    /// Still learning (< 3 attempts OR best score < 90)
    Learning,
    /// Proficient (3+ attempts AND best score >= 90)
    Proficient,
    /// Mastered (2+ perfect completions)
    Mastered,
}

impl ScenarioMastery {
    /// Get display name (alias for trait method)
    pub fn display_name(&self) -> &'static str {
        use super::traits::ProgressionTier;
        self.name()
    }

    /// Get emoji representation for UI display (convenience method)
    pub fn emoji(&self) -> &'static str {
        use super::traits::ProgressionTier;
        ProgressionTier::emoji(self)
    }

    /// Get XP reduction description (convenience method)
    pub fn xp_description(&self) -> &'static str {
        use super::traits::ProgressionTier;
        ProgressionTier::xp_description(self)
    }
}

impl super::traits::Modifier for ScenarioMastery {
    fn factor(&self) -> f64 {
        match self {
            ScenarioMastery::Learning => MASTERY_FACTOR_LEARNING,
            ScenarioMastery::Proficient => MASTERY_FACTOR_PROFICIENT,
            ScenarioMastery::Mastered => MASTERY_FACTOR_MASTERED,
        }
    }
}

impl super::traits::ProgressionTier for ScenarioMastery {
    fn name(&self) -> &'static str {
        match self {
            ScenarioMastery::Learning => "Learning",
            ScenarioMastery::Proficient => "Proficient",
            ScenarioMastery::Mastered => "Mastered",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            ScenarioMastery::Learning => "🌱",
            ScenarioMastery::Proficient => "⭐",
            ScenarioMastery::Mastered => "🏆",
        }
    }

    fn tier_level(&self) -> u32 {
        match self {
            ScenarioMastery::Learning => 0,
            ScenarioMastery::Proficient => 1,
            ScenarioMastery::Mastered => 2,
        }
    }

    fn is_max_tier(&self) -> bool {
        matches!(self, ScenarioMastery::Mastered)
    }

    fn xp_description(&self) -> &'static str {
        match self {
            ScenarioMastery::Learning => "",
            ScenarioMastery::Proficient => "(-50% XP)",
            ScenarioMastery::Mastered => "(-80% XP)",
        }
    }
}

/// Historical data for a single scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioCompletion {
    /// Scenario this record belongs to.
    pub scenario_id: ScenarioId,

    // Performance tracking
    /// Total attempts.
    pub attempts: u32,
    /// Highest score reached.
    pub best_score: u32,
    /// Number of 100% completions.
    pub perfect_count: u32,
    /// XP awarded across all attempts.
    pub total_xp_earned: u64,

    // Timestamps
    /// When the scenario was first attempted.
    pub first_attempt: DateTime<Utc>,
    /// When the scenario was last attempted.
    pub last_attempt: DateTime<Utc>,

    // Derived state
    /// Mastery tier derived from the completion history.
    pub mastery_level: ScenarioMastery,

    // Today's session tracking (reset at midnight)
    /// Attempts made on `last_attempt_date`; resets on a new day.
    pub attempts_today: u32,
    /// Date of the last attempt, used to detect the day change.
    pub last_attempt_date: NaiveDate,
}

impl ScenarioCompletion {
    /// Create new completion record for first attempt
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioCompletion;
    ///
    /// let id = ScenarioId::parse("delete_line_001")?;
    /// let completion = ScenarioCompletion::new(id, 80, 40, Utc::now());
    /// assert_eq!(completion.attempts, 1);
    /// assert_eq!(completion.best_score, 80);
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn new(scenario_id: ScenarioId, score: u32, xp_earned: u64, now: DateTime<Utc>) -> Self {
        let mut completion = Self {
            scenario_id,
            attempts: 0,
            best_score: 0,
            perfect_count: 0,
            total_xp_earned: 0,
            first_attempt: now,
            last_attempt: now,
            mastery_level: ScenarioMastery::Learning,
            attempts_today: 0,
            last_attempt_date: now.date_naive(),
        };

        // Record the first attempt
        completion.record_attempt(score, xp_earned, now);
        completion
    }

    /// Record a new attempt and update mastery
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioCompletion;
    ///
    /// let mut completion = ScenarioCompletion::new(ScenarioId::parse("test")?, 80, 40, Utc::now());
    /// completion.record_attempt(100, 50, Utc::now());
    /// assert_eq!(completion.attempts, 2);
    /// assert_eq!(completion.best_score, 100);
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn record_attempt(&mut self, score: u32, xp_earned: u64, now: DateTime<Utc>) {
        // Prevent infinite recursion with depth protection
        if self.attempts >= MAX_REPEAT_DEPTH {
            return;
        }

        // Single `now` read shared with check_and_reset_daily (previously two independent
        // Utc::now() calls, which could straddle a midnight boundary); this also fixes a
        // latent race where the daily counter reset used a different instant than the attempt.
        self.check_and_reset_daily(now);

        // Update counters using saturating arithmetic
        self.attempts = self.attempts.saturating_add(1);
        self.attempts_today = self.attempts_today.saturating_add(1);
        self.best_score = self.best_score.max(score);
        if score == 100 {
            self.perfect_count = self.perfect_count.saturating_add(1);
        }
        self.total_xp_earned = self.total_xp_earned.saturating_add(xp_earned);
        self.last_attempt = now;

        // Update mastery level
        self.update_mastery();
    }

    /// Calculate XP multiplier based on mastery and session count
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioCompletion;
    ///
    /// let completion = ScenarioCompletion::new(ScenarioId::parse("test")?, 50, 25, Utc::now());
    /// assert_eq!(completion.xp_multiplier(), 0.7); // Second attempt today (after construction)
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn xp_multiplier(&self) -> f64 {
        self.mastery_factor() * self.repeat_penalty()
    }

    /// Get XP multiplier from mastery level only
    ///
    /// Returns factor based on mastery: Learning = 1.0, Proficient = 0.5, Mastered = 0.2
    pub fn mastery_factor(&self) -> f64 {
        use super::traits::Modifier;
        self.mastery_level.factor()
    }

    /// Get XP multiplier from session repeat penalty only
    ///
    /// Returns factor based on attempts today: 1st = 1.0, 2nd-3rd = 0.7, 4th+ = 0.3
    pub fn repeat_penalty(&self) -> f64 {
        match self.attempts_today {
            0 => REPEAT_PENALTY_NONE,
            1 | 2 => REPEAT_PENALTY_LIGHT,
            _ => REPEAT_PENALTY_HEAVY,
        }
    }

    /// Update mastery level based on current stats
    fn update_mastery(&mut self) {
        self.mastery_level = if self.perfect_count >= 2 {
            ScenarioMastery::Mastered
        } else if self.attempts >= 3 && self.best_score >= 90 {
            ScenarioMastery::Proficient
        } else {
            ScenarioMastery::Learning
        };
    }

    /// Reset daily attempt counter if new day
    fn check_and_reset_daily(&mut self, now: DateTime<Utc>) {
        let today = now.date_naive();
        if today != self.last_attempt_date {
            self.attempts_today = 0;
            self.last_attempt_date = today;
        }
    }
}

/// Manager for all scenario completion history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioHistory {
    completions: HashMap<ScenarioId, ScenarioCompletion>,
}

impl ScenarioHistory {
    /// Create new history tracker
    pub fn new() -> Self {
        Self {
            completions: HashMap::new(),
        }
    }

    /// Get completion record for scenario (None if never attempted)
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let history = ScenarioHistory::new();
    /// assert!(history.get(&ScenarioId::parse("test")?).is_none());
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn get(&self, scenario_id: &ScenarioId) -> Option<&ScenarioCompletion> {
        self.completions.get(scenario_id)
    }

    /// Iterate over every recorded scenario completion.
    ///
    /// Used to compute cross-scenario aggregates (e.g. the overall attempt
    /// time span) that can't be derived from a single [`Self::get`] lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let mut history = ScenarioHistory::new();
    /// history.record_completion(&ScenarioId::parse("a")?, 100, 50, Utc::now());
    /// history.record_completion(&ScenarioId::parse("b")?, 80, 40, Utc::now());
    /// assert_eq!(history.iter().count(), 2);
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &ScenarioCompletion> {
        self.completions.values()
    }

    /// Record a scenario completion with score and base XP
    ///
    /// Returns actual XP awarded after mastery scaling
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let mut history = ScenarioHistory::new();
    /// let xp = history.record_completion(&ScenarioId::parse("test")?, 100, 50, Utc::now());
    /// assert_eq!(xp, 50); // First attempt, full XP
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn record_completion(
        &mut self,
        scenario_id: &ScenarioId,
        score: u32,
        base_xp: u64,
        now: DateTime<Utc>,
    ) -> u64 {
        // Check if we're at the limit and this is a new scenario
        if !self.completions.contains_key(scenario_id)
            && self.completions.len() >= MAX_SCENARIOS_TRACKED
        {
            // At capacity - silently ignore new scenarios (defense in depth)
            // In practice, this should never happen with ~20 scenarios
            return 0;
        }

        // Calculate multiplier BEFORE recording (to get current state)
        let multiplier = self
            .completions
            .get(scenario_id)
            .map_or(1.0, ScenarioCompletion::xp_multiplier); // First attempt = full XP

        let actual_xp = scale_xp(base_xp, multiplier);

        // Record completion (creates new or updates existing)
        self.completions
            .entry(scenario_id.clone())
            .and_modify(|c| c.record_attempt(score, actual_xp, now))
            .or_insert_with(|| ScenarioCompletion::new(scenario_id.clone(), score, actual_xp, now));

        actual_xp
    }

    /// Get mastery statistics (count per level)
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::ids::ScenarioId;
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let mut history = ScenarioHistory::new();
    /// history.record_completion(&ScenarioId::parse("test1")?, 100, 50, Utc::now());
    /// history.record_completion(&ScenarioId::parse("test2")?, 80, 40, Utc::now());
    ///
    /// let stats = history.mastery_stats();
    /// assert_eq!(stats.learning, 2);
    /// assert_eq!(stats.proficient, 0);
    /// assert_eq!(stats.mastered, 0);
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    pub fn mastery_stats(&self) -> MasteryStats {
        let mut stats = MasteryStats::default();

        for completion in self.completions.values() {
            match completion.mastery_level {
                ScenarioMastery::Learning => stats.learning = stats.learning.saturating_add(1),
                ScenarioMastery::Proficient => {
                    stats.proficient = stats.proficient.saturating_add(1);
                }
                ScenarioMastery::Mastered => stats.mastered = stats.mastered.saturating_add(1),
            }
        }

        stats
    }
}

/// Statistics about scenario mastery levels
#[derive(Debug, Clone, Default)]
pub struct MasteryStats {
    /// Scenarios still being learned.
    pub learning: u32,
    /// Scenarios completed often enough to earn reduced XP.
    pub proficient: u32,
    /// Scenarios mastered, with the strongest XP reduction.
    pub mastered: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::sid;

    #[test]
    fn test_new_completion_initializes_correctly() {
        let completion = ScenarioCompletion::new(sid("test_scenario"), 85, 42, Utc::now());

        assert_eq!(completion.scenario_id.as_str(), "test_scenario");
        assert_eq!(completion.attempts, 1);
        assert_eq!(completion.best_score, 85);
        assert_eq!(completion.perfect_count, 0);
        assert_eq!(completion.total_xp_earned, 42);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.attempts_today, 1);
    }

    #[test]
    fn test_mastery_progression_to_proficient() {
        let mut completion = ScenarioCompletion::new(sid("test"), 85, 40, Utc::now());

        // Need 3+ attempts AND 90+ score for proficient
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);

        completion.record_attempt(92, 45, Utc::now());
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning); // Only 2 attempts

        completion.record_attempt(95, 47, Utc::now());
        assert_eq!(completion.mastery_level, ScenarioMastery::Proficient); // 3 attempts + 95 score
        assert_eq!(completion.attempts, 3);
        assert_eq!(completion.best_score, 95);
    }

    #[test]
    fn test_mastery_progression_to_mastered() {
        let mut completion = ScenarioCompletion::new(sid("test"), 100, 50, Utc::now());

        assert_eq!(completion.perfect_count, 1);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning); // Need 2 perfect

        completion.record_attempt(100, 50, Utc::now());
        assert_eq!(completion.perfect_count, 2);
        assert_eq!(completion.mastery_level, ScenarioMastery::Mastered);
    }

    #[test]
    fn test_xp_multiplier_learning_phase() {
        let completion = ScenarioCompletion::new(sid("test"), 80, 40, Utc::now());
        // After new(), attempts_today = 1, so next call gets session penalty
        assert_eq!(completion.xp_multiplier(), 0.7); // Second attempt today (1.0 * 0.7)
    }

    #[test]
    fn test_xp_multiplier_session_penalty() {
        // Use scores < 90 to avoid triggering proficiency (needs 90+ AND 3+ attempts)
        let mut completion = ScenarioCompletion::new(sid("test"), 80, 40, Utc::now());

        // First attempt already recorded in new(), attempts_today = 1
        assert_eq!(completion.attempts_today, 1);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.xp_multiplier(), 0.7); // Second today (1.0 * 0.7)

        completion.record_attempt(85, 35, Utc::now());
        assert_eq!(completion.attempts_today, 2);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.xp_multiplier(), 0.7); // Third today (1.0 * 0.7)

        completion.record_attempt(88, 30, Utc::now()); // Keep < 90 to stay in learning
        assert_eq!(completion.attempts_today, 3);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.xp_multiplier(), 0.3); // Fourth today (1.0 * 0.3)
    }

    #[test]
    fn test_xp_multiplier_mastered() {
        let mut completion = ScenarioCompletion::new(sid("test"), 100, 50, Utc::now());
        completion.record_attempt(100, 10, Utc::now()); // Now mastered (2 perfect)

        assert_eq!(completion.mastery_level, ScenarioMastery::Mastered);
        // attempts_today = 2, so session_mult = 0.7
        // mastery_mult = 0.2
        // total = 0.2 * 0.7 = 0.14
        let multiplier = completion.xp_multiplier();
        assert!((multiplier - 0.14).abs() < 0.001); // Allow small floating point error
    }

    #[test]
    fn test_scenario_history_first_completion() {
        let mut history = ScenarioHistory::new();

        let xp = history.record_completion(&sid("test"), 100, 50, Utc::now());
        assert_eq!(xp, 50); // First attempt, full XP

        let completion = history.get(&sid("test")).unwrap();
        assert_eq!(completion.attempts, 1);
        assert_eq!(completion.best_score, 100);
        assert_eq!(completion.total_xp_earned, 50);
    }

    #[test]
    fn test_scenario_history_repeated_completions() {
        let mut history = ScenarioHistory::new();

        // First attempt (creates new completion with attempts_today=1)
        let xp1 = history.record_completion(&sid("test"), 100, 50, Utc::now());
        assert_eq!(xp1, 50); // Full XP (first attempt gets 1.0 multiplier)

        // Second attempt (attempts_today=1 before call, session_mult=0.7, still learning)
        let xp2 = history.record_completion(&sid("test"), 100, 50, Utc::now());
        assert_eq!(xp2, 35); // 50 * 0.7 = 35 (session penalty)

        // Third attempt
        // After 2nd completion: perfect_count=2 (Mastered!), attempts_today=2
        // multiplier = mastery(0.2) * session(2->0.7) = 0.14
        // XP = 50 * 0.14 = 7.0 (rounded)
        let xp3 = history.record_completion(&sid("test"), 100, 50, Utc::now());
        assert_eq!(xp3, 7);
    }

    #[test]
    fn test_scenario_history_mastery_stats() {
        let mut history = ScenarioHistory::new();

        // Create scenarios at different mastery levels
        history.record_completion(&sid("learning1"), 80, 40, Utc::now());
        history.record_completion(&sid("learning2"), 50, 25, Utc::now());

        history.record_completion(&sid("proficient"), 90, 45, Utc::now());
        history.record_completion(&sid("proficient"), 92, 23, Utc::now()); // 2nd attempt
        history.record_completion(&sid("proficient"), 95, 24, Utc::now()); // 3rd attempt -> proficient

        history.record_completion(&sid("mastered"), 100, 50, Utc::now());
        history.record_completion(&sid("mastered"), 100, 10, Utc::now()); // 2nd perfect -> mastered

        let stats = history.mastery_stats();
        assert_eq!(stats.learning, 2);
        assert_eq!(stats.proficient, 1);
        assert_eq!(stats.mastered, 1);
    }

    #[test]
    fn test_saturating_arithmetic_prevents_overflow() {
        let mut completion = ScenarioCompletion::new(sid("test"), 100, u64::MAX - 100, Utc::now());

        // Should not panic on overflow (note: new() already added some XP)
        completion.record_attempt(80, 1000, Utc::now()); // Use non-perfect to avoid mastery
        assert_eq!(completion.total_xp_earned, u64::MAX);

        // Test attempts saturation (but MAX_REPEAT_DEPTH = 100 prevents this)
        // Once attempts >= 100, record_attempt returns early
        completion.attempts = 50;
        completion.record_attempt(80, 100, Utc::now());
        assert_eq!(completion.attempts, 51); // Increments normally

        completion.attempts = MAX_REPEAT_DEPTH - 1;
        completion.record_attempt(80, 100, Utc::now());
        assert_eq!(completion.attempts, MAX_REPEAT_DEPTH); // Reaches limit

        completion.record_attempt(80, 100, Utc::now());
        assert_eq!(completion.attempts, MAX_REPEAT_DEPTH); // Blocked by MAX_REPEAT_DEPTH
    }

    #[test]
    fn test_max_repeat_depth_protection() {
        let mut completion = ScenarioCompletion::new(sid("test"), 100, 50, Utc::now());
        completion.attempts = MAX_REPEAT_DEPTH;

        // Should not increment beyond MAX_REPEAT_DEPTH
        completion.record_attempt(100, 50, Utc::now());
        assert_eq!(completion.attempts, MAX_REPEAT_DEPTH);
    }

    #[test]
    fn test_scenario_mastery_display_methods() {
        assert_eq!(ScenarioMastery::Learning.emoji(), "🌱");
        assert_eq!(ScenarioMastery::Proficient.emoji(), "⭐");
        assert_eq!(ScenarioMastery::Mastered.emoji(), "🏆");

        assert_eq!(ScenarioMastery::Learning.display_name(), "Learning");
        assert_eq!(ScenarioMastery::Proficient.display_name(), "Proficient");
        assert_eq!(ScenarioMastery::Mastered.display_name(), "Mastered");

        assert_eq!(ScenarioMastery::Learning.xp_description(), "");
        assert_eq!(ScenarioMastery::Proficient.xp_description(), "(-50% XP)");
        assert_eq!(ScenarioMastery::Mastered.xp_description(), "(-80% XP)");
    }

    #[test]
    fn test_best_score_tracking() {
        let mut completion = ScenarioCompletion::new(sid("test"), 70, 35, Utc::now());
        assert_eq!(completion.best_score, 70);

        completion.record_attempt(85, 42, Utc::now());
        assert_eq!(completion.best_score, 85);

        completion.record_attempt(80, 40, Utc::now()); // Lower score
        assert_eq!(completion.best_score, 85); // Should stay at max
    }

    #[test]
    fn test_perfect_count_tracking() {
        let mut completion = ScenarioCompletion::new(sid("test"), 95, 47, Utc::now());
        assert_eq!(completion.perfect_count, 0);

        completion.record_attempt(100, 50, Utc::now());
        assert_eq!(completion.perfect_count, 1);

        completion.record_attempt(98, 49, Utc::now());
        assert_eq!(completion.perfect_count, 1); // No change

        completion.record_attempt(100, 10, Utc::now());
        assert_eq!(completion.perfect_count, 2);
    }

    #[test]
    fn test_history_get_nonexistent_scenario() {
        let history = ScenarioHistory::new();
        assert!(history.get(&sid("nonexistent")).is_none());
    }

    #[test]
    fn test_empty_history_stats() {
        let history = ScenarioHistory::new();
        let stats = history.mastery_stats();
        assert_eq!(stats.learning, 0);
        assert_eq!(stats.proficient, 0);
        assert_eq!(stats.mastered, 0);
    }

    #[test]
    fn test_bounded_hashmap_respects_limit() {
        let mut history = ScenarioHistory::new();

        // Fill to just below limit
        for i in 0..MAX_SCENARIOS_TRACKED {
            history.record_completion(&sid(&format!("scenario_{i}")), 100, 50, Utc::now());
        }

        assert_eq!(history.completions.len(), MAX_SCENARIOS_TRACKED);

        // Try to add one more - should be rejected
        let xp = history.record_completion(&sid("overflow_scenario"), 100, 50, Utc::now());
        assert_eq!(xp, 0); // Returns 0 when at capacity
        assert_eq!(history.completions.len(), MAX_SCENARIOS_TRACKED); // No growth

        // Existing scenarios should still update
        let xp2 = history.record_completion(&sid("scenario_0"), 100, 50, Utc::now());
        assert!(xp2 > 0); // Still gets XP for existing scenario
    }

    #[test]
    fn test_check_and_reset_daily_honors_injected_now() {
        let day_one = DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let day_two = day_one + chrono::Duration::days(1);

        let mut completion = ScenarioCompletion::new(sid("test"), 80, 40, day_one);
        assert_eq!(completion.attempts_today, 1);

        completion.record_attempt(80, 40, day_one);
        assert_eq!(
            completion.attempts_today, 2,
            "same day must accumulate, not reset"
        );

        completion.record_attempt(80, 40, day_two);
        assert_eq!(
            completion.attempts_today, 1,
            "crossing the injected day boundary must reset the daily counter"
        );
    }
}
