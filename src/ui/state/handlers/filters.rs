//! Filtering and sorting message handlers
//!
//! Handles scenario list filtering and sorting

use crate::config::{Difficulty, ScenarioCategory, SortMode};
use crate::security::UserError;
use crate::ui::state::{HandlerContext, HandlerOutcome};

/// Handle SetSortMode message
///
/// Changes the sort order of scenarios
pub fn handle_set_sort_mode(
    ctx: &mut HandlerContext<'_>,
    mode: SortMode,
) -> Result<HandlerOutcome, UserError> {
    let profile = ctx.progress.profile.borrow();
    ctx.game.scenario_collection.sort(mode, Some(&profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleCategoryFilter message
///
/// Toggles category filter (add/remove from active filters)
pub fn handle_toggle_category_filter(
    ctx: &mut HandlerContext<'_>,
    category: ScenarioCategory,
) -> Result<HandlerOutcome, UserError> {
    let profile = ctx.progress.profile.borrow();
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    // Toggle category in the filter set
    let categories = new_filter.categories.get_or_insert_with(Default::default);
    if categories.contains(&category) {
        categories.remove(&category);
        // If no categories left, clear the filter
        if categories.is_empty() {
            new_filter.categories = None;
        }
    } else {
        categories.insert(category);
    }

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(&profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleDifficultyFilter message
///
/// Toggles difficulty filter (add/remove from active filters)
pub fn handle_toggle_difficulty_filter(
    ctx: &mut HandlerContext<'_>,
    difficulty: Difficulty,
) -> Result<HandlerOutcome, UserError> {
    let profile = ctx.progress.profile.borrow();
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    // Toggle difficulty in the filter set
    let difficulties = new_filter.difficulties.get_or_insert_with(Default::default);
    if difficulties.contains(&difficulty) {
        difficulties.remove(&difficulty);
        // If no difficulties left, clear the filter
        if difficulties.is_empty() {
            new_filter.difficulties = None;
        }
    } else {
        difficulties.insert(difficulty);
    }

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(&profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleCompletedFilter message
///
/// Toggles completed scenarios filter
///
/// Cycles through three states: Show All -> Show Only Completed -> Show Only Not Completed -> Show All
pub fn handle_toggle_completed_filter(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let profile = ctx.progress.profile.borrow();
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    // Cycle through completion filter states
    match (new_filter.completed_only, new_filter.not_completed_only) {
        (false, false) => {
            // Show all -> Show only completed
            new_filter.completed_only = true;
            new_filter.not_completed_only = false;
        }
        (true, false) => {
            // Show only completed -> Show only not completed
            new_filter.completed_only = false;
            new_filter.not_completed_only = true;
        }
        (false, true) => {
            // Show only not completed -> Show all
            new_filter.completed_only = false;
            new_filter.not_completed_only = false;
        }
        (true, true) => {
            // Invalid state, reset to show all
            new_filter.completed_only = false;
            new_filter.not_completed_only = false;
        }
    }

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(&profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ResetFilters message
///
/// Resets all filters to default
pub fn handle_reset_filters(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    ctx.game.scenario_collection.reset_filter();
    Ok(HandlerOutcome::Stay)
}
