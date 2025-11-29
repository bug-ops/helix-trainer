//! Scenario completion tracking and mastery system
//!
//! Prevents XP farming by tracking scenario mastery and scaling rewards.
//! Implements a three-tier mastery system: Learning → Proficient → Mastered

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum recursion depth for repeat command protection
const MAX_REPEAT_DEPTH: u32 = 100;

/// Maximum number of scenarios to track in history (prevents unbounded growth)
/// Current application has ~20 scenarios, this allows room for 500x growth
const MAX_SCENARIOS_TRACKED: usize = 10_000;

/// Maximum length for scenario IDs (defense in depth)
const MAX_SCENARIO_ID_LENGTH: usize = 100;

/// Validate scenario ID format (alphanumeric, underscores, hyphens only)
///
/// Defense in depth - scenario IDs are already validated at TOML load time,
/// but this provides an additional safety check at the storage boundary.
fn is_valid_scenario_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_SCENARIO_ID_LENGTH
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
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
    /// Get emoji representation for UI display
    pub fn emoji(&self) -> &'static str {
        match self {
            ScenarioMastery::Learning => "🌱",
            ScenarioMastery::Proficient => "⭐",
            ScenarioMastery::Mastered => "🏆",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            ScenarioMastery::Learning => "Learning",
            ScenarioMastery::Proficient => "Proficient",
            ScenarioMastery::Mastered => "Mastered",
        }
    }

    /// Get XP reduction description
    pub fn xp_description(&self) -> &'static str {
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
    pub scenario_id: String,

    // Performance tracking
    pub attempts: u32,
    pub best_score: u32,
    pub perfect_count: u32, // Number of 100% completions
    pub total_xp_earned: u64,

    // Timestamps
    pub first_attempt: DateTime<Utc>,
    pub last_attempt: DateTime<Utc>,

    // Derived state
    pub mastery_level: ScenarioMastery,

    // Today's session tracking (reset at midnight)
    pub attempts_today: u32,
    pub last_attempt_date: NaiveDate,
}

impl ScenarioCompletion {
    /// Create new completion record for first attempt
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::ScenarioCompletion;
    ///
    /// let completion = ScenarioCompletion::new("delete_line_001".to_string(), 80, 40);
    /// assert_eq!(completion.attempts, 1);
    /// assert_eq!(completion.best_score, 80);
    /// ```
    pub fn new(scenario_id: String, score: u32, xp_earned: u64) -> Self {
        let now = Utc::now();
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
        completion.record_attempt(score, xp_earned);
        completion
    }

