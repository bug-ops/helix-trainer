use super::performance::{CommandPerformance, MasteryLevel, PerformanceTracker, ReferenceTime};
use crate::ids::CommandId;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Whether the player recalled a reviewed command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewOutcome {
    /// The command was recalled correctly
    Recalled,
    /// The command was not recalled
    Forgot,
}

impl ReviewOutcome {
    /// Whether the command was recalled
    pub fn is_recalled(self) -> bool {
        self == Self::Recalled
    }
}

/// Result of a single review attempt
#[derive(Debug, Clone)]
pub struct ReviewResult {
    /// Reviewed command.
    pub command: CommandId,
    /// Time from showing the command to the answer.
    pub duration: Duration,
    /// Whether the attempt succeeded.
    pub success: bool,
    /// Mastery level before the attempt.
    pub old_mastery: MasteryLevel,
    /// Mastery level after the attempt.
    pub new_mastery: MasteryLevel,
}

/// Summary of a finished review session
#[derive(Debug, Clone)]
pub struct SessionSummary {
    /// Number of reviews performed.
    pub total_reviews: usize,
    /// Reviews that succeeded.
    pub successful: usize,
    /// Reviews that failed.
    pub failed: usize,
    /// Mean time per review.
    pub avg_time: Duration,
    /// Commands whose mastery level changed, with the old and new level, in review order.
    pub mastery_changes: Vec<(CommandId, MasteryLevel, MasteryLevel)>,
}

impl SessionSummary {
    fn from_results(results: &[ReviewResult]) -> Self {
        let total_reviews = results.len();
        let successful = results.iter().filter(|r| r.success).count();
        let failed = total_reviews.saturating_sub(successful);

        let total_time: Duration = results.iter().map(|r| r.duration).sum();
        let avg_time = total_time
            .checked_div(u32::try_from(total_reviews).unwrap_or(u32::MAX))
            .unwrap_or(Duration::ZERO);

        let mastery_changes = results
            .iter()
            .filter(|r| r.old_mastery != r.new_mastery)
            .map(|r| (r.command.clone(), r.old_mastery, r.new_mastery))
            .collect();

        Self {
            total_reviews,
            successful,
            failed,
            avg_time,
            mastery_changes,
        }
    }
}

/// What follows an answered review item
#[derive(Debug, Clone)]
pub enum ReviewStep {
    /// More commands are due; the session now shows the next one.
    Next(ReviewSession),
    /// The last command was answered.
    Finished(SessionSummary),
}

/// An active spaced-repetition review session
///
/// Owned by the Review screen. It always has a current command, so an empty or already
/// finished session cannot be built, and [`ReviewSession::answer`] consumes it: the answer to
/// the last command yields a [`ReviewStep::Finished`] value instead of an active session.
/// Each command is timed from the instant it was shown, not from the session start.
///
/// # Examples
///
/// ```
/// use helix_trainer::ids::CommandId;
/// use helix_trainer::learning::{PerformanceTracker, ReviewOutcome, ReviewSession, ReviewStep};
/// use std::time::{Duration, Instant};
///
/// let x = CommandId::from_card_id("x").unwrap();
/// let t0 = Instant::now();
/// let session = ReviewSession::start(vec![x.clone()], t0).expect("one command is due");
/// assert_eq!(session.current(), &x);
///
/// let mut tracker = PerformanceTracker::new();
/// let step = session.answer(
///     ReviewOutcome::Recalled,
///     t0 + Duration::from_secs(2),
///     &mut tracker,
/// );
/// assert!(matches!(step, ReviewStep::Finished(summary) if summary.successful == 1));
/// assert!(ReviewSession::start(Vec::new(), t0).is_none());
/// ```
#[derive(Debug, Clone)]
pub struct ReviewSession {
    current: CommandId,
    pending: VecDeque<CommandId>,
    shown_at: Instant,
    results: Vec<ReviewResult>,
}

impl ReviewSession {
    /// Starts a session over `due`, showing the first command at `now`
    ///
    /// Returns `None` when nothing is due.
    pub fn start(due: Vec<CommandId>, now: Instant) -> Option<Self> {
        let mut pending = VecDeque::from(due);
        let current = pending.pop_front()?;
        Some(Self {
            current,
            pending,
            shown_at: now,
            results: Vec::new(),
        })
    }

    /// The command being reviewed
    pub fn current(&self) -> &CommandId {
        &self.current
    }

    /// One-based position of the current command in the session
    pub fn position(&self) -> usize {
        self.results.len().saturating_add(1)
    }

    /// Number of commands in the session, answered or not
    pub fn total(&self) -> usize {
        self.position().saturating_add(self.pending.len())
    }

    /// Results of the commands answered so far, oldest first
    pub fn completed(&self) -> &[ReviewResult] {
        &self.results
    }

