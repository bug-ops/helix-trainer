use super::performance::{MasteryLevel, PerformanceTracker};
use chrono::{DateTime, Utc};

/// Summary of mastery distribution across all commands
#[derive(Debug, Clone)]
pub struct MasterySummary {
    pub total_commands: usize,
    pub master: usize,
    pub advanced: usize,
    pub intermediate: usize,
    pub beginner: usize,
    pub avg_stability: f32,
    pub avg_difficulty: f32,
}

/// Analytics for learning progress and performance insights
#[derive(Debug, Default)]
pub struct Analytics;

impl Analytics {
    /// Get overall mastery distribution and averages
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    pub fn get_mastery_summary(tracker: &PerformanceTracker) -> MasterySummary {
        let all_commands = tracker.all_commands();
        let total_commands = all_commands.len();

        if total_commands == 0 {
            return MasterySummary {
                total_commands: 0,
                master: 0,
                advanced: 0,
                intermediate: 0,
                beginner: 0,
                avg_stability: 0.0,
                avg_difficulty: 0.0,
            };
        }

        let mut master = 0;
        let mut advanced = 0;
        let mut intermediate = 0;
        let mut beginner = 0;
        let mut total_stability = 0.0;
        let mut total_difficulty = 0.0;

        for command in &all_commands {
            if let Some(perf) = tracker.performance(command) {
                use super::ProgressionTier;
                match perf.mastery_level.tier_level() {
                    3 => master += 1,
                    2 => advanced += 1,
                    1 => intermediate += 1,
                    _ => beginner += 1,
                }
                total_stability += perf.stability;
                total_difficulty += perf.difficulty;
            }
        }

        MasterySummary {
            total_commands,
            master,
            advanced,
            intermediate,
            beginner,
            avg_stability: total_stability / total_commands as f32,
            avg_difficulty: total_difficulty / total_commands as f32,
        }
    }

