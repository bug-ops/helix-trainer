//! Offline FSRS-6 versus FSRS-7 evaluation on a synthetic corpus (#437).
//!
//! Replays the same seeded review sequences through the FSRS-6 default
//! parameters and the FSRS-7 default parameters, scores every predicted
//! retrievability against the recalled-or-forgotten outcome with log-loss,
//! separately for same-day reviews (under one day since the previous review)
//! and multi-day reviews, and applies the decision rule fixed in the
//! fsrs-7-model-adoption spec (FR-006, FR-011).
//!
//! The outcome generator is neither FSRS-6 nor FSRS-7: the learner's true
//! recall follows a stretched-exponential forgetting curve with a hidden
//! stability that grows on success, shrinks on failure and carries seeded
//! noise. Everything is seeded and offline; no network is used.
//!
//! Run with: `cargo run --release --example fsrs_model_eval`

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the model API takes f32 and u32 days, and the corpus is a few thousand events"
)]

use fsrs::{DEFAULT_PARAMETERS, FSRS, FSRS6_DEFAULT_PARAMETERS, MemoryState};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

const SEEDS: [u64; 5] = [0x437_0001, 0x437_0002, 0x437_0003, 0x437_0004, 0x437_0005];
const CARDS_PER_SEED: usize = 1_500;
const REVIEWS_PER_CARD: usize = 10;
/// Probability that a timestamp is skewed back into the past.
const SKEW_PROBABILITY: f64 = 0.02;
const DESIRED_RETENTION: f32 = 0.9;
const LOG_LOSS_EPS: f64 = 1e-6;

/// FR-006: relative same-day log-loss gain needed for GO.
const GO_SAME_DAY_GAIN: f64 = 0.05;
/// FR-006: tolerated relative multi-day log-loss loss.
const GO_MULTI_DAY_TOLERANCE: f64 = 1.01;
/// FR-011 V1.
const MIN_EVENTS_PER_GROUP: usize = 5_000;
/// FR-011 V3.
const MAX_EXCLUDED_SHARE: f64 = 0.10;

#[derive(Clone, Copy)]
struct Event {
    at_days: f64,
    success: bool,
    rating: u32,
}

