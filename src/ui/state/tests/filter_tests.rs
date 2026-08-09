//! Tests for filter and sorting handlers

use super::common::{create_test_app_state, create_test_scenario};
use crate::config::{CompletionFilter, Difficulty, ScenarioCategory, SortMode};
use crate::testing::ScenarioBuilder;
use crate::ui::state::{Message, TypedScreen, update};

/// Create scenarios with different categories and difficulties
fn create_diverse_scenarios() -> Vec<crate::config::Scenario> {
    vec![
        ScenarioBuilder::new()
            .id("movement_001")
            .category(ScenarioCategory::Movement)
            .difficulty(Difficulty::Beginner)
            .build(),
        ScenarioBuilder::new()
            .id("editing_001")
            .category(ScenarioCategory::Editing)
            .difficulty(Difficulty::Intermediate)
            .build(),
        ScenarioBuilder::new()
            .id("selection_001")
            .category(ScenarioCategory::Selection)
            .difficulty(Difficulty::Advanced)
            .build(),
        ScenarioBuilder::new()
            .id("movement_002")
            .category(ScenarioCategory::Movement)
            .difficulty(Difficulty::Beginner)
            .build(),
    ]
}

#[test]
fn test_set_sort_mode_by_name() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    update(&mut state, Message::SetSortMode(SortMode::ByName)).unwrap();

    // Check that sort mode is applied (scenarios should be sorted)
    let count = state.game.scenario_collection.count();
    assert_eq!(count, 4);
}

#[test]
fn test_set_sort_mode_by_difficulty() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    update(&mut state, Message::SetSortMode(SortMode::ByDifficulty)).unwrap();

    let count = state.game.scenario_collection.count();
    assert_eq!(count, 4);
}

#[test]
fn test_toggle_category_filter_add() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Initially 4 scenarios
    assert_eq!(state.game.scenario_collection.count(), 4);

    // Filter by Movement category
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();

    // Should only show Movement scenarios (2)
    assert_eq!(state.game.scenario_collection.count(), 2);
}

#[test]
fn test_toggle_category_filter_remove() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Add Movement filter
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();
    assert_eq!(state.game.scenario_collection.count(), 2);

    // Remove Movement filter (toggle off)
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();

    // Should show all scenarios again
    assert_eq!(state.game.scenario_collection.count(), 4);
}

#[test]
fn test_toggle_multiple_category_filters() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Filter by Movement and Editing
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Editing),
    )
    .unwrap();

    // Should show Movement (2) + Editing (1) = 3
    assert_eq!(state.game.scenario_collection.count(), 3);
}

#[test]
fn test_toggle_difficulty_filter_add() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Filter by Beginner difficulty
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();

    // Should only show Beginner scenarios (2)
    assert_eq!(state.game.scenario_collection.count(), 2);
}

#[test]
fn test_toggle_difficulty_filter_remove() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Add Beginner filter
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();
    assert_eq!(state.game.scenario_collection.count(), 2);

    // Remove Beginner filter
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();

    // Should show all scenarios again
    assert_eq!(state.game.scenario_collection.count(), 4);
}

#[test]
fn test_toggle_multiple_difficulty_filters() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Filter by Beginner and Intermediate
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Intermediate),
    )
    .unwrap();

    // Should show Beginner (2) + Intermediate (1) = 3
    assert_eq!(state.game.scenario_collection.count(), 3);
}

#[test]
fn test_toggle_completed_filter_cycle() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Initially showing all
    assert_eq!(
        state.game.scenario_collection.active_filter().completion,
        CompletionFilter::Any
    );
    assert_eq!(state.game.scenario_collection.count(), 1);

    // First toggle: Show only completed (should show 0, nothing completed)
    update(&mut state, Message::ToggleCompletedFilter).unwrap();
    assert_eq!(
        state.game.scenario_collection.active_filter().completion,
        CompletionFilter::CompletedOnly
    );

    // Second toggle: Show only not completed
    update(&mut state, Message::ToggleCompletedFilter).unwrap();
    assert_eq!(
        state.game.scenario_collection.active_filter().completion,
        CompletionFilter::NotCompletedOnly
    );

    // Third toggle: Show all again
    update(&mut state, Message::ToggleCompletedFilter).unwrap();
    assert_eq!(
        state.game.scenario_collection.active_filter().completion,
        CompletionFilter::Any
    );
    assert_eq!(state.game.scenario_collection.count(), 1);
}

