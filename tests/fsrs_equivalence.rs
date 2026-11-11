//! FSRS-6 equivalence goldens (specs/fsrs-7-crate-migration, SC-001, SC-002, SC-004).
//!
//! `fixtures/fsrs6_goldens.json` was captured from the `fsrs` 6.6.2 build. The `fsrs` 7 build
//! must reproduce it within [`REL_TOLERANCE`]; outputs are not bitwise identical across the bump.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use helix_trainer::learning::{PerformanceData, PerformanceTracker, Scheduler};
use helix_trainer::time::FakeClock;
use serde::{Deserialize, Serialize};

fn cid(raw: &str) -> helix_trainer::ids::CommandId {
    helix_trainer::ids::CommandId::from_card_id(raw).expect("canonical card id fixture")
}

const REL_TOLERANCE: f32 = 1e-6;
const ABS_TOLERANCE: f32 = 1e-6;
const START: &str = "2026-01-15T12:00:00Z";
const OPTIMAL: Duration = Duration::from_secs(1);
const GAPS_DAYS: [i64; 6] = [0, 1, 3, 10, 60, 365];
const THREE_STEP_GAPS: [[i64; 2]; 2] = [[1, 7], [3, 30]];
const RETRIEVABILITY_GRID_DAYS: [i64; 20] = [
    0, 1, 2, 3, 5, 7, 10, 14, 21, 30, 45, 60, 90, 120, 150, 200, 250, 300, 365, 500,
];
const PROFILE_FIXTURE: &str = include_str!("fixtures/fsrs6_profile_performance.json");
const GOLDENS: &str = include_str!("fixtures/fsrs6_goldens.json");
const PROFILE_NOW: &str = "2026-03-01T12:00:00Z";

#[derive(Debug, Clone, Copy)]
enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

impl Rating {
    const ALL: [Self; 4] = [Self::Again, Self::Hard, Self::Good, Self::Easy];

    fn tag(self) -> char {
        match self {
            Self::Again => 'A',
            Self::Hard => 'H',
            Self::Good => 'G',
            Self::Easy => 'E',
        }
    }

