//! Tests for ScenarioCollection

use super::*;
use crate::config::{CursorSpec, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState};

/// Helper to create a test scenario with metadata
fn create_scenario(
    id: &str,
    name: &str,
    category: Option<ScenarioCategory>,
    difficulty: Option<Difficulty>,
    commands: Vec<&str>,
) -> Scenario {
    Scenario {
        id: id.to_string(),
        name: name.to_string(),
        description: "Test scenario".to_string(),
        setup: Setup {
            file_content: "test".to_string(),
            cursor: CursorSpec {
                cursor_position: Some((0, 0)),
                selection: None,
                cursors: None,
                selections: None,
            },
        },
        target: TargetState {
            file_content: "test".to_string(),
            cursor: CursorSpec {
                cursor_position: Some((0, 0)),
                selection: None,
                cursors: None,
                selections: None,
            },
        },
        solution: Solution {
            commands: vec!["test".to_string()],
            description: "Test solution".to_string(),
        },
        alternatives: vec![],
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
            max_points: 100,
            tolerance: 0,
        },
        metadata: Some(ScenarioMetadata {
            category,
            difficulty,
            tags: vec![],
            commands_taught: commands.iter().map(|s| s.to_string()).collect(),
            prerequisites: vec![],
            estimated_time_seconds: None,
            locale: None,
        }),
    }
}

#[test]
fn test_collection_creation() {
    let scenarios = vec![
        create_scenario(
            "001",
            "First",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Second",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Intermediate),
            vec!["d"],
        ),
    ];

    let collection = ScenarioCollection::new(scenarios);

    assert_eq!(collection.total_count(), 2);
    assert_eq!(collection.count(), 2); // No filter applied yet
}

#[test]
fn test_filter_by_category() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Movement1",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Editing1",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
        create_scenario(
            "003",
            "Movement2",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Intermediate),
            vec!["w"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);

    let filter = ScenarioFilter {
        categories: Some([ScenarioCategory::Movement].iter().copied().collect()),
        ..Default::default()
    };

    collection.apply_filter(&filter, None);

    assert_eq!(collection.count(), 2); // Only Movement scenarios
    let filtered = collection.get_filtered();
    assert_eq!(filtered[0].name, "Movement1");
    assert_eq!(filtered[1].name, "Movement2");
}

#[test]
fn test_filter_by_difficulty() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Easy1",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Hard1",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Advanced),
            vec!["d"],
        ),
        create_scenario(
            "003",
            "Easy2",
            Some(ScenarioCategory::Clipboard),
            Some(Difficulty::Beginner),
            vec!["y"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);

    let filter = ScenarioFilter {
        difficulties: Some([Difficulty::Beginner].iter().copied().collect()),
        ..Default::default()
    };

    collection.apply_filter(&filter, None);

    assert_eq!(collection.count(), 2); // Only Beginner scenarios
}

#[test]
fn test_filter_by_command() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Delete",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d", "x"],
        ),
        create_scenario(
            "002",
            "Movement",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h", "j"],
        ),
        create_scenario(
            "003",
            "Mixed",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Intermediate),
            vec!["d", "w"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);

    let filter = ScenarioFilter {
        commands: Some(["d".to_string()].iter().cloned().collect()),
        ..Default::default()
    };

    collection.apply_filter(&filter, None);

    assert_eq!(collection.count(), 2); // Scenarios teaching 'd' command
    let filtered = collection.get_filtered();
    assert_eq!(filtered[0].name, "Delete");
    assert_eq!(filtered[1].name, "Mixed");
}

#[test]
fn test_filter_multiple_criteria() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Easy Movement",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Hard Movement",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Advanced),
            vec!["w"],
        ),
        create_scenario(
            "003",
            "Easy Editing",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);

    let filter = ScenarioFilter {
        categories: Some([ScenarioCategory::Movement].iter().copied().collect()),
        difficulties: Some([Difficulty::Beginner].iter().copied().collect()),
        ..Default::default()
    };

    collection.apply_filter(&filter, None);

    assert_eq!(collection.count(), 1); // Only beginner movement
    assert_eq!(collection.get_filtered()[0].name, "Easy Movement");
}