#[test]
fn test_reset_filters() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Apply some filters
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();

    // Should have filtered count
    let filtered_count = state.game.scenario_collection.count();
    assert!(filtered_count <= 4);

    // Reset all filters
    update(&mut state, Message::ResetFilters).unwrap();

    // Should show all scenarios
    assert_eq!(state.game.scenario_collection.count(), 4);
}

#[test]
fn test_combined_category_and_difficulty_filters() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Filter by Movement category AND Beginner difficulty
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();

    // Should show only Movement+Beginner scenarios (2)
    assert_eq!(state.game.scenario_collection.count(), 2);
}

#[test]
fn test_filter_with_empty_result() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);

    // Filter by a combination that doesn't exist
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Selection),
    )
    .unwrap();
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();

    // Selection+Beginner doesn't exist, should show 0
    assert_eq!(state.game.scenario_collection.count(), 0);
}

// ==================== Regression tests for #312 ====================
// Filter toggles that shrink the filtered scenario list must clamp
// MenuData::selected_item back into bounds, or the menu can end up
// pointing at an index past the end of the (now shorter) item list.

fn selected_menu_item(state: &crate::ui::state::AppState) -> usize {
    match &state.screen {
        TypedScreen::Menu(menu_data) => menu_data.selected_item,
        other => panic!("expected TypedScreen::Menu, got {other:?}"),
    }
}

fn set_selected_menu_item(state: &mut crate::ui::state::AppState, index: usize) {
    match &mut state.screen {
        TypedScreen::Menu(menu_data) => menu_data.selected_item = index,
        other => panic!("expected TypedScreen::Menu, got {other:?}"),
    }
}

#[test]
fn test_toggle_category_filter_clamps_selection_when_list_shrinks() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);
    update(&mut state, Message::SelectTrainingMode).unwrap();

    // 4 scenarios + 4 fixed entries = 8 items, last valid index is 7 (Quit)
    set_selected_menu_item(&mut state, 7);

    // Movement filter shrinks the list to 2 scenarios: 2 + 4 = 6 items, last index 5
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();

    assert_eq!(state.game.scenario_collection.count(), 2);
    assert_eq!(selected_menu_item(&state), 5);
}

#[test]
fn test_toggle_difficulty_filter_clamps_selection_when_list_shrinks() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);
    update(&mut state, Message::SelectTrainingMode).unwrap();

    set_selected_menu_item(&mut state, 7);

    // Beginner filter shrinks the list to 2 scenarios (both movement_*): last index 5
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();

    assert_eq!(state.game.scenario_collection.count(), 2);
    assert_eq!(selected_menu_item(&state), 5);
}

#[test]
fn test_toggle_completed_filter_clamps_selection_to_fixed_entries_when_list_becomes_empty() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    update(&mut state, Message::SelectTrainingMode).unwrap();

    // 1 scenario + 4 fixed entries = 5 items, last valid index is 4 (Quit)
    set_selected_menu_item(&mut state, 4);

    // "Completed only" filter hides the one (uncompleted) scenario entirely
    update(&mut state, Message::ToggleCompletedFilter).unwrap();

    assert_eq!(state.game.scenario_collection.count(), 0);
    // The 4 fixed entries remain selectable even with an empty scenario list
    assert_eq!(selected_menu_item(&state), 3);
}

#[test]
fn test_toggle_category_filter_clamps_selection_across_sequential_toggles_to_empty() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);
    update(&mut state, Message::SelectTrainingMode).unwrap();

    set_selected_menu_item(&mut state, 7);

    // Selection category alone leaves 1 scenario: 1 + 4 = 5 items, last index 4
    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Selection),
    )
    .unwrap();
    assert_eq!(state.game.scenario_collection.count(), 1);
    assert_eq!(selected_menu_item(&state), 4);

    // Combined with Beginner, Selection+Beginner has no matches: 0 + 4 = 4 items, last index 3
    update(
        &mut state,
        Message::ToggleDifficultyFilter(Difficulty::Beginner),
    )
    .unwrap();
    assert_eq!(state.game.scenario_collection.count(), 0);
    assert_eq!(selected_menu_item(&state), 3);
}

#[test]
fn test_toggle_filter_does_not_change_selection_when_already_in_bounds() {
    let scenarios = create_diverse_scenarios();
    let mut state = create_test_app_state(scenarios);
    update(&mut state, Message::SelectTrainingMode).unwrap();

    // Selection stays at 0, well within bounds of the shrunk list too
    set_selected_menu_item(&mut state, 0);

    update(
        &mut state,
        Message::ToggleCategoryFilter(ScenarioCategory::Movement),
    )
    .unwrap();

    assert_eq!(state.game.scenario_collection.count(), 2);
    assert_eq!(selected_menu_item(&state), 0);
}
