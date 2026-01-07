//! Performance benchmarks for scenario filtering and sorting
//!
//! This benchmark suite measures the performance of ScenarioCollection operations
//! including filtering, sorting, and menu rendering for Phase 1.5.
//!
//! Run with: cargo bench --bench filtering_sorting

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use helix_trainer::config::{
    Difficulty, Scenario, ScenarioCategory, ScenarioCollection, ScenarioFilter, ScenarioMetadata,
    ScoringConfig, Setup, Solution, SortMode, TargetState,
};
use helix_trainer::gamification::UserProfile;
use std::collections::HashSet;
use std::hint::black_box;

// Helper: Create a test scenario
fn create_scenario(
    id: &str,
    name: &str,
    category: Option<ScenarioCategory>,
    difficulty: Option<Difficulty>,
) -> Scenario {
    Scenario {
        id: id.to_string(),
        name: name.to_string(),
        description: "Test scenario".to_string(),
        setup: Setup {
            file_content: "line 1\nline 2\nline 3\n".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        target: TargetState {
            file_content: "line 2\nline 3\n".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        solution: Solution {
            commands: vec!["x".to_string()],
            description: "Delete first line".to_string(),
        },
        alternatives: vec![],
        hints: vec!["Use x to delete a line".to_string()],
        scoring: ScoringConfig {
            optimal_count: 1,
            max_points: 100,
            tolerance: 0,
        },
        metadata: Some(ScenarioMetadata {
            category,
            difficulty,
            commands_taught: vec!["x".to_string()],
            prerequisites: vec![],
            tags: vec![],
            estimated_time_seconds: Some(30),
            locale: Some("en".to_string()),
        }),
    }
}

// Helper: Generate scenarios with varied metadata
fn generate_scenarios(count: usize) -> Vec<Scenario> {
    let categories = [
        Some(ScenarioCategory::Movement),
        Some(ScenarioCategory::Editing),
        Some(ScenarioCategory::Selection),
        Some(ScenarioCategory::Clipboard),
        Some(ScenarioCategory::Search),
        None,
    ];

    let difficulties = [
        Some(Difficulty::Beginner),
        Some(Difficulty::Intermediate),
        Some(Difficulty::Advanced),
        None,
    ];

    (0..count)
        .map(|i| {
            let cat = categories[i % categories.len()];
            let diff = difficulties[i % difficulties.len()];
            create_scenario(
                &format!("scenario_{:04}", i),
                &format!("Scenario {}", i),
                cat,
                diff,
            )
        })
        .collect()
}

// Helper: Create a profile with some scenario completions
fn create_profile_with_completions(scenario_ids: &[String]) -> UserProfile {
    let mut profile = UserProfile::new();

    // Record completions for some scenarios
    for (idx, scenario_id) in scenario_ids.iter().enumerate() {
        if idx % 3 == 0 {
            // Complete ~33% of scenarios
            profile
                .scenario_history
                .record_completion(scenario_id, 100, 50);
        }
    }

    profile
}

// =============================================================================
// Benchmark: Collection Creation
// =============================================================================

fn bench_collection_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("collection_creation");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || generate_scenarios(size),
                |scenarios| black_box(ScenarioCollection::new(scenarios)),
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark: Filtering Operations
// =============================================================================

fn bench_filter_by_category(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_by_category");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let collection = ScenarioCollection::new(scenarios);

                    let mut filter_categories = HashSet::new();
                    filter_categories.insert(ScenarioCategory::Movement);
                    filter_categories.insert(ScenarioCategory::Editing);

                    let filter = ScenarioFilter {
                        categories: Some(filter_categories),
                        ..Default::default()
                    };

                    (collection, filter)
                },
                |(mut collection, filter)| {
                    collection.apply_filter(black_box(&filter), black_box(None));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_filter_by_difficulty(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_by_difficulty");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let collection = ScenarioCollection::new(scenarios);

                    let mut filter_difficulties = HashSet::new();
                    filter_difficulties.insert(Difficulty::Beginner);

                    let filter = ScenarioFilter {
                        difficulties: Some(filter_difficulties),
                        ..Default::default()
                    };

                    (collection, filter)
                },
                |(mut collection, filter)| {
                    collection.apply_filter(black_box(&filter), black_box(None));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_filter_by_completion(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_by_completion");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let scenario_ids: Vec<String> =
                        scenarios.iter().map(|s| s.id.clone()).collect();
                    let profile = create_profile_with_completions(&scenario_ids);

                    let collection = ScenarioCollection::new(scenarios);

                    let filter = ScenarioFilter {
                        not_completed_only: true,
                        ..Default::default()
                    };

                    (collection, filter, profile)
                },
                |(mut collection, filter, profile)| {
                    collection.apply_filter(black_box(&filter), black_box(Some(&profile)));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_filter_multi_criteria(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_multi_criteria");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let scenario_ids: Vec<String> =
                        scenarios.iter().map(|s| s.id.clone()).collect();
                    let profile = create_profile_with_completions(&scenario_ids);

                    let collection = ScenarioCollection::new(scenarios);

                    // Complex filter: category + difficulty + not completed
                    let mut filter_categories = HashSet::new();
                    filter_categories.insert(ScenarioCategory::Movement);

                    let mut filter_difficulties = HashSet::new();
                    filter_difficulties.insert(Difficulty::Beginner);

                    let filter = ScenarioFilter {
                        categories: Some(filter_categories),
                        difficulties: Some(filter_difficulties),
                        not_completed_only: true,
                        ..Default::default()
                    };

                    (collection, filter, profile)
                },
                |(mut collection, filter, profile)| {
                    collection.apply_filter(black_box(&filter), black_box(Some(&profile)));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark: Sorting Operations
// =============================================================================

fn bench_sort_by_name(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_by_name");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    ScenarioCollection::new(scenarios)
                },
                |mut collection| {
                    collection.sort(black_box(SortMode::ByName), black_box(None));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_sort_by_difficulty(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_by_difficulty");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    ScenarioCollection::new(scenarios)
                },
                |mut collection| {
                    collection.sort(black_box(SortMode::ByDifficulty), black_box(None));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_sort_by_category(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_by_category");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    ScenarioCollection::new(scenarios)
                },
                |mut collection| {
                    collection.sort(black_box(SortMode::ByCategory), black_box(None));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_sort_by_progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_by_progress");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let scenario_ids: Vec<String> =
                        scenarios.iter().map(|s| s.id.clone()).collect();
                    let profile = create_profile_with_completions(&scenario_ids);
                    let collection = ScenarioCollection::new(scenarios);

                    (collection, profile)
                },
                |(mut collection, profile)| {
                    collection.sort(black_box(SortMode::ByProgress), black_box(Some(&profile)));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_sort_by_category_then_difficulty(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_by_category_then_difficulty");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    ScenarioCollection::new(scenarios)
                },
                |mut collection| {
                    collection.sort(
                        black_box(SortMode::ByCategoryThenDifficulty),
                        black_box(None),
                    );
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_sort_by_mastery(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort_by_mastery");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let scenario_ids: Vec<String> =
                        scenarios.iter().map(|s| s.id.clone()).collect();
                    let profile = create_profile_with_completions(&scenario_ids);
                    let collection = ScenarioCollection::new(scenarios);

                    (collection, profile)
                },
                |(mut collection, profile)| {
                    collection.sort(black_box(SortMode::ByMastery), black_box(Some(&profile)));
                    black_box(collection)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark: Worst-Case Scenarios
// =============================================================================

fn bench_sort_reverse_sorted(c: &mut Criterion) {
    c.bench_function("sort_reverse_sorted", |b| {
        b.iter_batched(
            || {
                let mut scenarios = generate_scenarios(100);
                // Reverse sort by name
                scenarios.sort_by(|a, b| b.name.cmp(&a.name));
                ScenarioCollection::new(scenarios)
            },
            |mut collection| {
                // Sort by name (worst case: reverse sorted input)
                collection.sort(black_box(SortMode::ByName), black_box(None));
                black_box(collection)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_filter_no_matches(c: &mut Criterion) {
    c.bench_function("filter_no_matches", |b| {
        b.iter_batched(
            || {
                let scenarios = generate_scenarios(100);
                let collection = ScenarioCollection::new(scenarios);

                // Filter by commands that don't exist in any scenario
                let mut filter_commands = HashSet::new();
                filter_commands.insert("NonExistentCommand".to_string());

                let filter = ScenarioFilter {
                    commands: Some(filter_commands),
                    ..Default::default()
                };

                (collection, filter)
            },
            |(mut collection, filter)| {
                collection.apply_filter(black_box(&filter), black_box(None));
                black_box(collection)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// =============================================================================
// Benchmark: Access Patterns
// =============================================================================

fn bench_get_filtered_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_filtered_scenarios");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    ScenarioCollection::new(scenarios)
                },
                |collection| {
                    let filtered = collection.get_filtered();
                    let count = filtered.len();
                    black_box(count)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_get_filtered_by_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_filtered_by_index");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    ScenarioCollection::new(scenarios)
                },
                |collection| {
                    // Access mixle scenario and check if it exists
                    let exists = collection
                        .get_filtered_by_index(black_box(size / 2))
                        .is_some();
                    black_box(exists)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

fn bench_sequential_access(c: &mut Criterion) {
    c.bench_function("sequential_access_100", |b| {
        b.iter_batched(
            || {
                let scenarios = generate_scenarios(100);
                ScenarioCollection::new(scenarios)
            },
            |collection| {
                // Simulate menu rendering: access all scenarios sequentially
                let count = collection.count();
                for i in 0..count {
                    let exists = collection.get_filtered_by_index(i).is_some();
                    black_box(exists);
                }
                black_box(count)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// =============================================================================
// Benchmark: Menu Rendering Simulation
// =============================================================================

fn bench_menu_render_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("menu_render_simulation");

    for size in [10, 25, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("scenarios", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let scenarios = generate_scenarios(size);
                    let scenario_ids: Vec<String> =
                        scenarios.iter().map(|s| s.id.clone()).collect();
                    let profile = create_profile_with_completions(&scenario_ids);
                    let collection = ScenarioCollection::new(scenarios);

                    (collection, profile)
                },
                |(collection, profile)| {
                    // Simulate menu rendering: access all filtered scenarios
                    let filtered = collection.get_filtered();

                    // Check completion status for each scenario (like menu does)
                    let mut count = 0;
                    for scenario in &filtered {
                        let _completed = profile.scenario_history.get(&scenario.id).is_some();
                        black_box(_completed);

                        // Get difficulty indicator (like menu does)
                        let _difficulty = scenario.metadata.as_ref().and_then(|m| m.difficulty);
                        black_box(_difficulty);
                        count += 1;
                    }

                    black_box(count)
                },
                criterion::BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

// =============================================================================
// Benchmark: Complete Workflows
// =============================================================================

fn bench_filter_sort_access_workflow(c: &mut Criterion) {
    c.bench_function("filter_sort_access_workflow", |b| {
        b.iter_batched(
            || {
                let scenarios = generate_scenarios(100);
                let scenario_ids: Vec<String> = scenarios.iter().map(|s| s.id.clone()).collect();
                let profile = create_profile_with_completions(&scenario_ids);

                let collection = ScenarioCollection::new(scenarios);

                let mut filter_categories = HashSet::new();
                filter_categories.insert(ScenarioCategory::Movement);

                let filter = ScenarioFilter {
                    categories: Some(filter_categories),
                    not_completed_only: true,
                    ..Default::default()
                };

                (collection, filter, profile)
            },
            |(mut collection, filter, profile)| {
                // Apply filter
                collection.apply_filter(black_box(&filter), black_box(Some(&profile)));

                // Sort by difficulty
                collection.sort(
                    black_box(SortMode::ByCategoryThenDifficulty),
                    black_box(Some(&profile)),
                );

                // Access all filtered scenarios
                let filtered = collection.get_filtered();
                let count = filtered.len();
                black_box(count)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_repeated_filter_changes(c: &mut Criterion) {
    c.bench_function("repeated_filter_changes", |b| {
        b.iter_batched(
            || {
                let scenarios = generate_scenarios(100);
                let profile = UserProfile::new();
                let collection = ScenarioCollection::new(scenarios);

                (collection, profile)
            },
            |(mut collection, profile)| {
                // Simulate user toggling filters multiple times
                for i in 0..5 {
                    let mut filter_categories = HashSet::new();
                    if i % 2 == 0 {
                        filter_categories.insert(ScenarioCategory::Movement);
                    } else {
                        filter_categories.insert(ScenarioCategory::Editing);
                    }

                    let filter = ScenarioFilter {
                        categories: Some(filter_categories),
                        ..Default::default()
                    };

                    collection.apply_filter(&filter, Some(&profile));
                }

                black_box(collection)
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

// =============================================================================
// Benchmark: Memory Allocation
// =============================================================================

fn bench_clone_avoidance(c: &mut Criterion) {
    let mut group = c.benchmark_group("clone_avoidance");

    group.bench_function("get_filtered_with_clone", |b| {
        b.iter_batched(
            || {
                let scenarios = generate_scenarios(100);
                ScenarioCollection::new(scenarios)
            },
            |collection| {
                // Worst case: clone all scenarios
                let filtered: Vec<Scenario> = collection
                    .get_filtered()
                    .iter()
                    .map(|&s| s.clone())
                    .collect();
                black_box(filtered)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("get_filtered_with_references", |b| {
        b.iter_batched(
            || {
                let scenarios = generate_scenarios(100);
                ScenarioCollection::new(scenarios)
            },
            |collection| {
                // Best case: use references (current implementation)
                let filtered = collection.get_filtered();
                let count = filtered.len();
                black_box(count)
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(creation, bench_collection_creation,);

criterion_group!(
    filtering,
    bench_filter_by_category,
    bench_filter_by_difficulty,
    bench_filter_by_completion,
    bench_filter_multi_criteria,
);

criterion_group!(
    sorting,
    bench_sort_by_name,
    bench_sort_by_difficulty,
    bench_sort_by_category,
    bench_sort_by_progress,
    bench_sort_by_category_then_difficulty,
    bench_sort_by_mastery,
);

criterion_group!(
    worst_case,
    bench_sort_reverse_sorted,
    bench_filter_no_matches,
);

criterion_group!(
    access,
    bench_get_filtered_scenarios,
    bench_get_filtered_by_index,
    bench_sequential_access,
);

criterion_group!(rendering, bench_menu_render_simulation,);

criterion_group!(
    workflows,
    bench_filter_sort_access_workflow,
    bench_repeated_filter_changes,
);

criterion_group!(memory, bench_clone_avoidance,);

criterion_main!(
    creation, filtering, sorting, worst_case, access, rendering, workflows, memory
);