#[test]
fn test_sort_by_name() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Zebra",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Apple",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
        create_scenario(
            "003",
            "Middle",
            Some(ScenarioCategory::Clipboard),
            Some(Difficulty::Beginner),
            vec!["y"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);
    collection.sort(SortMode::ByName, None);

    let sorted = collection.get_filtered();
    assert_eq!(sorted[0].name, "Apple");
    assert_eq!(sorted[1].name, "Middle");
    assert_eq!(sorted[2].name, "Zebra");
}

#[test]
fn test_sort_by_difficulty() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Advanced",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Advanced),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Beginner",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
        create_scenario(
            "003",
            "Intermediate",
            Some(ScenarioCategory::Clipboard),
            Some(Difficulty::Intermediate),
            vec!["y"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);
    collection.sort(SortMode::ByDifficulty, None);

    let sorted = collection.get_filtered();
    assert_eq!(sorted[0].name, "Beginner");
    assert_eq!(sorted[1].name, "Intermediate");
    assert_eq!(sorted[2].name, "Advanced");
}

#[test]
fn test_sort_by_category() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Clipboard",
            Some(ScenarioCategory::Clipboard),
            Some(Difficulty::Beginner),
            vec!["y"],
        ),
        create_scenario(
            "002",
            "Movement",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "003",
            "Editing",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);
    collection.sort(SortMode::ByCategory, None);

    let sorted = collection.get_filtered();
    // Categories are sorted by enum order
    assert_eq!(sorted[0].name, "Movement");
    assert_eq!(sorted[1].name, "Editing");
    assert_eq!(sorted[2].name, "Clipboard");
}

#[test]
fn test_sort_by_category_then_difficulty() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Movement Advanced",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Advanced),
            vec!["w"],
        ),
        create_scenario(
            "002",
            "Editing Beginner",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["x"],
        ),
        create_scenario(
            "003",
            "Movement Beginner",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "004",
            "Editing Advanced",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Advanced),
            vec!["c"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);
    collection.sort(SortMode::ByCategoryThenDifficulty, None);

    let sorted = collection.get_filtered();
    // Should group by category, then sort by difficulty within each group
    assert_eq!(sorted[0].name, "Movement Beginner");
    assert_eq!(sorted[1].name, "Movement Advanced");
    assert_eq!(sorted[2].name, "Editing Beginner");
    assert_eq!(sorted[3].name, "Editing Advanced");
}

#[test]
fn test_sort_by_difficulty_then_category() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Movement Advanced",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Advanced),
            vec!["w"],
        ),
        create_scenario(
            "002",
            "Editing Beginner",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["x"],
        ),
        create_scenario(
            "003",
            "Movement Beginner",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "004",
            "Editing Advanced",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Advanced),
            vec!["c"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);
    collection.sort(SortMode::ByDifficultyThenCategory, None);

    let sorted = collection.get_filtered();
    // Should sort by difficulty first, then by category within each difficulty level
    // Beginner: Movement, Editing (category order)
    // Advanced: Movement, Editing (category order)
    assert_eq!(sorted[0].name, "Movement Beginner");
    assert_eq!(sorted[1].name, "Editing Beginner");
    assert_eq!(sorted[2].name, "Movement Advanced");
    assert_eq!(sorted[3].name, "Editing Advanced");
}

#[test]
fn test_reset_filter() {
    let scenarios = vec![
        create_scenario(
            "001",
            "First",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Second",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Intermediate),
            vec!["d"],
        ),
    ];

    let mut collection = ScenarioCollection::new(scenarios);

    // Apply filter
    let filter = ScenarioFilter {
        categories: Some([ScenarioCategory::Movement].iter().copied().collect()),
        ..Default::default()
    };
    collection.apply_filter(&filter, None);
    assert_eq!(collection.count(), 1);

    // Reset filter
    collection.reset_filter();
    assert_eq!(collection.count(), 2); // Back to all scenarios
}