    /// Record a new attempt and update mastery
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::ScenarioCompletion;
    ///
    /// let mut completion = ScenarioCompletion::new("test".to_string(), 80, 40);
    /// completion.record_attempt(100, 50);
    /// assert_eq!(completion.attempts, 2);
    /// assert_eq!(completion.best_score, 100);
    /// ```
    pub fn record_attempt(&mut self, score: u32, xp_earned: u64) {
        // Prevent infinite recursion with depth protection
        if self.attempts >= MAX_REPEAT_DEPTH {
            return;
        }

        let now = Utc::now();

        // Reset daily counter if new day
        self.check_and_reset_daily();

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
    /// use helix_trainer::learning::ScenarioCompletion;
    ///
    /// let completion = ScenarioCompletion::new("test".to_string(), 50, 25);
    /// assert_eq!(completion.xp_multiplier(), 0.7); // Second attempt today (after construction)
    /// ```
    pub fn xp_multiplier(&self) -> f64 {
        // Base multiplier from mastery
        let mastery_mult = match self.mastery_level {
            ScenarioMastery::Learning => 1.0,
            ScenarioMastery::Proficient => 0.5,
            ScenarioMastery::Mastered => 0.2,
        };

        // Session repeat penalty (anti-spam)
        let session_mult = match self.attempts_today {
            0 => 1.0, // First today (before current attempt)
            1 => 0.7, // Second attempt today
            2 => 0.7, // Third attempt today
            _ => 0.3, // Heavy reduction (3rd+ repeat)
        };

        mastery_mult * session_mult
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
    fn check_and_reset_daily(&mut self) {
        let today = Utc::now().date_naive();
        if today != self.last_attempt_date {
            self.attempts_today = 0;
            self.last_attempt_date = today;
        }
    }
}

/// Manager for all scenario completion history
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioHistory {
    completions: HashMap<String, ScenarioCompletion>,
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
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let history = ScenarioHistory::new();
    /// assert!(history.get("test").is_none());
    /// ```
    pub fn get(&self, scenario_id: &str) -> Option<&ScenarioCompletion> {
        self.completions.get(scenario_id)
    }

    /// Record a scenario completion with score and base XP
    ///
    /// Returns actual XP awarded after mastery scaling
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let mut history = ScenarioHistory::new();
    /// let xp = history.record_completion("test", 100, 50);
    /// assert_eq!(xp, 50); // First attempt, full XP
    /// ```
    pub fn record_completion(&mut self, scenario_id: &str, score: u32, base_xp: u64) -> u64 {
        // Validate scenario ID format (defense in depth)
        if !is_valid_scenario_id(scenario_id) {
            return 0;
        }

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
            .map(|c| c.xp_multiplier())
            .unwrap_or(1.0); // First attempt = full XP

        // Round to nearest integer to avoid floating point truncation issues
        let actual_xp = (base_xp as f64 * multiplier).round() as u64;

        // Record completion (creates new or updates existing)
        self.completions
            .entry(scenario_id.to_string())
            .and_modify(|c| c.record_attempt(score, actual_xp))
            .or_insert_with(|| ScenarioCompletion::new(scenario_id.to_string(), score, actual_xp));

        actual_xp
    }

    /// Get mastery statistics (count per level)
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::ScenarioHistory;
    ///
    /// let mut history = ScenarioHistory::new();
    /// history.record_completion("test1", 100, 50);
    /// history.record_completion("test2", 80, 40);
    ///
    /// let stats = history.mastery_stats();
    /// assert_eq!(stats.learning, 2);
    /// assert_eq!(stats.proficient, 0);
    /// assert_eq!(stats.mastered, 0);
    /// ```
    pub fn mastery_stats(&self) -> MasteryStats {
        let mut stats = MasteryStats::default();

        for completion in self.completions.values() {
            match completion.mastery_level {
                ScenarioMastery::Learning => stats.learning += 1,
                ScenarioMastery::Proficient => stats.proficient += 1,
                ScenarioMastery::Mastered => stats.mastered += 1,
            }
        }

        stats
    }
}

/// Statistics about scenario mastery levels
#[derive(Debug, Clone, Default)]
pub struct MasteryStats {
    pub learning: u32,
    pub proficient: u32,
    pub mastered: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_completion_initializes_correctly() {
        let completion = ScenarioCompletion::new("test_scenario".to_string(), 85, 42);

        assert_eq!(completion.scenario_id, "test_scenario");
        assert_eq!(completion.attempts, 1);
        assert_eq!(completion.best_score, 85);
        assert_eq!(completion.perfect_count, 0);
        assert_eq!(completion.total_xp_earned, 42);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.attempts_today, 1);
    }

    #[test]
    fn test_mastery_progression_to_proficient() {
        let mut completion = ScenarioCompletion::new("test".to_string(), 85, 40);

        // Need 3+ attempts AND 90+ score for proficient
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);

        completion.record_attempt(92, 45);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning); // Only 2 attempts