    /// Get all commands at a specific mastery level
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    /// * `level` - Mastery level to filter by
    pub fn get_commands_by_mastery(
        tracker: &PerformanceTracker,
        level: MasteryLevel,
    ) -> Vec<String> {
        tracker
            .all_commands()
            .iter()
            .filter_map(|&cmd| {
                tracker.performance(cmd).and_then(|perf| {
                    if perf.mastery_level == level {
                        Some(cmd.to_string())
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Get progress over time (average stability trend)
    ///
    /// Note: Currently simulates historical data based on current state.
    /// In production, this would read from stored historical snapshots.
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    /// * `days` - Number of days to simulate
    pub fn get_progress_over_time(
        tracker: &PerformanceTracker,
        days: u32,
        now: DateTime<Utc>,
    ) -> Vec<(DateTime<Utc>, f64)> {
        if days == 0 {
            return Vec::new();
        }

        let summary = Self::get_mastery_summary(tracker);

        // Simulate historical progress (linear growth for now)
        // In production, this would be actual historical data
        let current_avg = summary.avg_stability as f64;
        let step = current_avg / days as f64;

        (0..=days)
            .map(|day| {
                let timestamp = now - chrono::Duration::days((days - day) as i64);
                let avg_stability = step * day as f64;
                (timestamp, avg_stability)
            })
            .collect()
    }

    /// Identify commands that have plateaued (many attempts but low stability)
    ///
    /// A plateau is defined as:
    /// - High number of attempts (>= 5)
    /// - Low stability relative to attempts (stability < attempts / 2.0)
    /// - Not at Master level
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    pub fn identify_plateaus(tracker: &PerformanceTracker) -> Vec<String> {
        const MIN_ATTEMPTS: u32 = 5;

        tracker
            .all_commands()
            .iter()
            .filter_map(|&cmd| {
                tracker.performance(cmd).and_then(|perf| {
                    let expected_stability = perf.attempts as f32 / 2.0;

                    if perf.attempts >= MIN_ATTEMPTS
                        && perf.stability < expected_stability
                        && perf.mastery_level != MasteryLevel::Master
                    {
                        Some(cmd.to_string())
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Get total number of tracked commands
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    pub fn total_commands(tracker: &PerformanceTracker) -> usize {
        tracker.all_commands().len()
    }

    /// Get average success rate across all commands
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    pub fn avg_success_rate(tracker: &PerformanceTracker) -> f64 {
        let all_commands = tracker.all_commands();
        if all_commands.is_empty() {
            return 0.0;
        }

        let total: f64 = all_commands
            .iter()
            .filter_map(|&cmd| tracker.performance(cmd))
            .map(|perf| perf.success_rate())
            .sum();

        total / all_commands.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::performance::PerformanceTracker;
    use std::time::Duration;

    fn setup_tracker_with_varied_mastery() -> PerformanceTracker {
        let mut tracker = PerformanceTracker::new();

        // Beginner: new command, no practice
        tracker.record_attempt(
            "beginner1",
            Duration::from_secs(5),
            false,
            Duration::from_secs(1),
        );

        // Intermediate: moderate stability
        for _ in 0..3 {
            tracker.record_attempt(
                "intermediate1",
                Duration::from_secs(2),
                true,
                Duration::from_secs(1),
            );
        }

        // Advanced: high stability
        for _ in 0..10 {
            tracker.record_attempt(
                "advanced1",
                Duration::from_secs(1),
                true,
                Duration::from_secs(1),
            );
        }

        tracker
    }

    #[test]
    fn test_mastery_summary_empty_tracker() {
        let tracker = PerformanceTracker::new();

        let summary = Analytics::get_mastery_summary(&tracker);
        assert_eq!(summary.total_commands, 0);
        assert_eq!(summary.master, 0);
        assert_eq!(summary.advanced, 0);
        assert_eq!(summary.intermediate, 0);
        assert_eq!(summary.beginner, 0);
        assert_eq!(summary.avg_stability, 0.0);
        assert_eq!(summary.avg_difficulty, 0.0);
    }

    #[test]
    fn test_mastery_summary_counts() {
        let tracker = setup_tracker_with_varied_mastery();

        let summary = Analytics::get_mastery_summary(&tracker);
        assert_eq!(summary.total_commands, 3);

        // At least one beginner should exist
        assert!(summary.beginner >= 1);

        // Total should match
        assert_eq!(
            summary.total_commands,
            summary.master + summary.advanced + summary.intermediate + summary.beginner
        );
    }

    #[test]
    fn test_mastery_summary_averages() {
        let tracker = setup_tracker_with_varied_mastery();

        let summary = Analytics::get_mastery_summary(&tracker);

        // Averages should be within valid ranges
        assert!(summary.avg_stability >= 0.0);
        assert!(summary.avg_difficulty >= 0.0);
        assert!(summary.avg_difficulty <= 10.0);
    }

    #[test]
    fn test_get_commands_by_mastery() {
        let tracker = setup_tracker_with_varied_mastery();

        let beginners = Analytics::get_commands_by_mastery(&tracker, MasteryLevel::Beginner);
        let intermediates =
            Analytics::get_commands_by_mastery(&tracker, MasteryLevel::Intermediate);
        let advanced = Analytics::get_commands_by_mastery(&tracker, MasteryLevel::Advanced);
        let masters = Analytics::get_commands_by_mastery(&tracker, MasteryLevel::Master);

        // Should have at least one beginner
        assert!(!beginners.is_empty());

        // Total commands should match
        let total = beginners.len() + intermediates.len() + advanced.len() + masters.len();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_get_commands_by_mastery_specific_level() {
        let tracker = setup_tracker_with_varied_mastery();

        let beginners = Analytics::get_commands_by_mastery(&tracker, MasteryLevel::Beginner);

        // Verify beginner1 is in the list
        assert!(beginners.contains(&"beginner1".to_string()));
    }

    #[test]
    fn test_progress_over_time_zero_days() {
        let tracker = setup_tracker_with_varied_mastery();

        let progress = Analytics::get_progress_over_time(&tracker, 0, Utc::now());
        assert!(progress.is_empty());
    }

    #[test]
    fn test_progress_over_time_generates_points() {
        let tracker = setup_tracker_with_varied_mastery();

        let days = 7;
        let progress = Analytics::get_progress_over_time(&tracker, days, Utc::now());

        // Should have days+1 points (including day 0)
        assert_eq!(progress.len(), (days + 1) as usize);

        // Check timestamps are in ascending order
        for window in progress.windows(2) {
            assert!(window[0].0 < window[1].0);
        }

        // Check values are non-negative and ascending
        for (_, value) in &progress {
            assert!(*value >= 0.0);
        }
    }

    #[test]
    fn test_progress_over_time_growth() {
        let tracker = setup_tracker_with_varied_mastery();

        let progress = Analytics::get_progress_over_time(&tracker, 5, Utc::now());

        // First point should be 0 (simulated start)
        assert_eq!(progress[0].1, 0.0);

        // Last point should match current avg stability
        let summary = Analytics::get_mastery_summary(&tracker);
        assert_eq!(progress.last().unwrap().1, summary.avg_stability as f64);

        // Values should increase monotonically
        for window in progress.windows(2) {
            assert!(window[0].1 <= window[1].1);
        }
    }

    #[test]
    fn test_identify_plateaus_no_plateaus() {
        let tracker = PerformanceTracker::new();

        let plateaus = Analytics::identify_plateaus(&tracker);
        assert!(plateaus.is_empty());
    }

    #[test]
    fn test_identify_plateaus_detects_stuck_commands() {
        let mut tracker = PerformanceTracker::new();

        // Create a plateau: many attempts but low stability
        for _ in 0..10 {
            tracker.record_attempt(
                "plateau_cmd",
                Duration::from_secs(4), // Hard rating
                true,
                Duration::from_secs(1),
            );
        }

        // Create a good command: many attempts with high stability
        for _ in 0..10 {
            tracker.record_attempt(
                "good_cmd",
                Duration::from_secs(1), // Easy rating
                true,
                Duration::from_secs(1),
            );
        }

        let plateaus = Analytics::identify_plateaus(&tracker);

        // plateau_cmd should be detected (struggling despite attempts)
        assert!(plateaus.contains(&"plateau_cmd".to_string()));
    }

    #[test]
    fn test_identify_plateaus_min_attempts() {
        let mut tracker = PerformanceTracker::new();

        // Command with < 5 attempts should not be considered plateau
        for _ in 0..3 {
            tracker.record_attempt(
                "few_attempts",
                Duration::from_secs(10),
                false,
                Duration::from_secs(1),
            );
        }

        let plateaus = Analytics::identify_plateaus(&tracker);

        // Should not be detected (too few attempts)
        assert!(plateaus.is_empty());
    }

    #[test]
    fn test_total_commands() {
        let tracker = setup_tracker_with_varied_mastery();

        assert_eq!(Analytics::total_commands(&tracker), 3);
    }

    #[test]
    fn test_avg_success_rate_empty() {
        let tracker = PerformanceTracker::new();

        assert_eq!(Analytics::avg_success_rate(&tracker), 0.0);
    }

    #[test]
    fn test_avg_success_rate_calculation() {
        let tracker = setup_tracker_with_varied_mastery();

        let avg_rate = Analytics::avg_success_rate(&tracker);

        // Should be within valid range
        assert!(avg_rate >= 0.0);
        assert!(avg_rate <= 1.0);

        // With our test data (1 failure, rest successes), should be > 0
        assert!(avg_rate > 0.0);
    }
}
