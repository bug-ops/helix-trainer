use super::performance::{CommandPerformance, MasteryLevel, PerformanceTracker, ReferenceTime};
use super::urgency::{Urgency, command_urgency};
use crate::ids::CommandId;
use crate::time::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;

/// Review item with priority information
#[derive(Debug, Clone)]
pub struct ReviewItem {
    /// Command id.
    pub id: CommandId,
    /// When the review becomes due.
    pub due: DateTime<Utc>,
    /// Urgency score; higher is reviewed first.
    pub priority: f64,
}

// Defined via `Ord` so that `a == b` iff `a.cmp(&b) == Equal` (incl. `-0.0` and NaN).
impl PartialEq for ReviewItem {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for ReviewItem {}

// Higher priority = greater (max-heap). Ties: earlier `due`, then smaller `id`, rank greater,
// so the queue order does not depend on hash-map iteration order.
impl Ord for ReviewItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .total_cmp(&other.priority)
            .then_with(|| other.due.cmp(&self.due))
            .then_with(|| other.id.cmp(&self.id))
    }
}

impl PartialOrd for ReviewItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Scheduler for spaced repetition reviews
pub struct Scheduler {
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler").finish_non_exhaustive()
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    fn average_duration(total: std::time::Duration, count: usize) -> std::time::Duration {
        total
            .checked_div(u32::try_from(count).unwrap_or(u32::MAX))
            .unwrap_or(std::time::Duration::ZERO)
    }

