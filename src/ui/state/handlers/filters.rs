//! Filtering and sorting message handlers
//!
//! Handles scenario list filtering and sorting

use crate::config::{Difficulty, ScenarioCategory, SortMode};
use crate::security::UserError;
use crate::ui::state::AppState;

/// Handle SetSortMode message
///
/// Changes the sort order of scenarios
pub fn handle_set_sort_mode(state: &mut AppState, mode: SortMode) -> Result<(), UserError> {
    let profile = state.profile.borrow();
    state.scenario_collection.sort(mode, Some(&profile));
    Ok(())
}

/// Handle ToggleCategoryFilter message
///
/// Toggles category filter (add/remove from active filters)
pub fn handle_toggle_category_filter(
    _state: &mut AppState,
    _category: ScenarioCategory,
) -> Result<(), UserError> {
    // TODO: Implement category filtering in future iteration
    // For now, this is a placeholder
    Ok(())
}

/// Handle ToggleDifficultyFilter message
///
/// Toggles difficulty filter (add/remove from active filters)
pub fn handle_toggle_difficulty_filter(
    _state: &mut AppState,
    _difficulty: Difficulty,
) -> Result<(), UserError> {
    // TODO: Implement difficulty filtering in future iteration
    // For now, this is a placeholder
    Ok(())
}

/// Handle ToggleCompletedFilter message
///
/// Toggles completed scenarios filter
pub fn handle_toggle_completed_filter(_state: &mut AppState) -> Result<(), UserError> {
    // TODO: Implement completion filtering in future iteration
    // For now, this is a placeholder
    Ok(())
}

/// Handle ResetFilters message
///
/// Resets all filters to default
pub fn handle_reset_filters(state: &mut AppState) -> Result<(), UserError> {
    state.scenario_collection.reset_filter();
    Ok(())
}