/// Standard normal sample by the Box-Muller transform.
fn normal(rng: &mut StdRng) -> f64 {
    let u1: f64 = rng.random::<f64>().max(f64::MIN_POSITIVE);
    let u2: f64 = rng.random();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// A value log-uniform between `low` and `high`.
fn log_uniform(rng: &mut StdRng, low: f64, high: f64) -> f64 {
    (low.ln() + rng.random::<f64>() * (high.ln() - low.ln())).exp()
}

/// The trainer's attempt-to-rating mapping (fsrs-review-rating-and-session
/// FR-004..FR-006): failure is Again; success is Easy up to a duration ratio
/// of 1.5, Good up to 3.0 and Hard beyond.
fn rating_of(success: bool, ratio: f64) -> u32 {
    match (success, ratio) {
        (false, _) => 1,
        (true, r) if r <= 1.5 => 4,
        (true, r) if r <= 3.0 => 3,
        (true, _) => 2,
    }
}

/// One card's review sequence from the model-neutral generator.
fn generate_card(rng: &mut StdRng) -> Vec<Event> {
    let mut stability = log_uniform(rng, 0.1, 1.0);
    let mut at_days = rng.random::<f64>() * 30.0;
    let mut events = Vec::with_capacity(REVIEWS_PER_CARD);
    events.push(Event {
        at_days,
        success: true,
        rating: 3,
    });
    for _ in 1..REVIEWS_PER_CARD {
        let gap = if rng.random_bool(0.55) {
            log_uniform(rng, 1.0 / 1440.0, 0.5)
        } else {
            log_uniform(rng, 1.0, 60.0)
        };
        at_days += gap;
        let recall = 0.9_f64.powf((gap / stability).powf(0.8));
        let success = rng.random_bool(recall);
        let ratio = (0.4 + 0.6 * normal(rng)).exp();
        let noise = (0.15 * normal(rng)).exp();
        stability = if success {
            let gain = 1.1 + 2.0 * (1.0 - recall) * (0.5 + rng.random::<f64>());
            stability * gain * noise
        } else {
            (stability * 0.35 * noise).max(0.05)
        };
        let shown = if rng.random_bool(SKEW_PROBABILITY) {
            at_days - log_uniform(rng, 0.01, 2.0)
        } else {
            at_days
        };
        events.push(Event {
            at_days: shown,
            success,
            rating: rating_of(success, ratio),
        });
    }
    events
}

/// How the elapsed time reaches `FSRS::next_states`.
#[derive(Clone, Copy)]
enum Elapsed {
    /// Whole days, as the trainer passes it today.
    Whole,
    /// Fractional days.
    Fractional,
}

struct Candidate {
    name: &'static str,
    fsrs: FSRS,
    elapsed: Elapsed,
}

#[derive(Default, Clone, Copy)]
struct Loss {
    sum: f64,
    count: usize,
}

impl Loss {
    fn add(&mut self, predicted: f64, success: bool) {
        let p = predicted.clamp(LOG_LOSS_EPS, 1.0 - LOG_LOSS_EPS);
        self.sum -= if success { p.ln() } else { (1.0 - p).ln() };
        self.count += 1;
    }

    fn mean(self) -> f64 {
        self.sum / self.count.max(1) as f64
    }
}

#[derive(Default, Clone, Copy)]
struct Report {
    same_day: Loss,
    multi_day: Loss,
    excluded: usize,
    total: usize,
}

fn replay(candidate: &Candidate, cards: &[Vec<Event>]) -> Report {
    let mut report = Report::default();
    for events in cards {
        let mut state: Option<MemoryState> = None;
        let mut last_at: Option<f64> = None;
        for event in events {
            report.total += 1;
            let elapsed = last_at.map_or(0.0, |last| event.at_days - last);
            if elapsed < 0.0 {
                report.excluded += 1;
                continue;
            }
            if let Some(memory) = state {
                let predicted = f64::from(
                    candidate
                        .fsrs
                        .current_retrievability(memory, elapsed as f32),
                );
                let group = if elapsed < 1.0 {
                    &mut report.same_day
                } else {
                    &mut report.multi_day
                };
                group.add(predicted, event.success);
            }
            let next = match candidate.elapsed {
                Elapsed::Whole => {
                    candidate
                        .fsrs
                        .next_states(state, DESIRED_RETENTION, elapsed.floor() as u32)
                }
                Elapsed::Fractional => candidate.fsrs.next_states_with_elapsed_days(
                    state,
                    DESIRED_RETENTION,
                    elapsed as f32,
                ),
            }
            .expect("next states for a valid memory state");
            state = Some(
                match event.rating {
                    1 => next.again,
                    2 => next.hard,
                    3 => next.good,
                    _ => next.easy,
                }
                .memory,
            );
            last_at = Some(event.at_days);
        }
    }
    report
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Go,
    NoGo,
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Self::Go => "GO",
            Self::NoGo => "NO-GO",
        }
    }
}

/// FR-006 on one corpus.
fn verdict(fsrs6: &Report, fsrs7: &Report) -> Verdict {
    let (ll6_same, ll7_same) = (fsrs6.same_day.mean(), fsrs7.same_day.mean());
    let gain_same = (ll6_same - ll7_same) / ll6_same;
    let multi_ok = fsrs7.multi_day.mean() <= GO_MULTI_DAY_TOLERANCE * fsrs6.multi_day.mean();
    if gain_same >= GO_SAME_DAY_GAIN && multi_ok {
        Verdict::Go
    } else {
        Verdict::NoGo
    }
}