    /// Create a new scheduler backed by the system clock
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemClock))
    }

    /// Create a new scheduler backed by the given clock
    pub fn with_clock(clock: Arc<dyn Clock>) -> Self {
        Self { clock }
    }

    /// Record the commands of a finished scenario and report mastery level-ups
    ///
    /// Each distinct command is recorded once, with the scenario's average time per command,
    /// rated against `reference` (the expected time per command). Returns the commands whose
    /// mastery level rose with the new level, in the order they first occur in `commands`.
    pub fn record_scenario_commands_with_mastery(
        &self,
        tracker: &mut PerformanceTracker,
        commands: &[CommandId],
        total_duration: std::time::Duration,
        success: bool,
        reference: ReferenceTime,
    ) -> Vec<(CommandId, MasteryLevel)> {
        let avg_duration = Self::average_duration(total_duration, commands.len());
        let mut seen = HashSet::new();

        commands
            .iter()
            .filter(|command| seen.insert(*command))
            .filter_map(|command| {
                tracker
                    .record_attempt_with_mastery_change(command, avg_duration, success, reference)
                    .map(|(command, _old, new_level)| (command, new_level))
            })
            .collect()
    }

    /// Get commands that are due for review now
    ///
    /// # Arguments
    /// * `tracker` - Reference to performance tracker
    pub fn get_due_reviews(&self, tracker: &PerformanceTracker) -> Vec<CommandId> {
        let now = self.clock.now();

        tracker
            .all_commands()
            .into_iter()
            .filter_map(|cmd| {
                tracker
                    .performance(cmd)
                    .filter(|perf| perf.due() <= now)
                    .map(|_| cmd.clone())
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
        let now = self.clock.now();

        // Build priority queue (BinaryHeap is max-heap by default)
        let heap: BinaryHeap<ReviewItem> = tracker
            .all_commands()
            .into_iter()
            .filter_map(|cmd| {
                tracker.performance(cmd).and_then(|perf| {
                    if perf.due() <= now {
                        Some(ReviewItem {
                            id: cmd.clone(),
                            due: perf.due(),
                            priority: Self::calculate_priority(perf, now),
                        })
                    } else {
                        None
                    }
                })
            })
            .collect();

        heap.into_sorted_vec()
            .into_iter()
            .rev()
            .take(limit)
            .collect()
    }

    /// Calculate priority score for a command
    ///
    /// Higher priority = more urgent to review
    ///
    /// Review projection of [`command_urgency`]:
    /// - `overdue_ratio` = `days_overdue` / `scheduled_days` (how late is it?)
    /// - `difficulty_weight` = difficulty / 5.0 (harder = more important)
    /// - `priority` = `overdue_ratio` * `difficulty_weight`
    ///
    /// A [`Urgency::Novel`] command has no card and is never queued, so it scores 0.
    fn calculate_priority(perf: &CommandPerformance, now: DateTime<Utc>) -> f64 {
        match command_urgency(Some(perf), now) {
            Urgency::Practiced {
                overdue_ratio,
                difficulty,
                ..
            } => overdue_ratio * (difficulty / 5.0),
            Urgency::Novel => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::cid;
    use chrono::Duration as ChronoDuration;
    use std::time::Duration;

    fn create_test_tracker() -> PerformanceTracker {
        let mut tracker = PerformanceTracker::new();

        // Add some commands with different due dates and difficulties
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            true,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("yy"),
            Duration::from_secs(2),
            true,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("p"),
            Duration::from_secs(4),
            true,
            Duration::from_secs(1),
        );

        tracker
    }

    #[test]
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
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
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            true,
            Duration::from_secs(1),
        );

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
        let mut cmd1 = CommandPerformance::new(now);
        cmd1.set_difficulty(8.0);
        cmd1.set_scheduled_days(10);
        cmd1.set_due(now - ChronoDuration::days(20)); // 20 days overdue

        // Command 2: Slightly overdue, easy difficulty
        let mut cmd2 = CommandPerformance::new(now);
        cmd2.set_difficulty(3.0);
        cmd2.set_scheduled_days(10);
        cmd2.set_due(now - ChronoDuration::days(5)); // 5 days overdue

        let priority1 = Scheduler::calculate_priority(&cmd1, now);
        let priority2 = Scheduler::calculate_priority(&cmd2, now);

        // Hard + overdue should have higher priority
        assert!(priority1 > priority2);
    }

    #[test]
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    fn test_priority_keeps_the_pre_urgency_formula_exactly() {
        let now = Utc::now();
        for (difficulty, scheduled_days, days_overdue) in [
            (8.0, 10, 20),
            (3.0, 10, 5),
            (5.0, 1, 0),
            (9.5, 3, 7),
            (0.0, 4, 9),
        ] {
            let mut perf = CommandPerformance::new(now);
            perf.set_difficulty(difficulty);
            perf.set_scheduled_days(scheduled_days);
            perf.set_due(now - ChronoDuration::days(days_overdue));

            let expected = days_overdue as f64 / f64::from(scheduled_days.max(1))
                * (f64::from(perf.difficulty()) / 5.0);

            assert_eq!(Scheduler::calculate_priority(&perf, now), expected);
        }
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
        tracker.record_attempt(
            &cid("low"),
            Duration::from_secs(1),
            true,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("high"),
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
    fn test_priority_zero_when_not_overdue() {
        let now = Utc::now();

        let mut cmd = CommandPerformance::new(now);
        cmd.set_difficulty(8.0);
        cmd.set_scheduled_days(10);
        cmd.set_due(now + ChronoDuration::days(5)); // Future due date

        let priority = Scheduler::calculate_priority(&cmd, now);

        // Not overdue = zero priority
        assert_eq!(priority, 0.0);
    }

    #[test]
    fn test_get_due_reviews_honors_injected_clock() {
        use crate::time::FakeClock;

        let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
        let mut tracker = PerformanceTracker::with_clock(clock.clone());
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            true,
            Duration::from_secs(1),
        );
        tracker.record_attempt(
            &cid("x"),
            Duration::from_secs(1),
            true,
            Duration::from_secs(1),
        );

        let scheduled_days = tracker.performance(&cid("x")).unwrap().scheduled_days();
        assert!(
            scheduled_days >= 1,
            "second review should push the due date forward"
        );

        let scheduler = Scheduler::with_clock(clock.clone());
        assert!(
            scheduler.get_due_reviews(&tracker).is_empty(),
            "not due yet relative to the fake clock"
        );

        clock.advance_days(i64::from(scheduled_days));
        assert_eq!(
            scheduler.get_due_reviews(&tracker),
            vec![cid("x")],
            "must become due once the injected clock advances, proving Scheduler doesn't read wall-clock time"
        );
    }

    /// #433/FR-009: equal priority orders by `due` ascending, then `id` ascending,
    /// independent of hash-map iteration order, and `limit` truncates that order.
    #[test]
    fn test_get_review_queue_breaks_priority_ties_by_due_then_id() {
        use crate::time::FakeClock;

        let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
        let now = clock.now();
        let card = |id: &str, overdue: ChronoDuration| {
            let mut perf = CommandPerformance::new(now);
            perf.set_scheduled_days(10);
            perf.set_due(now - overdue);
            (cid(id), perf)
        };
        // Same whole overdue days (3) => identical priority; different `due` / `id`.
        let day3 = ChronoDuration::days(3);
        let stats = std::collections::HashMap::from([
            card("c", day3),
            card("b", day3 + ChronoDuration::hours(1)),
            card("a", day3 + ChronoDuration::hours(1)),
            card("z", day3 + ChronoDuration::hours(2)),
            card("low", ChronoDuration::days(1)),
        ]);
        let tracker = PerformanceTracker::from_stats_with_clock(stats.into(), clock.clone());
        let scheduler = Scheduler::with_clock(clock);

        let ids = |limit| -> Vec<String> {
            scheduler
                .get_review_queue(&tracker, limit)
                .into_iter()
                .map(|item| item.id.into_string())
                .collect()
        };

        assert_eq!(ids(10), ["z", "a", "b", "c", "low"]);
        assert_eq!(ids(3), ["z", "a", "b"]);
        assert_eq!(ids(1), ["z"]);
    }

    #[test]
    fn test_review_item_ordering() {
        let now = Utc::now();

        let item1 = ReviewItem {
            id: cid("low"),
            due: now,
            priority: 1.0,
        };

        let item2 = ReviewItem {
            id: cid("high"),
            due: now,
            priority: 5.0,
        };

        // Test direct comparison
        assert!(item2 > item1); // Higher priority > lower priority

        // Test heap ordering (BinaryHeap is max-heap with our reversed Ord impl)
        let mut heap = BinaryHeap::new();
        heap.push(item1);
        heap.push(item2);

        let top = heap.pop().unwrap();
        assert_eq!(top.id.as_str(), "high"); // Higher priority should come first
        assert_eq!(top.priority, 5.0);

        let second = heap.pop().unwrap();
        assert_eq!(second.id.as_str(), "low");
        assert_eq!(second.priority, 1.0);
    }
    mod props {
        use super::*;
        use crate::time::FakeClock;
        use proptest::prelude::*;
        use std::cmp::Ordering;
        use std::collections::HashSet;

        const COMMAND_POOL: usize = 8;

        #[derive(Debug, Clone)]
        struct Event {
            command: usize,
            duration_secs: u64,
            optimal_secs: u64,
            success: bool,
            elapsed_days: i64,
        }

        fn event_strategy() -> impl Strategy<Value = Event> {
            (0..COMMAND_POOL, 1u64..90, 1u64..90, any::<bool>(), 0i64..60).prop_map(
                |(command, duration_secs, optimal_secs, success, elapsed_days)| Event {
                    command,
                    duration_secs,
                    optimal_secs,
                    success,
                    elapsed_days,
                },
            )
        }

        fn review_item_strategy() -> impl Strategy<Value = ReviewItem> {
            let priority = prop_oneof![
                any::<f64>(),
                Just(f64::NAN),
                Just(0.0),
                Just(-0.0),
                Just(f64::INFINITY),
                Just(f64::NEG_INFINITY),
            ];
            ("[a-z]{1,3}", priority).prop_map(|(id, priority)| ReviewItem {
                id: cid(&format!("c{id}")),
                due: DateTime::UNIX_EPOCH,
                priority,
            })
        }

        /// Replays `events` against a tracker on a fake clock, then advances the
        /// clock by `final_days` so a mix of due and not-yet-due commands exists.
        fn build(events: &[Event], final_days: i64) -> (Scheduler, PerformanceTracker) {
            let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
            let mut tracker = PerformanceTracker::with_clock(clock.clone());
            for event in events {
                clock.advance_days(event.elapsed_days);
                tracker.record_attempt(
                    &cid(&format!("c{}", event.command)),
                    Duration::from_secs(event.duration_secs),
                    event.success,
                    Duration::from_secs(event.optimal_secs),
                );
            }
            clock.advance_days(final_days);
            (Scheduler::with_clock(clock), tracker)
        }

        fn due_ids(scheduler: &Scheduler, tracker: &PerformanceTracker) -> HashSet<CommandId> {
            scheduler.get_due_reviews(tracker).into_iter().collect()
        }

        proptest! {
            #[test]
            fn prop_review_item_ord_is_antisymmetric_and_matches_partial_ord(
                a in review_item_strategy(),
                b in review_item_strategy(),
            ) {
                prop_assert_eq!(a.cmp(&b), b.cmp(&a).reverse());
                prop_assert_eq!(a.partial_cmp(&b), Some(a.cmp(&b)));
                prop_assert_eq!(a.cmp(&a), Ordering::Equal);
            }

            #[test]
            fn prop_review_item_ord_is_transitive(
                a in review_item_strategy(),
                b in review_item_strategy(),
                c in review_item_strategy(),
            ) {
                if a <= b && b <= c {
                    prop_assert!(a <= c);
                }
                if a < b && b < c {
                    prop_assert!(a < c);
                }
            }

            #[test]
            fn prop_review_item_eq_agrees_with_ord(
                a in review_item_strategy(),
                b in review_item_strategy(),
            ) {
                prop_assert_eq!(a == b, a.cmp(&b) == Ordering::Equal);
            }

            #[test]
            fn prop_review_item_heap_pops_in_non_increasing_order(
                items in prop::collection::vec(review_item_strategy(), 0..32),
            ) {
                let mut heap: BinaryHeap<ReviewItem> = items.iter().cloned().collect();
                let mut previous: Option<ReviewItem> = None;
                while let Some(item) = heap.pop() {
                    if let Some(prev) = &previous {
                        prop_assert!(prev >= &item);
                    }
                    previous = Some(item);
                }
            }

            #[test]
            fn prop_get_due_reviews_returns_exactly_the_due_commands(
                events in prop::collection::vec(event_strategy(), 1..30),
                final_days in 0i64..400,
            ) {
                let (scheduler, tracker) = build(&events, final_days);
                prop_assume!(!scheduler.get_due_reviews(&tracker).is_empty());
                let now = scheduler.clock.now();

                let returned = scheduler.get_due_reviews(&tracker);
                let returned_set: HashSet<CommandId> = returned.iter().cloned().collect();
                prop_assert_eq!(returned.len(), returned_set.len(), "duplicate ids");

                let expected: HashSet<CommandId> = tracker
                    .all_commands()
                    .into_iter()
                    .filter(|cmd| tracker.performance(cmd).is_some_and(|p| p.due() <= now))
                    .cloned()
                    .collect();
                prop_assert_eq!(returned_set, expected);
            }

            #[test]
            fn prop_get_review_queue_respects_limit_and_selects_due_commands(
                events in prop::collection::vec(event_strategy(), 1..30),
                final_days in 0i64..400,
                limit in 0usize..12,
            ) {
                let (scheduler, tracker) = build(&events, final_days);
                prop_assume!(!scheduler.get_due_reviews(&tracker).is_empty());
                let due = due_ids(&scheduler, &tracker);

                let queue = scheduler.get_review_queue(&tracker, limit);

                prop_assert_eq!(queue.len(), limit.min(due.len()));
                let ids: HashSet<&str> = queue.iter().map(|i| i.id.as_str()).collect();
                prop_assert_eq!(ids.len(), queue.len(), "duplicate ids in queue");
                for item in &queue {
                    prop_assert!(due.contains(&item.id), "{} is not due", item.id);
                    prop_assert!(item.due <= scheduler.clock.now());
                }
            }

            #[test]
            fn prop_get_review_queue_is_sorted_by_priority_descending(
                events in prop::collection::vec(event_strategy(), 1..30),
                final_days in 0i64..400,
                limit in 0usize..12,
            ) {
                let (scheduler, tracker) = build(&events, final_days);
                prop_assume!(!scheduler.get_due_reviews(&tracker).is_empty());
                let queue = scheduler.get_review_queue(&tracker, limit);
                for [first, second] in queue.array_windows::<2>() {
                    prop_assert!(
                        first.priority.total_cmp(&second.priority) != Ordering::Less,
                        "queue not sorted: {} before {}",
                        first.priority,
                        second.priority
                    );
                }
            }

            #[test]
            fn prop_get_review_queue_keeps_the_highest_priority_items(
                events in prop::collection::vec(event_strategy(), 1..30),
                final_days in 0i64..400,
                limit in 1usize..12,
            ) {
                let (scheduler, tracker) = build(&events, final_days);
                prop_assume!(!scheduler.get_due_reviews(&tracker).is_empty());
                let full = scheduler.get_review_queue(&tracker, usize::MAX);
                let limited = scheduler.get_review_queue(&tracker, limit);

                let mut all: Vec<f64> = full.iter().map(|i| i.priority).collect();
                all.sort_by(|a, b| b.total_cmp(a));
                all.truncate(limit);
                let mut got: Vec<f64> = limited.iter().map(|i| i.priority).collect();
                got.sort_by(|a, b| b.total_cmp(a));
                prop_assert_eq!(got, all);
            }
        }
    }

    /// #551: level-ups are typed and listed in first-occurrence order, not hash order.
    #[test]
    fn test_scenario_mastery_changes_follow_first_occurrence_order() {
        let commands: Vec<_> = ["w", "b", "e", "x", "dd", "k", "w", "x"]
            .into_iter()
            .map(cid)
            .collect();
        let run = || {
            let mut tracker = PerformanceTracker::new();
            Scheduler::new().record_scenario_commands_with_mastery(
                &mut tracker,
                &commands,
                Duration::from_secs(4),
                true,
                ReferenceTime::new(Duration::from_secs(60)),
            )
        };

        let first = run();

        assert!(
            !first.is_empty(),
            "a quick first success levels commands up"
        );
        let listed: Vec<_> = first.iter().map(|(command, _)| command.clone()).collect();
        let expected: Vec<_> = ["w", "b", "e", "x", "dd", "k"]
            .into_iter()
            .map(cid)
            .collect();
        assert_eq!(listed, expected);
        for _ in 0..20 {
            assert_eq!(run(), first);
        }
    }

    /// #550: the training rating depends on the measured time against an independent reference.
    #[test]
    fn test_scenario_rating_depends_on_time_against_reference() {
        let commands = [cid("w")];
        let reference = ReferenceTime::for_optimal_actions(std::num::NonZeroUsize::new(2).unwrap());
        let stability_after = |total: Duration| {
            let mut tracker = PerformanceTracker::new();
            Scheduler::new().record_scenario_commands_with_mastery(
                &mut tracker,
                &commands,
                total,
                true,
                reference,
            );
            tracker.performance(&cid("w")).unwrap().stability()
        };

        let quick = stability_after(Duration::from_secs(6));
        let slow = stability_after(Duration::from_secs(120));

        assert!(
            quick > slow,
            "quick {quick} should out-stabilise slow {slow}"
        );
    }
}