    fn attempt(self) -> (Duration, bool) {
        match self {
            Self::Again => (Duration::from_secs(1), false),
            Self::Hard => (Duration::from_secs(4), true),
            Self::Good => (Duration::from_secs(2), true),
            Self::Easy => (Duration::from_secs(1), true),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StepGolden {
    stability: f32,
    difficulty: f32,
    scheduled_days: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SequenceGolden {
    label: String,
    steps: Vec<StepGolden>,
    retrievability: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProfileGolden {
    command: String,
    retrievability: f32,
    due: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Goldens {
    sequences: Vec<SequenceGolden>,
    profile: Vec<ProfileGolden>,
}

fn parse_time(rfc3339: &str) -> DateTime<Utc> {
    rfc3339.parse().expect("valid RFC 3339 literal")
}

/// Plays `ratings` with `gaps_days[i]` days elapsed before review `i + 1`.
fn play(ratings: &[Rating], gaps_days: &[i64]) -> SequenceGolden {
    let clock = Arc::new(FakeClock::at(START));
    let mut tracker = PerformanceTracker::with_clock(clock.clone());
    let mut label = String::new();
    let mut steps = Vec::new();
    for (index, rating) in ratings.iter().enumerate() {
        if index > 0 {
            clock.advance_days(gaps_days[index - 1]);
            label.push_str(&format!("+{}d>", gaps_days[index - 1]));
        }
        label.push(rating.tag());
        let (duration, success) = rating.attempt();
        tracker.record_attempt(&cid("x"), duration, success, OPTIMAL);
        let card = tracker
            .performance(&cid("x"))
            .expect("card exists after attempt");
        steps.push(StepGolden {
            stability: card.stability(),
            difficulty: card.difficulty(),
            scheduled_days: card.scheduled_days(),
        });
    }
    let card = tracker.performance(&cid("x")).expect("card exists");
    let last_review = card.last_review();
    let retrievability = RETRIEVABILITY_GRID_DAYS
        .iter()
        .map(|days| card.retrievability(last_review + chrono::Duration::days(*days)))
        .collect();
    SequenceGolden {
        label,
        steps,
        retrievability,
    }
}

fn corpus() -> Vec<SequenceGolden> {
    let mut sequences = Vec::new();
    for first in Rating::ALL {
        sequences.push(play(&[first], &[]));
        for gap in GAPS_DAYS {
            for second in Rating::ALL {
                sequences.push(play(&[first, second], &[gap]));
            }
        }
        for gaps in THREE_STEP_GAPS {
            for second in Rating::ALL {
                for third in Rating::ALL {
                    sequences.push(play(&[first, second, third], &gaps));
                }
            }
        }
    }
    sequences
}

fn profile_observations() -> Vec<ProfileGolden> {
    let data: PerformanceData =
        serde_json::from_str(PROFILE_FIXTURE).expect("profile fixture deserializes");
    let now = parse_time(PROFILE_NOW);
    let tracker = PerformanceTracker::from_stats(data);
    let scheduler = Scheduler::with_clock(Arc::new(FakeClock::new(now)));
    let due = scheduler.get_due_reviews(&tracker);
    let mut commands = tracker.all_commands();
    commands.sort_unstable();
    commands
        .into_iter()
        .map(|command| ProfileGolden {
            command: command.as_str().to_owned(),
            retrievability: tracker
                .performance(command)
                .expect("listed command has a card")
                .retrievability(now),
            due: due.iter().any(|d| d == command),
        })
        .collect()
}

fn assert_close_rel(actual: f32, expected: f32, context: &str) {
    let scale = expected.abs().max(f32::MIN_POSITIVE);
    assert!(
        (actual - expected).abs() / scale <= REL_TOLERANCE,
        "{context}: actual {actual} vs golden {expected}"
    );
}

#[test]
fn sequences_match_fsrs6_goldens() {
    let goldens: Goldens = serde_json::from_str(GOLDENS).expect("goldens parse");
    let actual = corpus();
    assert_eq!(actual.len(), goldens.sequences.len(), "corpus size drifted");
    for (got, want) in actual.iter().zip(&goldens.sequences) {
        assert_eq!(got.label, want.label);
        assert_eq!(got.steps.len(), want.steps.len(), "{}", want.label);
        for (index, (g, w)) in got.steps.iter().zip(&want.steps).enumerate() {
            let context = format!("{} step {index}", want.label);
            assert_close_rel(g.stability, w.stability, &format!("{context} stability"));
            assert_close_rel(g.difficulty, w.difficulty, &format!("{context} difficulty"));
            assert_eq!(
                g.scheduled_days, w.scheduled_days,
                "{context} scheduled_days"
            );
        }
        for (index, (g, w)) in got
            .retrievability
            .iter()
            .zip(&want.retrievability)
            .enumerate()
        {
            assert!(
                (g - w).abs() <= ABS_TOLERANCE,
                "{} retrievability[{index}]: {g} vs {w}",
                want.label
            );
        }
    }
}

#[test]
fn legacy_profile_fixture_matches_fsrs6_goldens() {
    let goldens: Goldens = serde_json::from_str(GOLDENS).expect("goldens parse");
    let actual = profile_observations();
    assert_eq!(actual.len(), goldens.profile.len());
    for (got, want) in actual.iter().zip(&goldens.profile) {
        assert_eq!(got.command, want.command);
        assert_eq!(got.due, want.due, "{} due flag", want.command);
        assert!(
            (got.retrievability - want.retrievability).abs() <= ABS_TOLERANCE,
            "{} retrievability: {} vs {}",
            want.command,
            got.retrievability,
            want.retrievability
        );
    }
}