fn candidates() -> [Candidate; 3] {
    [
        Candidate {
            name: "FSRS-6 default, whole-day elapsed (as shipped)",
            fsrs: FSRS::new(&FSRS6_DEFAULT_PARAMETERS).expect("FSRS-6 defaults"),
            elapsed: Elapsed::Whole,
        },
        Candidate {
            name: "FSRS-7 default, whole-day elapsed",
            fsrs: FSRS::new(&DEFAULT_PARAMETERS).expect("FSRS-7 defaults"),
            elapsed: Elapsed::Whole,
        },
        Candidate {
            name: "FSRS-7 default, fractional-day elapsed",
            fsrs: FSRS::new(&DEFAULT_PARAMETERS).expect("FSRS-7 defaults"),
            elapsed: Elapsed::Fractional,
        },
    ]
}

fn main() {
    let candidates = candidates();
    let mut totals = [Report::default(); 3];
    let mut verdicts = Vec::new();

    for seed in SEEDS {
        let mut rng = StdRng::seed_from_u64(seed);
        let cards: Vec<Vec<Event>> = (0..CARDS_PER_SEED)
            .map(|_| generate_card(&mut rng))
            .collect();
        let reports: Vec<Report> = candidates.iter().map(|c| replay(c, &cards)).collect();
        println!("seed {seed:#x}");
        for (candidate, report) in candidates.iter().zip(&reports) {
            println!(
                "  {:<48} same-day LL {:.4} (n={})  multi-day LL {:.4} (n={})  excluded {}/{}",
                candidate.name,
                report.same_day.mean(),
                report.same_day.count,
                report.multi_day.mean(),
                report.multi_day.count,
                report.excluded,
                report.total,
            );
        }
        // LL7 is FSRS-7 with fractional elapsed time: the form an adoption would use.
        let seed_verdict = verdict(&reports[0], &reports[2]);
        println!(
            "  verdict on this seed: {} (sensitivity, FSRS-7 with whole-day elapsed: {})",
            seed_verdict.label(),
            verdict(&reports[0], &reports[1]).label()
        );
        verdicts.push(seed_verdict);
        for (total, report) in totals.iter_mut().zip(&reports) {
            total.same_day.sum += report.same_day.sum;
            total.same_day.count += report.same_day.count;
            total.multi_day.sum += report.multi_day.sum;
            total.multi_day.count += report.multi_day.count;
            total.excluded += report.excluded;
            total.total += report.total;
        }
    }

    let v1 = totals[0].same_day.count >= MIN_EVENTS_PER_GROUP
        && totals[0].multi_day.count >= MIN_EVENTS_PER_GROUP;
    let v2 = SEEDS.len() >= 5 && verdicts.iter().all(|v| *v == verdicts[0]);
    let excluded_share = totals[0].excluded as f64 / totals[0].total as f64;
    let v3 = excluded_share <= MAX_EXCLUDED_SHARE;
    // V4 holds by construction: the generator is a stretched-exponential
    // forgetting curve with its own stability dynamics, not an FSRS model.
    let v4 = true;

    println!("\ncorpus validity (FR-011)");
    println!(
        "  V1 events: same-day {}, multi-day {} (need {MIN_EVENTS_PER_GROUP} each): {v1}",
        totals[0].same_day.count, totals[0].multi_day.count
    );
    println!("  V2 same verdict on {} seeds: {v2}", SEEDS.len());
    println!(
        "  V3 excluded {:.2}% (max 10%): {v3}",
        excluded_share * 100.0
    );
    println!("  V4 generator neither FSRS-6 nor FSRS-7: {v4}");

    let outcome = if v1 && v2 && v3 && v4 {
        verdicts[0].label().to_string()
    } else {
        "DEFER".to_string()
    };
    println!("\naggregate");
    for (candidate, report) in candidates.iter().zip(&totals) {
        println!(
            "  {:<48} same-day LL {:.4}  multi-day LL {:.4}",
            candidate.name,
            report.same_day.mean(),
            report.multi_day.mean()
        );
    }
    let gain = (totals[0].same_day.mean() - totals[2].same_day.mean()) / totals[0].same_day.mean();
    let multi_ratio = totals[2].multi_day.mean() / totals[0].multi_day.mean();
    println!(
        "  gain_same {:.2}%  LL7_multi/LL6_multi {multi_ratio:.4}",
        gain * 100.0
    );
    println!("\nDECISION: {outcome}");
}
