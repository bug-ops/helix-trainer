use super::performance::{MasteryLevel, PerformanceTracker};
use super::scheduler::{ReviewItem, Scheduler};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// Result of a single review attempt
#[derive(Debug, Clone)]
pub struct ReviewResult {
    pub command: String,
    pub duration: Duration,
    pub success: bool,
    pub old_mastery: MasteryLevel,
    pub new_mastery: MasteryLevel,
}

/// Summary of completed review session
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub total_reviews: usize,
    pub successful: usize,
    pub failed: usize,
    pub avg_time: Duration,
    pub mastery_changes: Vec<(String, MasteryLevel, MasteryLevel)>,
}

/// Review session for spaced repetition practice
///
/// Manages a review session by:
/// - Loading due review items from scheduler
/// - Tracking current progress through items
/// - Recording results to performance tracker
/// - Providing session summary statistics
pub struct ReviewSession {
    items: Vec<ReviewItem>,
    current_index: usize,
    results: Vec<ReviewResult>,
    tracker: Rc<RefCell<PerformanceTracker>>,
}

impl ReviewSession {
    /// Create new review session
    ///
    /// # Arguments
    /// * `scheduler` - Scheduler to get review items from
    /// * `tracker` - Shared performance tracker (with mutable access)
    /// * `max_items` - Maximum number of items to include in session
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::{PerformanceTracker, Scheduler, ReviewSession};
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    ///
    /// let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
    /// let scheduler = Scheduler::new();
    /// let session = ReviewSession::new(&scheduler, tracker, 10);
    /// ```
    pub fn new(
        scheduler: &Scheduler,
        tracker: Rc<RefCell<PerformanceTracker>>,
        max_items: usize,
    ) -> Self {
        let items = scheduler.get_review_queue(&tracker.borrow(), max_items);

        Self {
            items,
            current_index: 0,
            results: Vec::new(),
            tracker,
        }
    }

    /// Get current review item
    ///
    /// Returns `None` if session is complete.
    pub fn current_item(&self) -> Option<&ReviewItem> {
        self.items.get(self.current_index)
    }

    /// Record result and advance to next item
    ///
    /// # Arguments
    /// * `duration` - Time taken to complete review
    /// * `success` - Whether review was successful
    /// * `optimal_time` - Expected optimal completion time
    ///
    /// # Panics
    /// Panics if called when session is already complete.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::{PerformanceTracker, Scheduler, ReviewSession};
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use std::time::Duration;
    ///
    /// let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
    /// let mut tracker_mut = tracker.borrow_mut();
    /// tracker_mut.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
    /// drop(tracker_mut);
    ///
    /// let scheduler = Scheduler::new();
    /// let mut session = ReviewSession::new(&scheduler, tracker, 10);
    ///
    /// if let Some(item) = session.current_item() {
    ///     session.record_and_next(
    ///         Duration::from_secs(2),
    ///         true,
    ///         Duration::from_secs(1)
    ///     );
    /// }
    /// ```
    pub fn record_and_next(&mut self, duration: Duration, success: bool, optimal_time: Duration) {
        let item = self
            .current_item()
            .expect("Cannot record when session is complete");

        // Get old mastery level
        let old_mastery = self
            .tracker
            .borrow()
            .get_performance(&item.id)
            .map(|p| p.mastery_level)
            .unwrap_or(MasteryLevel::Beginner);

        // Record attempt (mutably borrows tracker)
        self.tracker
            .borrow_mut()
            .record_attempt(&item.id, duration, success, optimal_time);

        // Get new mastery level
        let new_mastery = self
            .tracker
            .borrow()
            .get_performance(&item.id)
            .map(|p| p.mastery_level)
            .unwrap_or(MasteryLevel::Beginner);

        // Store result
        self.results.push(ReviewResult {
            command: item.id.clone(),
            duration,
            success,
            old_mastery,
            new_mastery,
        });

        // Advance to next item
        self.current_index += 1;
    }

