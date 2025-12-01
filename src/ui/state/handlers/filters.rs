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
    _ctx: &mut HandlerContext<'_>,
    _category: ScenarioCategory,
) -> Result<HandlerOutcome, UserError> {
    // TODO: Implement category filtering in future iteration
    // For now, this is a placeholder
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleDifficultyFilter message
///
/// Toggles difficulty filter (add/remove from active filters)
pub fn handle_toggle_difficulty_filter(
    _ctx: &mut HandlerContext<'_>,
    _difficulty: Difficulty,
) -> Result<HandlerOutcome, UserError> {
    // TODO: Implement difficulty filtering in future iteration
    // For now, this is a placeholder
    Ok(HandlerOutcome::Stay)
}

/// Handle ToggleCompletedFilter message
///
/// Toggles completed scenarios filter
pub fn handle_toggle_completed_filter(
    _ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // TODO: Implement completion filtering in future iteration
    // For now, this is a placeholder
    Ok(HandlerOutcome::Stay)
}

/// Handle ResetFilters message
///
/// Resets all filters to default
pub fn handle_reset_filters(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    ctx.game.scenario_collection.reset_filter();
    Ok(HandlerOutcome::Stay)
}
