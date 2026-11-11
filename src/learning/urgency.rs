//! Single authority for "how much does this command need practice".
//!
//! [`command_urgency`] turns a command's FSRS card into an [`Urgency`]. Consumers do not
//! share a score; each applies its own documented projection to the same urgency, so the
//! definition of "overdue", "weak" and "novel" cannot drift between them:
//!
//! - Review queue ([`super::Scheduler`]): `overdue_ratio * difficulty / 5`.
//! - Arcade selection (`minigame::ScenarioPrioritizer`):
//!   `W_overdue * min(overdue_ratio, 1) + W_weakness * weakness`, or the novelty weight.

use chrono::{DateTime, Utc};

use super::performance::CommandPerformance;

/// How urgently a command needs practice, before any consumer-specific weighting.
///
/// # Examples
///
/// ```
/// use chrono::Utc;
/// use helix_trainer::learning::{Urgency, command_urgency};
///
/// assert_eq!(command_urgency(None, Utc::now()), Urgency::Novel);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Urgency {
    /// The tracker has no entry for the command: it was never practiced.
    Novel,
    /// The command has a card, whatever its attempt count or FSRS memory.
    Practiced {
        /// Whole days past due divided by the scheduled interval (`scheduled_days`, at
        /// least 1). `0.0` while not yet due; unbounded above.
        overdue_ratio: f64,
        /// FSRS item difficulty, `0.0..=10.0`.
        difficulty: f64,
        /// `1 - success_rate`, `0.0..=1.0`; `1.0` for a card with no attempts.
        weakness: f64,
    },
}

/// Compute the [`Urgency`] of a command at `now`.
///
/// `perf` is the tracker's entry for the command. `None` means the tracker has no entry
/// and yields [`Urgency::Novel`]; a card that has attempts but no FSRS memory is still
/// [`Urgency::Practiced`]. `now` is the caller's injected clock reading, so the result is
/// deterministic under a fake clock.
///
/// # Examples
///
/// ```
/// use helix_trainer::learning::{PerformanceTracker, Urgency, command_urgency};
/// use std::time::Duration;
///
/// let mut tracker = PerformanceTracker::new();
/// tracker.record_attempt("x", Duration::from_secs(1), false, Duration::from_secs(1));
///
/// let now = tracker.clock().now();
/// match command_urgency(tracker.performance("x"), now) {
///     Urgency::Practiced { weakness, .. } => assert_eq!(weakness, 1.0),
///     Urgency::Novel => unreachable!("the tracker has an entry for x"),
/// }
/// ```
#[must_use]
pub fn command_urgency(perf: Option<&CommandPerformance>, now: DateTime<Utc>) -> Urgency {
    let Some(perf) = perf else {
        return Urgency::Novel;
    };
    let days_overdue = (now - perf.due()).num_days().max(0) as f64;
    Urgency::Practiced {
        overdue_ratio: days_overdue / f64::from(perf.scheduled_days().max(1)),
        difficulty: f64::from(perf.difficulty()),
        weakness: 1.0 - perf.success_rate(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::learning::{PerformanceData, PerformanceTracker};
    use crate::time::FakeClock;

    fn practiced(urgency: Urgency) -> (f64, f64, f64) {
        match urgency {
            Urgency::Practiced {
                overdue_ratio,
                difficulty,
                weakness,
            } => (overdue_ratio, difficulty, weakness),
            Urgency::Novel => panic!("expected a practiced command"),
        }
    }

    #[test]
    fn no_tracker_entry_is_novel() {
        assert_eq!(command_urgency(None, Utc::now()), Urgency::Novel);
    }

    #[test]
    fn card_with_attempts_but_no_memory_is_practiced_not_novel() {
        let data: PerformanceData = serde_json::from_str(
            r#"{"x": {
                "stability": 0.0, "difficulty": 5.0, "state": "New", "reps": 0,
                "lapses": 0, "attempts": 4, "successes": 1, "total_time": 4000,
                "last_review": "2030-01-01T00:00:00Z", "due": "2030-01-01T00:00:00Z",
                "scheduled_days": 0
            }}"#,
        )
        .unwrap();
        let perf = data.get("x").unwrap();
        assert!(perf.memory_state().is_none());

        let (ratio, difficulty, weakness) = practiced(command_urgency(Some(perf), perf.due()));

        assert_eq!(ratio, 0.0);
        assert_eq!(difficulty, 5.0);
        assert_eq!(weakness, 0.75);
    }

    #[test]
    fn overdue_ratio_is_days_past_due_over_the_interval() {
        let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
        let mut tracker = PerformanceTracker::with_clock(clock);
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("x", Duration::from_secs(1), true, Duration::from_secs(1));
        let perf = tracker.performance("x").unwrap();
        let interval = f64::from(perf.scheduled_days());

        let at_due = practiced(command_urgency(Some(perf), perf.due())).0;
        let two_days_late = practiced(command_urgency(
            Some(perf),
            perf.due() + chrono::Duration::days(2),
        ))
        .0;
        let before_due = practiced(command_urgency(
            Some(perf),
            perf.due() - chrono::Duration::days(2),
        ))
        .0;

        assert_eq!(at_due, 0.0);
        assert_eq!(two_days_late, 2.0 / interval);
        assert_eq!(before_due, 0.0);
    }

    #[test]
    fn same_inputs_give_the_same_urgency() {
        let clock = Arc::new(FakeClock::at("2030-01-01T00:00:00Z"));
        let mut tracker = PerformanceTracker::with_clock(clock.clone());
        tracker.record_attempt("x", Duration::from_secs(9), false, Duration::from_secs(1));
        clock.advance_days(10);

        let now = tracker.clock().now();
        let first = command_urgency(tracker.performance("x"), now);
        let second = command_urgency(tracker.performance("x"), now);

        assert_eq!(first, second);
    }
}
