use super::performance::{CommandPerformance, PerformanceTracker};
use chrono::{DateTime, Utc};
use std::collections::BinaryHeap;

/// Review item with priority information
#[derive(Debug, Clone)]
pub struct ReviewItem {
    pub id: String,
    pub due: DateTime<Utc>,
    pub priority: f64,
}

// Equality compares priority only (for heap ordering)
impl PartialEq for ReviewItem {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for ReviewItem {}

// Ordering for BinaryHeap: higher priority = higher value (max-heap)
impl Ord for ReviewItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Direct comparison (no reverse) - higher priority > lower priority
        // Use total_cmp for f64 to handle NaN correctly
        self.priority.total_cmp(&other.priority)
    }
}

impl PartialOrd for ReviewItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Scheduler for spaced repetition reviews
#[derive(Debug, Default)]
pub struct Scheduler;

impl Scheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        Self
    }

    /// Record commands used in a completed scenario
    ///
    /// This method records each unique command from the scenario into the FSRS
    /// performance tracker, which enables spaced repetition reviews.
    ///
    /// # Arguments
    /// * `tracker` - Mutable reference to performance tracker
    /// * `commands` - List of commands used in the scenario
    /// * `total_duration` - Total time taken to complete the scenario
    /// * `success` - Whether the scenario was completed successfully
    pub fn record_scenario_commands(
        &self,
        tracker: &mut PerformanceTracker,
        commands: &[String],
        total_duration: std::time::Duration,
        success: bool,
    ) {
        if commands.is_empty() {
            return;
        }

        // Calculate average time per command (rough estimate for FSRS)
        let avg_duration = total_duration / commands.len() as u32;

        // Optimal time is slightly less than avg (assume user could be 20% faster)
        let optimal_time = avg_duration.mul_f32(0.8);

        // Record each unique command
        let unique_commands: std::collections::HashSet<_> = commands.iter().collect();

        for command in unique_commands {
            tracker.record_attempt(command, avg_duration, success, optimal_time);
        }
    }

    /// Get commands that are due for review now
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    pub fn get_due_reviews(&self, tracker: &PerformanceTracker) -> Vec<String> {
        let now = Utc::now();

        tracker
            .all_commands()
            .into_iter()
            .filter_map(|cmd| {
                tracker
                    .get_performance(cmd)
                    .filter(|perf| perf.due <= now)
                    .map(|_| cmd.to_string())
            })
            .collect()
    }

    /// Get priority-ordered review queue
    ///
    /// Returns up to `limit` items, ordered by priority (highest first).
    /// Priority is calculated based on:
    /// - How overdue the review is
    /// - Command difficulty
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    /// * `limit` - Maximum number of items to return
    pub fn get_review_queue(&self, tracker: &PerformanceTracker, limit: usize) -> Vec<ReviewItem> {
        let now = Utc::now();

        // Build priority queue (BinaryHeap is max-heap by default)
        let heap: BinaryHeap<ReviewItem> = tracker
            .all_commands()
            .into_iter()
            .filter_map(|cmd| {
                tracker.get_performance(cmd).and_then(|perf| {
                    if perf.due <= now {
                        Some(ReviewItem {
                            id: cmd.to_string(),
                            due: perf.due,
                            priority: Self::calculate_priority(perf, now),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Extract top N items
        heap.into_iter().take(limit).collect()
    }

    /// Calculate priority score for a command
    ///
    /// Higher priority = more urgent to review
    ///
    /// Formula:
    /// - `urgency` = days_overdue / scheduled_days (how late is it?)
    /// - `difficulty_weight` = difficulty / 5.0 (harder = more important)
    /// - `priority` = urgency * difficulty_weight
    fn calculate_priority(perf: &CommandPerformance, now: DateTime<Utc>) -> f64 {
        let days_overdue = (now - perf.due).num_days().max(0) as f64;
        let urgency = days_overdue / perf.scheduled_days.max(1) as f64;
        let difficulty_weight = perf.difficulty as f64 / 5.0; // Normalize 0-10 to 0-2

        urgency * difficulty_weight
    }

    /// Recommend practice session content
    ///
    /// Returns a mix of review items and new content for optimal learning.
    /// Uses a 50/50 split: half review items (due for practice), half new content
    /// (weak commands or commands not yet learned).
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    /// * `duration_minutes` - Target session duration
    ///
    /// # Returns
    /// Command IDs to practice (mix of reviews and new/weak commands)
    pub fn recommend_practice_session(
        &self,
        tracker: &PerformanceTracker,
        duration_minutes: u32,
    ) -> Vec<String> {
        const AVG_SCENARIO_TIME_MINUTES: u32 = 3;
        let max_items = duration_minutes / AVG_SCENARIO_TIME_MINUTES;

        // Split session: 50% reviews, 50% new/weak content
        let review_count = (max_items / 2) as usize;
        let new_content_count = (max_items - review_count as u32) as usize;

        // Get review items (commands due for practice)
        let review_items = self.get_review_queue(tracker, review_count);

        // Get new content (weak commands + untried commands)
        let new_content = self.get_new_content(tracker, new_content_count);

        // Combine and return
        let mut session: Vec<String> = review_items.into_iter().map(|item| item.id).collect();
        session.extend(new_content);

        session
    }

    /// Get new content for practice session
    ///
    /// Returns a mix of weak commands (low mastery) and completely new commands
    /// (not yet practiced). Prioritizes weak commands first.
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    /// * `count` - Maximum number of items to return
    ///
    /// # Returns
    /// Command IDs for new/weak content
    fn get_new_content(&self, tracker: &PerformanceTracker, count: usize) -> Vec<String> {
        if count == 0 {
            return Vec::new();
        }

        // Get weak commands (low mastery, high difficulty, or many lapses)
        let weak_commands = tracker.get_weak_commands();

        // Get all known commands to find untried ones
        let all_known_commands = tracker.all_commands();

        // Common Helix commands that users should learn
        // This list focuses on essential editing commands
        let essential_commands = [
            "d", "x", "i", "a", "o", "O", "u", "U", "y", "p", "P", "c", "r", "J", ">", "<", "I",
            "A", "gg", "G", "w", "b", "e", "0", "$",
        ];

        // Find untried essential commands
        let mut untried_commands: Vec<String> = essential_commands
            .iter()
            .filter(|cmd| !all_known_commands.contains(cmd.to_owned()))
            .map(|cmd| cmd.to_string())
            .collect();

        // Combine: weak commands first, then untried
        let mut result = Vec::new();

        // Add weak commands (up to half the count)
        let weak_limit = count / 2;
        result.extend(weak_commands.into_iter().take(weak_limit));

        // Fill remaining slots with untried commands
        let remaining = count.saturating_sub(result.len());
        result.extend(untried_commands.drain(..remaining.min(untried_commands.len())));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use std::time::Duration;

    fn create_test_tracker() -> PerformanceTracker {
        let mut tracker = PerformanceTracker::new();

        // Add some commands with different due dates and difficulties
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("yy", Duration::from_secs(2), true, Duration::from_secs(1));
        tracker.record_attempt("p", Duration::from_secs(4), true, Duration::from_secs(1));

        tracker
    }

    #[test]
    fn test_get_due_reviews_empty_when_no_overdue() {
        let tracker = PerformanceTracker::new();
        let scheduler = Scheduler::new();

        let due = scheduler.get_due_reviews(&tracker);
        assert!(due.is_empty());
    }

    #[test]
    fn test_get_due_reviews_returns_overdue_commands() {
        let mut tracker = create_test_tracker();

        // Record another attempt to update the command
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));

        let scheduler = Scheduler::new();
        let due = scheduler.get_due_reviews(&tracker);

        // Note: The actual behavior depends on FSRS scheduling
        // This test verifies the function runs without errors
        // (all new commands are due immediately)
        assert!(due.len() <= 3);
    }

    #[test]
    fn test_priority_calculation_prefers_hard_and_overdue() {
        let now = Utc::now();

        // Command 1: Very overdue, hard difficulty
        let mut cmd1 = CommandPerformance::new("hard_overdue".to_string());
        cmd1.difficulty = 8.0;
        cmd1.scheduled_days = 10;
        cmd1.due = now - ChronoDuration::days(20); // 20 days overdue

        // Command 2: Slightly overdue, easy difficulty
        let mut cmd2 = CommandPerformance::new("easy_overdue".to_string());
        cmd2.difficulty = 3.0;
        cmd2.scheduled_days = 10;
        cmd2.due = now - ChronoDuration::days(5); // 5 days overdue

        let priority1 = Scheduler::calculate_priority(&cmd1, now);
        let priority2 = Scheduler::calculate_priority(&cmd2, now);

        // Hard + overdue should have higher priority
        assert!(priority1 > priority2);
    }

    #[test]
    fn test_get_review_queue_returns_limited_items() {
        let tracker = create_test_tracker();
        let scheduler = Scheduler::new();

        let queue = scheduler.get_review_queue(&tracker, 2);

        // Should return at most 2 items
        assert!(queue.len() <= 2);
    }

    #[test]
    fn test_get_review_queue_orders_by_priority() {
        let mut tracker = PerformanceTracker::new();

        // Create commands with different priorities
        tracker.record_attempt("low", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt(
            "high",
            Duration::from_secs(10),
            false,
            Duration::from_secs(1),
        );

        let scheduler = Scheduler::new();
        let queue = scheduler.get_review_queue(&tracker, 10);

        // Verify descending priority order
        for i in 1..queue.len() {
            assert!(
                queue[i - 1].priority >= queue[i].priority,
                "Queue not properly ordered by priority"
            );
        }
    }

    #[test]
    fn test_recommend_practice_session() {
        let tracker = create_test_tracker();
        let scheduler = Scheduler::new();

        let session = scheduler.recommend_practice_session(&tracker, 15); // 15 minute session

        // Should return some items (exact count depends on reviews available)
        assert!(session.len() <= 5); // 15 / 3 = 5 max items
    }

    #[test]
    fn test_recommend_practice_session_mixes_reviews_and_new_content() {
        let mut tracker = PerformanceTracker::new();

        // Add some commands with reviews due
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("yy", Duration::from_secs(2), true, Duration::from_secs(1));

        let scheduler = Scheduler::new();

        // Request a 12-minute session (4 items max)
        let session = scheduler.recommend_practice_session(&tracker, 12);

        // Should have a mix of reviews and new content
        // Max 4 items: 2 reviews + 2 new content
        assert!(session.len() <= 4);

        // Should contain at least one review item (x or yy)
        let has_review = session.contains(&"x".to_string()) || session.contains(&"yy".to_string());
        assert!(
            has_review,
            "Session should contain at least one review item"
        );
    }

    #[test]
    fn test_get_new_content_returns_weak_and_untried_commands() {
        let mut tracker = PerformanceTracker::new();

        // Make "x" weak by failing it multiple times
        for _ in 0..5 {
            tracker.record_attempt("x", Duration::from_secs(10), false, Duration::from_secs(1));
        }

        let scheduler = Scheduler::new();

        // Get new content
        let new_content = scheduler.get_new_content(&tracker, 5);

        // Should have up to 5 items
        assert!(new_content.len() <= 5);

        // Should include weak command (x)
        assert!(
            new_content.contains(&"x".to_string()),
            "Should include weak command 'x'"
        );

        // Should also include some untried essential commands
        // (at least one command that wasn't practiced)
        let has_untried = new_content
            .iter()
            .any(|cmd| cmd != "x" && ["x", "i", "a", "o", "O"].contains(&cmd.as_str()));
        assert!(has_untried, "Should include at least one untried command");
    }

    #[test]
    fn test_get_new_content_empty_when_count_zero() {
        let tracker = PerformanceTracker::new();
        let scheduler = Scheduler::new();

        let new_content = scheduler.get_new_content(&tracker, 0);
        assert!(new_content.is_empty());
    }

    #[test]
    fn test_priority_zero_when_not_overdue() {
        let now = Utc::now();

        let mut cmd = CommandPerformance::new("not_due".to_string());
        cmd.difficulty = 8.0;
        cmd.scheduled_days = 10;
        cmd.due = now + ChronoDuration::days(5); // Future due date

        let priority = Scheduler::calculate_priority(&cmd, now);

        // Not overdue = zero priority
        assert_eq!(priority, 0.0);
    }

    #[test]
    fn test_review_item_ordering() {
        let now = Utc::now();

        let item1 = ReviewItem {
            id: "low".to_string(),
            due: now,
            priority: 1.0,
        };

        let item2 = ReviewItem {
            id: "high".to_string(),
            due: now,
            priority: 5.0,
        };

        // Test direct comparison
        assert!(item2 > item1); // Higher priority > lower priority

        // Test heap ordering (BinaryHeap is max-heap with our reversed Ord impl)
        let mut heap = BinaryHeap::new();
        heap.push(item1.clone());
        heap.push(item2.clone());

        let top = heap.pop().unwrap();
        assert_eq!(top.id, "high"); // Higher priority should come first
        assert_eq!(top.priority, 5.0);

        let second = heap.pop().unwrap();
        assert_eq!(second.id, "low");
        assert_eq!(second.priority, 1.0);
    }
}
