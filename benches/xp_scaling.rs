//! Performance benchmarks for scenario mastery XP scaling system
//!
//! This benchmark suite measures the performance impact of the new XP scaling feature,
//! including HashMap operations, DateTime operations, float arithmetic, and serialization.
//!
//! Run with: cargo bench --bench xp_scaling

use chrono::{DateTime, Utc};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hint::black_box;

// Replicate the data structures from ADR-005

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioMastery {
    Learning,
    Proficient,
    Mastered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioCompletion {
    pub scenario_id: String,
    pub attempts: u32,
    pub best_score: u32,
    pub perfect_count: u32,
    pub total_xp_earned: u64,
    pub first_attempt: DateTime<Utc>,
    pub last_attempt: DateTime<Utc>,
    pub mastery_level: ScenarioMastery,
    pub attempts_today: u32,
    pub last_attempt_date: String,
}

impl ScenarioCompletion {
    pub fn new(scenario_id: String) -> Self {
        let now = Utc::now();
        Self {
            scenario_id,
            attempts: 0,
            best_score: 0,
            perfect_count: 0,
            total_xp_earned: 0,
            first_attempt: now,
            last_attempt: now,
            mastery_level: ScenarioMastery::Learning,
            attempts_today: 0,
            last_attempt_date: now.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn record_completion(&mut self, score: u32, xp_earned: u64) {
        let now = Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        // Reset daily counter if new day
        if today != self.last_attempt_date {
            self.attempts_today = 0;
            self.last_attempt_date = today;
        }

        // Update counters
        self.attempts += 1;
        self.attempts_today += 1;
        self.best_score = self.best_score.max(score);
        if score == 100 {
            self.perfect_count += 1;
        }
        self.total_xp_earned += xp_earned;
        self.last_attempt = now;

        // Update mastery level
        self.update_mastery();
    }

    fn update_mastery(&mut self) {
        self.mastery_level = if self.perfect_count >= 2 {
            ScenarioMastery::Mastered
        } else if self.attempts >= 3 && self.best_score >= 90 {
            ScenarioMastery::Proficient
        } else {
            ScenarioMastery::Learning
        };
    }

    pub fn xp_multiplier(&self) -> f64 {
        let mastery_mult = match self.mastery_level {
            ScenarioMastery::Learning => 1.0,
            ScenarioMastery::Proficient => 0.5,
            ScenarioMastery::Mastered => 0.2,
        };

        let session_mult = match self.attempts_today {
            0 => 1.0,
            1..=2 => 0.7,
            _ => 0.3,
        };

        mastery_mult * session_mult
    }
}

pub struct ScenarioHistoryTracker {
    completions: HashMap<String, ScenarioCompletion>,
}

impl ScenarioHistoryTracker {
    pub fn new() -> Self {
        Self {
            completions: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, scenario_id: &str) -> &mut ScenarioCompletion {
        self.completions
            .entry(scenario_id.to_string())
            .or_insert_with(|| ScenarioCompletion::new(scenario_id.to_string()))
    }

    pub fn get_xp_multiplier(&self, scenario_id: &str) -> f64 {
        self.completions
            .get(scenario_id)
            .map(|c| c.xp_multiplier())
            .unwrap_or(1.0)
    }

    pub fn record_completion(&mut self, scenario_id: &str, score: u32, base_xp: u64) -> u64 {
        let multiplier = self.get_xp_multiplier(scenario_id);
        let actual_xp = (base_xp as f64 * multiplier) as u64;

        let completion = self.get_or_create(scenario_id);
        completion.record_completion(score, actual_xp);

        actual_xp
    }
}

impl Default for ScenarioHistoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

// Benchmark: XP Calculation Overhead

fn bench_xp_first_attempt(c: &mut Criterion) {
    c.bench_function("xp_first_attempt", |b| {
        b.iter_batched(
            ScenarioHistoryTracker::new,
            |mut tracker| {
                black_box(tracker.record_completion(
                    black_box("delete_line_001"),
                    black_box(100),
                    black_box(50),
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_xp_existing_record(c: &mut Criterion) {
    c.bench_function("xp_existing_record", |b| {
        b.iter_batched(
            || {
                let mut tracker = ScenarioHistoryTracker::new();
                // Pre-populate with one completion
                tracker.record_completion("delete_line_001", 100, 50);
                tracker
            },
            |mut tracker| {
                black_box(tracker.record_completion(
                    black_box("delete_line_001"),
                    black_box(100),
                    black_box(50),
                ))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_xp_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("xp_varying_sizes");

    for size in [1, 10, 100, 500, 1000, 5000] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut tracker = ScenarioHistoryTracker::new();
                    // Pre-populate with N scenarios
                    for i in 0..size {
                        tracker.record_completion(&format!("scenario_{}", i), 100, 50);
                    }
                    tracker
                },
                |mut tracker| {
                    // Lookup existing scenario (hot path)
                    black_box(tracker.record_completion(
                        black_box("scenario_0"),
                        black_box(100),
                        black_box(50),
                    ))
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// Benchmark: DateTime Operations

fn bench_datetime_now(c: &mut Criterion) {
    c.bench_function("datetime_now", |b| b.iter(|| black_box(Utc::now())));
}

fn bench_datetime_format(c: &mut Criterion) {
    let now = Utc::now();
    c.bench_function("datetime_format", |b| {
        b.iter(|| black_box(now.format("%Y-%m-%d").to_string()))
    });
}

fn bench_datetime_full_check(c: &mut Criterion) {
    c.bench_function("datetime_full_check", |b| {
        b.iter(|| {
            let now = Utc::now();
            let today = now.format("%Y-%m-%d").to_string();
            black_box(today)
        })
    });
}

// Benchmark: Float Arithmetic

fn bench_float_multiplier(c: &mut Criterion) {
    c.bench_function("float_multiplier", |b| {
        let base_xp = 50u64;
        let multiplier = 0.7f64;
        b.iter(|| {
            let result = (black_box(base_xp) as f64 * black_box(multiplier)) as u64;
            black_box(result)
        })
    });
}

fn bench_fixed_point_multiplier(c: &mut Criterion) {
    c.bench_function("fixed_point_multiplier", |b| {
        let base_xp = 50u64;
        let multiplier = 70u64; // 70% = 70/100
        b.iter(|| {
            let result = (black_box(base_xp) * black_box(multiplier)) / 100;
            black_box(result)
        })
    });
}

fn bench_multiplier_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiplier_comparison");

    let base_xp = 50u64;

    group.bench_function("float", |b| {
        let multiplier = 0.7f64;
        b.iter(|| {
            let result = (black_box(base_xp) as f64 * black_box(multiplier)) as u64;
            black_box(result)
        })
    });

    group.bench_function("fixed_point", |b| {
        let multiplier = 70u64;
        b.iter(|| {
            let result = (black_box(base_xp) * black_box(multiplier)) / 100;
            black_box(result)
        })
    });

    group.finish();
}

// Benchmark: Profile Serialization

fn bench_profile_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("profile_save");

    for size in [0, 10, 50, 100, 500, 1000] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut tracker = ScenarioHistoryTracker::new();
                    for i in 0..size {
                        tracker.record_completion(&format!("scenario_{}", i), 100, 50);
                    }
                    tracker
                },
                |tracker| black_box(serde_json::to_string(&tracker.completions).unwrap()),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_profile_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("profile_load");

    for size in [0, 10, 50, 100, 500, 1000] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut tracker = ScenarioHistoryTracker::new();
                    for i in 0..size {
                        tracker.record_completion(&format!("scenario_{}", i), 100, 50);
                    }
                    serde_json::to_string(&tracker.completions).unwrap()
                },
                |json| {
                    black_box(
                        serde_json::from_str::<HashMap<String, ScenarioCompletion>>(&json).unwrap(),
                    )
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// Benchmark: String Comparison for Date Reset

fn bench_string_date_comparison(c: &mut Criterion) {
    let date1 = "2025-11-28".to_string();
    let date2 = "2025-11-28".to_string();

    c.bench_function("string_date_comparison", |b| {
        b.iter(|| black_box(date1 != date2))
    });
}

fn bench_timestamp_comparison(c: &mut Criterion) {
    let ts1 = Utc::now().timestamp();
    let ts2 = Utc::now().timestamp();

    c.bench_function("timestamp_comparison", |b| {
        b.iter(|| black_box(ts1 / 86400 != ts2 / 86400))
    });
}

// Benchmark: HashMap vs BTreeMap

fn bench_hashmap_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashmap_lookup");

    for size in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("size", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut map = HashMap::new();
                    for i in 0..size {
                        map.insert(format!("scenario_{}", i), i);
                    }
                    map
                },
                |map| black_box(map.get("scenario_0").copied()),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_btreemap_lookup(c: &mut Criterion) {
    use std::collections::BTreeMap;

    let mut group = c.benchmark_group("btreemap_lookup");

    for size in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("size", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut map = BTreeMap::new();
                    for i in 0..size {
                        map.insert(format!("scenario_{}", i), i);
                    }
                    map
                },
                |map| black_box(map.get("scenario_0").copied()),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// Benchmark: Complete Workflow

fn bench_complete_workflow(c: &mut Criterion) {
    c.bench_function("complete_workflow", |b| {
        b.iter_batched(
            ScenarioHistoryTracker::new,
            |mut tracker| {
                // First attempt
                tracker.record_completion("scenario_1", 100, 50);
                // Second attempt
                tracker.record_completion("scenario_1", 100, 50);
                // Different scenario
                tracker.record_completion("scenario_2", 85, 50);
                // Third attempt on first scenario (mastered)
                black_box(tracker.record_completion("scenario_1", 100, 50))
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    xp_calculation,
    bench_xp_first_attempt,
    bench_xp_existing_record,
    bench_xp_varying_sizes,
);

criterion_group!(
    datetime_ops,
    bench_datetime_now,
    bench_datetime_format,
    bench_datetime_full_check,
);

criterion_group!(
    float_ops,
    bench_float_multiplier,
    bench_fixed_point_multiplier,
    bench_multiplier_comparison,
);

criterion_group!(serialization, bench_profile_save, bench_profile_load,);

criterion_group!(
    comparison_ops,
    bench_string_date_comparison,
    bench_timestamp_comparison,
);

criterion_group!(data_structures, bench_hashmap_lookup, bench_btreemap_lookup,);

criterion_group!(workflows, bench_complete_workflow);

criterion_main!(
    xp_calculation,
    datetime_ops,
    float_ops,
    serialization,
    comparison_ops,
    data_structures,
    workflows
);
