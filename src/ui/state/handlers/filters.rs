//! Filtering and sorting message handlers
//!
//! Handles scenario list filtering and sorting

use std::collections::HashSet;
use std::hash::Hash;

use crate::config::{Difficulty, ScenarioCategory, SortMode};
use crate::security::UserError;
use crate::ui::state::{HandlerContext, HandlerOutcome};

/// Toggles `value` in an optional filter set, clearing the set (setting it to `None`)
/// when the last member is removed.
fn toggle_filter_set<T: Eq + Hash>(filter: &mut Option<HashSet<T>>, value: T) {
    let set = filter.get_or_insert_default();
    if !set.remove(&value) {
        set.insert(value);
    }
    if set.is_empty() {
        *filter = None;
    }
}

/// Handle SetSortMode message
///
/// Changes the sort order of scenarios
pub fn handle_set_sort_mode(
    ctx: &mut HandlerContext<'_>,
    mode: SortMode,
) -> Result<HandlerOutcome, UserError> {
    let profile = &ctx.progress.profile;
    ctx.game.scenario_collection.sort(mode, Some(profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleCategoryFilter message
///
/// Toggles category filter (add/remove from active filters)
pub fn handle_toggle_category_filter(
    ctx: &mut HandlerContext<'_>,
    category: ScenarioCategory,
) -> Result<HandlerOutcome, UserError> {
    let profile = &ctx.progress.profile;
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    toggle_filter_set(&mut new_filter.categories, category);

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleDifficultyFilter message
///
/// Toggles difficulty filter (add/remove from active filters)
pub fn handle_toggle_difficulty_filter(
    ctx: &mut HandlerContext<'_>,
    difficulty: Difficulty,
) -> Result<HandlerOutcome, UserError> {
    let profile = &ctx.progress.profile;
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    toggle_filter_set(&mut new_filter.difficulties, difficulty);

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(profile));
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
    let profile = &ctx.progress.profile;
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
        .apply_filter(&new_filter, Some(profile));
    Ok(HandlerOutcome::Stay)
}

/// Handle ResetFilters message
///
/// Resets all filters to default
pub fn handle_reset_filters(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    ctx.game.scenario_collection.reset_filter();
    Ok(HandlerOutcome::Stay)
}
