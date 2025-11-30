use super::performance::{MasteryLevel, PerformanceTracker};
use chrono::{DateTime, Utc};
use std::cell::RefCell;
use std::rc::Rc;

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
pub struct Analytics {
    tracker: Rc<RefCell<PerformanceTracker>>,
}

impl Analytics {
    pub fn new(tracker: Rc<RefCell<PerformanceTracker>>) -> Self {
        Self { tracker }
    }

    /// Get overall mastery distribution and averages
    pub fn get_mastery_summary(&self) -> MasterySummary {
        let tracker = self.tracker.borrow();
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
            if let Some(perf) = tracker.get_performance(command) {
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
    pub fn get_commands_by_mastery(&self, level: MasteryLevel) -> Vec<String> {
        let tracker = self.tracker.borrow();
        tracker
            .all_commands()
            .iter()
            .filter_map(|&cmd| {
                tracker.get_performance(cmd).and_then(|perf| {
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
    pub fn get_progress_over_time(&self, days: u32) -> Vec<(DateTime<Utc>, f64)> {
        if days == 0 {
            return Vec::new();
        }

        let now = Utc::now();
        let summary = self.get_mastery_summary();

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
    pub fn identify_plateaus(&self) -> Vec<String> {
        const MIN_ATTEMPTS: u32 = 5;

        let tracker = self.tracker.borrow();
        tracker
            .all_commands()
            .iter()
            .filter_map(|&cmd| {
                tracker.get_performance(cmd).and_then(|perf| {
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
    pub fn total_commands(&self) -> usize {
        self.tracker.borrow().all_commands().len()
    }

    /// Get average success rate across all commands
    pub fn avg_success_rate(&self) -> f64 {
        let tracker = self.tracker.borrow();
        let all_commands = tracker.all_commands();
        if all_commands.is_empty() {
            return 0.0;
        }

        let total: f64 = all_commands
            .iter()
            .filter_map(|&cmd| tracker.get_performance(cmd))
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

    fn create_test_tracker() -> Rc<RefCell<PerformanceTracker>> {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));

        // Create commands at different mastery levels
        {
            let mut tracker_mut = tracker.borrow_mut();

            // Beginner: new command, no practice
            tracker_mut.record_attempt(
                "beginner1",
                Duration::from_secs(5),
                false,
                Duration::from_secs(1),
            );

            // Intermediate: moderate stability
            for _ in 0..3 {
                tracker_mut.record_attempt(
                    "intermediate1",
                    Duration::from_secs(2),
                    true,
                    Duration::from_secs(1),
                );
            }

            // Advanced: high stability
            for _ in 0..10 {
                tracker_mut.record_attempt(
                    "advanced1",
                    Duration::from_secs(1),
                    true,
                    Duration::from_secs(1),
                );
            }
        }

        tracker
    }

    #[test]
    fn test_mastery_summary_empty_tracker() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
        let analytics = Analytics::new(tracker);

        let summary = analytics.get_mastery_summary();
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
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let summary = analytics.get_mastery_summary();
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
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let summary = analytics.get_mastery_summary();

        // Averages should be within valid ranges
        assert!(summary.avg_stability >= 0.0);
        assert!(summary.avg_difficulty >= 0.0);
        assert!(summary.avg_difficulty <= 10.0);
    }

    #[test]
    fn test_get_commands_by_mastery() {
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let beginners = analytics.get_commands_by_mastery(MasteryLevel::Beginner);
        let intermediates = analytics.get_commands_by_mastery(MasteryLevel::Intermediate);
        let advanced = analytics.get_commands_by_mastery(MasteryLevel::Advanced);
        let masters = analytics.get_commands_by_mastery(MasteryLevel::Master);

        // Should have at least one beginner
        assert!(!beginners.is_empty());

        // Total commands should match
        let total = beginners.len() + intermediates.len() + advanced.len() + masters.len();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_get_commands_by_mastery_specific_level() {
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let beginners = analytics.get_commands_by_mastery(MasteryLevel::Beginner);

        // Verify beginner1 is in the list
        assert!(beginners.contains(&"beginner1".to_string()));
    }

    #[test]
    fn test_progress_over_time_zero_days() {
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let progress = analytics.get_progress_over_time(0);
        assert!(progress.is_empty());
    }

    #[test]
    fn test_progress_over_time_generates_points() {
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let days = 7;
        let progress = analytics.get_progress_over_time(days);

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
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let progress = analytics.get_progress_over_time(5);

        // First point should be 0 (simulated start)
        assert_eq!(progress[0].1, 0.0);

        // Last point should match current avg stability
        let summary = analytics.get_mastery_summary();
        assert_eq!(progress.last().unwrap().1, summary.avg_stability as f64);

        // Values should increase monotonically
        for window in progress.windows(2) {
            assert!(window[0].1 <= window[1].1);
        }
    }

    #[test]
    fn test_identify_plateaus_no_plateaus() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
        let analytics = Analytics::new(tracker);

        let plateaus = analytics.identify_plateaus();
        assert!(plateaus.is_empty());
    }

    #[test]
    fn test_identify_plateaus_detects_stuck_commands() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));

        {
            let mut tracker_mut = tracker.borrow_mut();

            // Create a plateau: many attempts but low stability
            for _ in 0..10 {
                tracker_mut.record_attempt(
                    "plateau_cmd",
                    Duration::from_secs(4), // Hard rating
                    true,
                    Duration::from_secs(1),
                );
            }

            // Create a good command: many attempts with high stability
            for _ in 0..10 {
                tracker_mut.record_attempt(
                    "good_cmd",
                    Duration::from_secs(1), // Easy rating
                    true,
                    Duration::from_secs(1),
                );
            }
        }

        let analytics = Analytics::new(tracker);
        let plateaus = analytics.identify_plateaus();

        // plateau_cmd should be detected (struggling despite attempts)
        assert!(plateaus.contains(&"plateau_cmd".to_string()));
    }

    #[test]
    fn test_identify_plateaus_min_attempts() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));

        {
            let mut tracker_mut = tracker.borrow_mut();

            // Command with < 5 attempts should not be considered plateau
            for _ in 0..3 {
                tracker_mut.record_attempt(
                    "few_attempts",
                    Duration::from_secs(10),
                    false,
                    Duration::from_secs(1),
                );
            }
        }

        let analytics = Analytics::new(tracker);
        let plateaus = analytics.identify_plateaus();

        // Should not be detected (too few attempts)
        assert!(plateaus.is_empty());
    }

    #[test]
    fn test_total_commands() {
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        assert_eq!(analytics.total_commands(), 3);
    }

    #[test]
    fn test_avg_success_rate_empty() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
        let analytics = Analytics::new(tracker);

        assert_eq!(analytics.avg_success_rate(), 0.0);
    }

    #[test]
    fn test_avg_success_rate_calculation() {
        let tracker = create_test_tracker();
        let analytics = Analytics::new(tracker);

        let avg_rate = analytics.avg_success_rate();

        // Should be within valid range
        assert!(avg_rate >= 0.0);
        assert!(avg_rate <= 1.0);

        // With our test data (1 failure, rest successes), should be > 0
        assert!(avg_rate > 0.0);
    }
}