        completion.record_attempt(95, 47);
        assert_eq!(completion.mastery_level, ScenarioMastery::Proficient); // 3 attempts + 95 score
        assert_eq!(completion.attempts, 3);
        assert_eq!(completion.best_score, 95);
    }

    #[test]
    fn test_mastery_progression_to_mastered() {
        let mut completion = ScenarioCompletion::new("test".to_string(), 100, 50);

        assert_eq!(completion.perfect_count, 1);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning); // Need 2 perfect

        completion.record_attempt(100, 50);
        assert_eq!(completion.perfect_count, 2);
        assert_eq!(completion.mastery_level, ScenarioMastery::Mastered);
    }

    #[test]
    fn test_xp_multiplier_learning_phase() {
        let completion = ScenarioCompletion::new("test".to_string(), 80, 40);
        // After new(), attempts_today = 1, so next call gets session penalty
        assert_eq!(completion.xp_multiplier(), 0.7); // Second attempt today (1.0 * 0.7)
    }

    #[test]
    fn test_xp_multiplier_session_penalty() {
        // Use scores < 90 to avoid triggering proficiency (needs 90+ AND 3+ attempts)
        let mut completion = ScenarioCompletion::new("test".to_string(), 80, 40);

        // First attempt already recorded in new(), attempts_today = 1
        assert_eq!(completion.attempts_today, 1);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.xp_multiplier(), 0.7); // Second today (1.0 * 0.7)

        completion.record_attempt(85, 35);
        assert_eq!(completion.attempts_today, 2);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.xp_multiplier(), 0.7); // Third today (1.0 * 0.7)

        completion.record_attempt(88, 30); // Keep < 90 to stay in learning
        assert_eq!(completion.attempts_today, 3);
        assert_eq!(completion.mastery_level, ScenarioMastery::Learning);
        assert_eq!(completion.xp_multiplier(), 0.3); // Fourth today (1.0 * 0.3)
    }

    #[test]
    fn test_xp_multiplier_mastered() {
        let mut completion = ScenarioCompletion::new("test".to_string(), 100, 50);
        completion.record_attempt(100, 10); // Now mastered (2 perfect)

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

        let xp = history.record_completion("test", 100, 50);
        assert_eq!(xp, 50); // First attempt, full XP

        let completion = history.get("test").unwrap();
        assert_eq!(completion.attempts, 1);
        assert_eq!(completion.best_score, 100);
        assert_eq!(completion.total_xp_earned, 50);
    }

    #[test]
    fn test_scenario_history_repeated_completions() {
        let mut history = ScenarioHistory::new();

        // First attempt (creates new completion with attempts_today=1)
        let xp1 = history.record_completion("test", 100, 50);
        assert_eq!(xp1, 50); // Full XP (first attempt gets 1.0 multiplier)

        // Second attempt (attempts_today=1 before call, session_mult=0.7, still learning)
        let xp2 = history.record_completion("test", 100, 50);
        assert_eq!(xp2, 35); // 50 * 0.7 = 35 (session penalty)

        // Third attempt
        // After 2nd completion: perfect_count=2 (Mastered!), attempts_today=2
        // multiplier = mastery(0.2) * session(2->0.7) = 0.14
        // XP = 50 * 0.14 = 7.0 (rounded)
        let xp3 = history.record_completion("test", 100, 50);
        assert_eq!(xp3, 7);
    }

    #[test]
    fn test_scenario_history_mastery_stats() {
        let mut history = ScenarioHistory::new();

        // Create scenarios at different mastery levels
        history.record_completion("learning1", 80, 40);
        history.record_completion("learning2", 50, 25);

        history.record_completion("proficient", 90, 45);
        history.record_completion("proficient", 92, 23); // 2nd attempt
        history.record_completion("proficient", 95, 24); // 3rd attempt -> proficient

        history.record_completion("mastered", 100, 50);
        history.record_completion("mastered", 100, 10); // 2nd perfect -> mastered

        let stats = history.mastery_stats();
        assert_eq!(stats.learning, 2);
        assert_eq!(stats.proficient, 1);
        assert_eq!(stats.mastered, 1);
    }

    #[test]
    fn test_saturating_arithmetic_prevents_overflow() {
        let mut completion = ScenarioCompletion::new("test".to_string(), 100, u64::MAX - 100);

        // Should not panic on overflow (note: new() already added some XP)
        completion.record_attempt(80, 1000); // Use non-perfect to avoid mastery
        assert_eq!(completion.total_xp_earned, u64::MAX);

        // Test attempts saturation (but MAX_REPEAT_DEPTH = 100 prevents this)
        // Once attempts >= 100, record_attempt returns early
        completion.attempts = 50;
        completion.record_attempt(80, 100);
        assert_eq!(completion.attempts, 51); // Increments normally

        completion.attempts = MAX_REPEAT_DEPTH - 1;
        completion.record_attempt(80, 100);
        assert_eq!(completion.attempts, MAX_REPEAT_DEPTH); // Reaches limit

        completion.record_attempt(80, 100);
        assert_eq!(completion.attempts, MAX_REPEAT_DEPTH); // Blocked by MAX_REPEAT_DEPTH
    }

    #[test]
    fn test_max_repeat_depth_protection() {
        let mut completion = ScenarioCompletion::new("test".to_string(), 100, 50);
        completion.attempts = MAX_REPEAT_DEPTH;

        // Should not increment beyond MAX_REPEAT_DEPTH
        completion.record_attempt(100, 50);
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
        let mut completion = ScenarioCompletion::new("test".to_string(), 70, 35);
        assert_eq!(completion.best_score, 70);

        completion.record_attempt(85, 42);
        assert_eq!(completion.best_score, 85);

        completion.record_attempt(80, 40); // Lower score
        assert_eq!(completion.best_score, 85); // Should stay at max
    }

    #[test]
    fn test_perfect_count_tracking() {
        let mut completion = ScenarioCompletion::new("test".to_string(), 95, 47);
        assert_eq!(completion.perfect_count, 0);

        completion.record_attempt(100, 50);
        assert_eq!(completion.perfect_count, 1);

        completion.record_attempt(98, 49);
        assert_eq!(completion.perfect_count, 1); // No change

        completion.record_attempt(100, 10);
        assert_eq!(completion.perfect_count, 2);
    }

    #[test]
    fn test_history_get_nonexistent_scenario() {
        let history = ScenarioHistory::new();
        assert!(history.get("nonexistent").is_none());
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
            history.record_completion(&format!("scenario_{}", i), 100, 50);
        }

        assert_eq!(history.completions.len(), MAX_SCENARIOS_TRACKED);

        // Try to add one more - should be rejected
        let xp = history.record_completion("overflow_scenario", 100, 50);
        assert_eq!(xp, 0); // Returns 0 when at capacity
        assert_eq!(history.completions.len(), MAX_SCENARIOS_TRACKED); // No growth

        // Existing scenarios should still update
        let xp2 = history.record_completion("scenario_0", 100, 50);
        assert!(xp2 > 0); // Still gets XP for existing scenario
    }

    #[test]
    fn test_scenario_id_validation() {
        let mut history = ScenarioHistory::new();

        // Valid IDs
        assert!(history.record_completion("valid_scenario_001", 100, 50) > 0);
        assert!(history.record_completion("test-scenario-2", 100, 50) > 0);
        assert!(history.record_completion("a1b2c3", 100, 50) > 0);

        // Invalid IDs - should be rejected
        assert_eq!(history.record_completion("", 100, 50), 0); // Empty
        assert_eq!(history.record_completion("../../../etc/passwd", 100, 50), 0); // Path traversal
        assert_eq!(history.record_completion("drop table;", 100, 50), 0); // SQL injection attempt
        assert_eq!(history.record_completion(&"x".repeat(101), 100, 50), 0); // Too long

        // Only valid IDs should be stored
        assert_eq!(history.completions.len(), 3);
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_is_valid_scenario_id_accepts_valid() {
        assert!(is_valid_scenario_id("delete_line_001"));
        assert!(is_valid_scenario_id("test-scenario"));
        assert!(is_valid_scenario_id("a1b2c3"));
        assert!(is_valid_scenario_id("Movement123"));
    }

    #[test]
    fn test_is_valid_scenario_id_rejects_invalid() {
        assert!(!is_valid_scenario_id("")); // Empty
        assert!(!is_valid_scenario_id("../../etc/passwd")); // Path traversal
        assert!(!is_valid_scenario_id("drop table;")); // Special chars
        assert!(!is_valid_scenario_id("test scenario")); // Space
        assert!(!is_valid_scenario_id(&"x".repeat(101))); // Too long
    }
}