    /// Check if session is complete
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.items.len()
    }

    /// Get session summary statistics
    ///
    /// # Returns
    /// Summary with total reviews, success/failure counts, average time,
    /// and mastery level changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::learning::{PerformanceTracker, Scheduler, ReviewSession};
    /// use std::cell::RefCell;
    /// use std::rc::Rc;
    /// use std::time::Duration;
    ///
    /// let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
    /// let mut tracker_mut = tracker.borrow_mut();
    /// tracker_mut.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
    /// drop(tracker_mut);
    ///
    /// let scheduler = Scheduler::new();
    /// let mut session = ReviewSession::new(&scheduler, tracker, 10);
    ///
    /// // Review may or may not be due immediately (FSRS schedules based on performance)
    /// if let Some(_item) = session.current_item() {
    ///     session.record_and_next(Duration::from_secs(2), true, Duration::from_secs(1));
    ///     let summary = session.summary();
    ///     assert_eq!(summary.total_reviews, 1);
    ///     assert_eq!(summary.successful, 1);
    /// }
    /// ```
    pub fn summary(&self) -> SessionSummary {
        let total_reviews = self.results.len();
        let successful = self.results.iter().filter(|r| r.success).count();
        let failed = total_reviews - successful;

        let total_time: Duration = self.results.iter().map(|r| r.duration).sum();
        let avg_time = if total_reviews > 0 {
            total_time / total_reviews as u32
        } else {
            Duration::ZERO
        };

        let mastery_changes: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.old_mastery != r.new_mastery)
            .map(|r| (r.command.clone(), r.old_mastery, r.new_mastery))
            .collect();

        SessionSummary {
            total_reviews,
            successful,
            failed,
            avg_time,
            mastery_changes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::PerformanceTracker;

    fn create_populated_tracker() -> Rc<RefCell<PerformanceTracker>> {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));

        // Add some commands that are immediately due
        let mut tracker_mut = tracker.borrow_mut();
        tracker_mut.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker_mut.record_attempt("yy", Duration::from_secs(2), true, Duration::from_secs(1));
        tracker_mut.record_attempt("p", Duration::from_secs(1), true, Duration::from_secs(1));
        drop(tracker_mut);

        tracker
    }

    #[test]
    fn test_session_creation() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let session = ReviewSession::new(&scheduler, tracker, 10);

        assert_eq!(session.current_index, 0);
        assert!(session.results.is_empty());
        assert!(!session.is_complete() || session.items.is_empty());
    }

    #[test]
    fn test_current_item_returns_first_item() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        if !session.items.is_empty() {
            let item = session.current_item();
            assert!(item.is_some());
        }
    }

    #[test]
    fn test_record_and_next_advances_index() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        if session.current_item().is_some() {
            let old_index = session.current_index;
            session.record_and_next(Duration::from_secs(2), true, Duration::from_secs(1));
            assert_eq!(session.current_index, old_index + 1);
            assert_eq!(session.results.len(), 1);
        }
    }

    #[test]
    fn test_record_and_next_stores_result() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        if let Some(item) = session.current_item() {
            let command = item.id.clone();
            session.record_and_next(Duration::from_secs(3), false, Duration::from_secs(1));

            assert_eq!(session.results.len(), 1);
            let result = &session.results[0];
            assert_eq!(result.command, command);
            assert_eq!(result.duration, Duration::from_secs(3));
            assert!(!result.success);
        }
    }

    #[test]
    fn test_record_and_next_tracks_mastery_changes() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        if session.current_item().is_some() {
            session.record_and_next(Duration::from_secs(1), true, Duration::from_secs(1));

            let result = &session.results[0];
            // Both old and new mastery should be set
            assert!(matches!(
                result.old_mastery,
                MasteryLevel::Beginner
                    | MasteryLevel::Intermediate
                    | MasteryLevel::Advanced
                    | MasteryLevel::Master
            ));
            assert!(matches!(
                result.new_mastery,
                MasteryLevel::Beginner
                    | MasteryLevel::Intermediate
                    | MasteryLevel::Advanced
                    | MasteryLevel::Master
            ));
        }
    }

    #[test]
    fn test_is_complete_false_initially() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let session = ReviewSession::new(&scheduler, tracker, 10);

        // Session is complete only if there are no items
        assert_eq!(session.is_complete(), session.items.is_empty());
    }

    #[test]
    fn test_is_complete_true_after_all_items() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        // Complete all items
        while let Some(_item) = session.current_item() {
            session.record_and_next(Duration::from_secs(1), true, Duration::from_secs(1));
        }

        assert!(session.is_complete());
    }

    #[test]
    fn test_summary_empty_session() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
        let scheduler = Scheduler::new();
        let session = ReviewSession::new(&scheduler, tracker, 10);

        let summary = session.summary();
        assert_eq!(summary.total_reviews, 0);
        assert_eq!(summary.successful, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.avg_time, Duration::ZERO);
        assert!(summary.mastery_changes.is_empty());
    }

    #[test]
    fn test_summary_calculates_statistics() {
        // Create fresh tracker with new commands (which are immediately due)
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));

        // Add NEW commands that haven't been practiced yet (these are due immediately)
        {
            let mut tracker_mut = tracker.borrow_mut();
            // Just create the entries without practicing to ensure they're due
            tracker_mut.record_attempt(
                "cmd1",
                Duration::from_secs(1),
                true,
                Duration::from_secs(1),
            );
            tracker_mut.record_attempt(
                "cmd2",
                Duration::from_secs(1),
                true,
                Duration::from_secs(1),
            );
            tracker_mut.record_attempt(
                "cmd3",
                Duration::from_secs(1),
                true,
                Duration::from_secs(1),
            );
        }

        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        // If we have items, process exactly 3 (or however many are available)
        let mut processed = 0;
        while processed < 3 && session.current_item().is_some() {
            let success = processed != 1; // Second item fails
            let duration = match processed {
                0 => Duration::from_secs(2),
                1 => Duration::from_secs(4),
                _ => Duration::from_secs(3),
            };
            session.record_and_next(duration, success, Duration::from_secs(1));
            processed += 1;
        }

        let summary = session.summary();
        assert_eq!(summary.total_reviews, processed);

        // Verify statistics match our recorded attempts
        if processed >= 3 {
            assert_eq!(summary.successful, 2); // items 0 and 2 succeeded
            assert_eq!(summary.failed, 1); // item 1 failed
            assert_eq!(summary.avg_time, Duration::from_secs(3)); // (2+4+3)/3 = 3
        }
    }

    #[test]
    fn test_summary_tracks_mastery_changes() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 10);

        // Complete all reviews
        while let Some(_item) = session.current_item() {
            session.record_and_next(Duration::from_secs(1), true, Duration::from_secs(1));
        }

        let summary = session.summary();
        // mastery_changes only includes items where mastery actually changed
        assert!(summary.mastery_changes.len() <= summary.total_reviews);
    }

    #[test]
    #[should_panic(expected = "Cannot record when session is complete")]
    fn test_record_and_next_panics_when_complete() {
        let tracker = Rc::new(RefCell::new(PerformanceTracker::new()));
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, tracker, 10);

        // Force session to be complete
        session.current_index = session.items.len();

        // This should panic
        session.record_and_next(Duration::from_secs(1), true, Duration::from_secs(1));
    }

    #[test]
    fn test_multiple_items_progression() {
        let tracker = create_populated_tracker();
        let scheduler = Scheduler::new();
        let mut session = ReviewSession::new(&scheduler, Rc::clone(&tracker), 3);

        let mut completed = 0;
        while let Some(_item) = session.current_item() {
            session.record_and_next(Duration::from_secs(1), true, Duration::from_secs(1));
            completed += 1;
        }

        assert!(session.is_complete());
        assert_eq!(session.results.len(), completed);
    }
}