    /// Records the answer to the current command and moves on
    ///
    /// The attempt lasted from the moment the command was shown until `now`; the next command
    /// is shown at `now`. An FSRS update failure is logged and the session carries on.
    pub fn answer(
        self,
        outcome: ReviewOutcome,
        now: Instant,
        tracker: &mut PerformanceTracker,
    ) -> ReviewStep {
        let Self {
            current,
            mut pending,
            shown_at,
            mut results,
        } = self;

        let duration = now.saturating_duration_since(shown_at);
        let success = outcome.is_recalled();
        let old_mastery = mastery_of(tracker, &current);
        tracker.record_attempt_against(&current, duration, success, ReferenceTime::SINGLE_ACTION);
        let new_mastery = mastery_of(tracker, &current);

        results.push(ReviewResult {
            command: current,
            duration,
            success,
            old_mastery,
            new_mastery,
        });

        match pending.pop_front() {
            Some(next) => ReviewStep::Next(Self {
                current: next,
                pending,
                shown_at: now,
                results,
            }),
            None => ReviewStep::Finished(SessionSummary::from_results(&results)),
        }
    }
}

fn mastery_of(tracker: &PerformanceTracker, command: &CommandId) -> MasteryLevel {
    tracker
        .performance(command)
        .map_or(MasteryLevel::Beginner, CommandPerformance::mastery_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::cid;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn finished(step: ReviewStep) -> SessionSummary {
        match step {
            ReviewStep::Finished(summary) => summary,
            ReviewStep::Next(_) => panic!("session should have finished"),
        }
    }

    fn next(step: ReviewStep) -> ReviewSession {
        match step {
            ReviewStep::Next(session) => session,
            ReviewStep::Finished(_) => panic!("session should continue"),
        }
    }

    #[test]
    fn start_with_nothing_due_yields_no_session() {
        assert!(ReviewSession::start(Vec::new(), Instant::now()).is_none());
    }

    #[test]
    fn start_shows_the_first_command() {
        let session = ReviewSession::start(vec![cid("x"), cid("w")], Instant::now()).unwrap();

        assert_eq!(session.current(), &cid("x"));
        assert_eq!(session.position(), 1);
        assert_eq!(session.total(), 2);
    }

    #[test]
    fn answer_advances_in_due_order() {
        let t0 = Instant::now();
        let mut tracker = PerformanceTracker::new();
        let session = ReviewSession::start(vec![cid("x"), cid("w")], t0).unwrap();

        let session = next(session.answer(ReviewOutcome::Recalled, t0 + secs(1), &mut tracker));

        assert_eq!(session.current(), &cid("w"));
        assert_eq!(session.position(), 2);
        assert_eq!(session.total(), 2);
    }

    #[test]
    fn duration_is_measured_from_when_each_command_was_shown() {
        let t0 = Instant::now();
        let mut tracker = PerformanceTracker::new();
        let session = ReviewSession::start(vec![cid("x"), cid("w")], t0).unwrap();

        let session = next(session.answer(ReviewOutcome::Recalled, t0 + secs(10), &mut tracker));
        let summary =
            finished(session.answer(ReviewOutcome::Recalled, t0 + secs(12), &mut tracker));

        // 10 s for the first item and 2 s for the second, not 10 s and 12 s
        assert_eq!(summary.avg_time, secs(6));
    }

    #[test]
    fn summary_counts_successes_and_failures() {
        let t0 = Instant::now();
        let mut tracker = PerformanceTracker::new();
        let session = ReviewSession::start(vec![cid("x"), cid("w"), cid("b")], t0).unwrap();

        let session = next(session.answer(ReviewOutcome::Recalled, t0 + secs(1), &mut tracker));
        let session = next(session.answer(ReviewOutcome::Forgot, t0 + secs(2), &mut tracker));
        let summary = finished(session.answer(ReviewOutcome::Recalled, t0 + secs(3), &mut tracker));

        assert_eq!(summary.total_reviews, 3);
        assert_eq!(summary.successful, 2);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn answers_are_recorded_in_the_tracker() {
        let t0 = Instant::now();
        let mut tracker = PerformanceTracker::new();
        let session = ReviewSession::start(vec![cid("x")], t0).unwrap();

        finished(session.answer(ReviewOutcome::Forgot, t0 + secs(1), &mut tracker));

        let perf = tracker.performance(&cid("x")).unwrap();
        assert_eq!(perf.attempts(), 1);
        assert_eq!(perf.successes(), 0);
    }

    #[test]
    fn mastery_changes_follow_review_order() {
        let t0 = Instant::now();
        let mut tracker = PerformanceTracker::new();
        let commands = ["x", "w", "b", "e", "dd"].map(cid).to_vec();
        let mut session = ReviewSession::start(commands.clone(), t0).unwrap();
        let mut answered = 0;
        let summary = loop {
            answered += 1;
            match session.answer(ReviewOutcome::Recalled, t0 + secs(answered), &mut tracker) {
                ReviewStep::Next(s) => session = s,
                ReviewStep::Finished(summary) => break summary,
            }
        };

        let order: Vec<_> = summary
            .mastery_changes
            .iter()
            .map(|(command, _, _)| command)
            .collect();
        let expected: Vec<_> = commands.iter().filter(|c| order.contains(c)).collect();
        assert_eq!(order, expected);
    }
}