#[test]
fn test_get_categories() {
    let scenarios = vec![
        create_scenario(
            "001",
            "First",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Second",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
        create_scenario(
            "003",
            "Third",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["w"],
        ),
    ];

    let collection = ScenarioCollection::new(scenarios);
    let categories = collection.get_categories();

    assert_eq!(categories.len(), 2); // Movement and Editing
    assert!(categories.contains(&ScenarioCategory::Movement));
    assert!(categories.contains(&ScenarioCategory::Editing));
}

#[test]
fn test_get_difficulties() {
    let scenarios = vec![
        create_scenario(
            "001",
            "Easy",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Medium",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Intermediate),
            vec!["d"],
        ),
        create_scenario(
            "003",
            "Hard",
            Some(ScenarioCategory::Clipboard),
            Some(Difficulty::Advanced),
            vec!["y"],
        ),
    ];

    let collection = ScenarioCollection::new(scenarios);
    let difficulties = collection.get_difficulties();

    assert_eq!(difficulties.len(), 3);
    assert_eq!(difficulties[0], Difficulty::Beginner);
    assert_eq!(difficulties[1], Difficulty::Intermediate);
    assert_eq!(difficulties[2], Difficulty::Advanced);
}

#[test]
fn test_get_filtered_by_index() {
    let scenarios = vec![
        create_scenario(
            "001",
            "First",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
        create_scenario(
            "002",
            "Second",
            Some(ScenarioCategory::Editing),
            Some(Difficulty::Beginner),
            vec!["d"],
        ),
    ];

    let collection = ScenarioCollection::new(scenarios);

    assert_eq!(collection.get_filtered_by_index(0).unwrap().name, "First");
    assert_eq!(collection.get_filtered_by_index(1).unwrap().name, "Second");
    assert!(collection.get_filtered_by_index(2).is_none());
}

#[test]
fn test_filter_empty_result() {
    let scenarios = vec![create_scenario(
        "001",
        "Movement",
        Some(ScenarioCategory::Movement),
        Some(Difficulty::Beginner),
        vec!["h"],
    )];

    let mut collection = ScenarioCollection::new(scenarios);

    let filter = ScenarioFilter {
        categories: Some([ScenarioCategory::Editing].iter().copied().collect()),
        ..Default::default()
    };

    collection.apply_filter(&filter, None);

    assert_eq!(collection.count(), 0); // No matching scenarios
    assert!(collection.get_filtered().is_empty());
}

#[test]
fn test_scenarios_without_metadata() {
    // Scenario without metadata
    let scenario_no_meta = Scenario {
        id: "no_meta".to_string(),
        name: "No Metadata".to_string(),
        description: "Test".to_string(),
        setup: Setup {
            file_content: "test".to_string(),
            cursor: CursorSpec {
                cursor_position: Some((0, 0)),
                selection: None,
                cursors: None,
                selections: None,
            },
        },
        target: TargetState {
            file_content: "test".to_string(),
            cursor: CursorSpec {
                cursor_position: Some((0, 0)),
                selection: None,
                cursors: None,
                selections: None,
            },
        },
        solution: Solution {
            commands: vec!["test".to_string()],
            description: "Test".to_string(),
        },
        alternatives: vec![],
        hints: vec![],
        scoring: ScoringConfig {
            optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
            max_points: 100,
            tolerance: 0,
        },
        metadata: None, // No metadata
    };

    let scenarios = vec![
        scenario_no_meta,
        create_scenario(
            "001",
            "With Metadata",
            Some(ScenarioCategory::Movement),
            Some(Difficulty::Beginner),
            vec!["h"],
        ),
    ];

    let collection = ScenarioCollection::new(scenarios);

    // Filtering by category should exclude scenario without metadata
    let filter = ScenarioFilter {
        categories: Some([ScenarioCategory::Movement].iter().copied().collect()),
        ..Default::default()
    };

    let mut filtered_collection = collection.clone();
    filtered_collection.apply_filter(&filter, None);

    assert_eq!(filtered_collection.count(), 1);
    assert_eq!(filtered_collection.get_filtered()[0].name, "With Metadata");
}

#[test]
fn test_active_filter_and_sort() {
    let scenarios = vec![create_scenario(
        "001",
        "First",
        Some(ScenarioCategory::Movement),
        Some(Difficulty::Beginner),
        vec!["h"],
    )];

    let mut collection = ScenarioCollection::new(scenarios);

    // Check initial state (default is ByDifficultyThenCategory)
    assert_eq!(collection.active_sort(), SortMode::ByDifficultyThenCategory);

    // Apply filter and sort
    let filter = ScenarioFilter {
        categories: Some([ScenarioCategory::Movement].iter().copied().collect()),
        ..Default::default()
    };
    collection.apply_filter(&filter, None);
    collection.sort(SortMode::ByName, None);

    // Check active filter and sort are tracked
    assert!(collection.active_filter().categories.is_some());
    assert_eq!(collection.active_sort(), SortMode::ByName);
}
