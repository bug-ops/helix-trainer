//! Menu interaction message handlers
//!
//! Handles menu navigation and selection
//!
//! Type-safe handlers that receive MenuData directly instead of
//! performing runtime checks on AppState.

use crate::security::UserError;
use crate::ui::state::{AppState, HandlerContext, HandlerOutcome, MenuData};

/// Number of fixed menu entries appended after the scenario list: Review, Profile, Statistics, Quit.
const FIXED_MENU_ITEMS: usize = 4;

/// Total number of menu items for a given filtered scenario count: scenarios plus the fixed trailing entries.
pub(crate) fn total_menu_items_for_count(scenario_count: usize) -> usize {
    scenario_count + FIXED_MENU_ITEMS
}

/// Total number of menu items: filtered scenarios plus the fixed trailing entries.
fn total_menu_items(ctx: &HandlerContext<'_>) -> usize {
    total_menu_items_for_count(ctx.game.scenario_collection.count())
}

/// Handle MenuUp message
///
/// Moves menu selection up (with bounds checking).
/// Type-safe handler that only accepts MenuData.
pub fn handle_menu_up(data: &mut MenuData) -> Result<HandlerOutcome, UserError> {
    if data.selected_item > 0 {
        data.selected_item -= 1;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuDown message
///
/// Moves menu selection down (with bounds checking).
/// Type-safe handler that only accepts MenuData and HandlerContext for scenario count.
pub fn handle_menu_down(
    data: &mut MenuData,
    ctx: &HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let max_items = total_menu_items(ctx);
    if data.selected_item < max_items - 1 {
        data.selected_item += 1;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuUpBy message
///
/// Moves menu selection up by N items (with bounds checking).
pub fn handle_menu_up_by(data: &mut MenuData, count: usize) -> Result<HandlerOutcome, UserError> {
    data.selected_item = data.selected_item.saturating_sub(count);
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuDownBy message
///
/// Moves menu selection down by N items (with bounds checking).
pub fn handle_menu_down_by(
    data: &mut MenuData,
    ctx: &HandlerContext<'_>,
    count: usize,
) -> Result<HandlerOutcome, UserError> {
    let max_items = total_menu_items(ctx);
    let max_index = max_items.saturating_sub(1);
    data.selected_item = (data.selected_item + count).min(max_index);
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuJumpToFirst message (gg)
///
/// Jumps to first menu item.
pub fn handle_menu_jump_to_first(data: &mut MenuData) -> Result<HandlerOutcome, UserError> {
    data.selected_item = 0;
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuJumpToLast message (G)
///
/// Jumps to last menu item.
pub fn handle_menu_jump_to_last(
    data: &mut MenuData,
    ctx: &HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let max_items = total_menu_items(ctx);
    data.selected_item = max_items.saturating_sub(1);
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuJumpTo message ({n}G or {n}gg)
///
/// Jumps to specific menu item (1-indexed like Helix line numbers).
pub fn handle_menu_jump_to(
    data: &mut MenuData,
    ctx: &HandlerContext<'_>,
    line: usize,
) -> Result<HandlerOutcome, UserError> {
    let max_items = total_menu_items(ctx);
    // Convert 1-indexed to 0-indexed, clamping to valid range
    let target = line.saturating_sub(1).min(max_items.saturating_sub(1));
    data.selected_item = target;
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuSelect message
///
/// Executes action based on currently selected menu item.
/// Uses HandlerContext to delegate to other handlers and perform transitions.
#[allow(dead_code)] // Used directly in update() function, not through handlers::
pub fn handle_menu_select(
    data: &MenuData,
    state: &mut AppState,
) -> Result<HandlerOutcome, UserError> {
    let selected = data.selected_item;
    let scenario_count = state.game.scenario_collection.count();

    if selected < scenario_count {
        // Start selected scenario (0..scenario_count-1)
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        super::scenario::handle_start_scenario(&mut ctx, selected)
    } else if selected == scenario_count {
        // Review Commands (index = scenario_count)
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        super::review::handle_start_review_session(&mut ctx)
    } else if selected == scenario_count + 1 {
        // View Profile (index = scenario_count + 1)
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        super::profile::handle_show_profile(&mut ctx)
    } else if selected == scenario_count + 2 {
        // Statistics (index = scenario_count + 2)
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        super::profile::handle_show_statistics(&mut ctx)
    } else if selected == scenario_count + 3 {
        // Quit (index = scenario_count + 3). Routes through `handle_quit_app`
        // (not a direct `state.ui.running = false`) so a live paused arcade
        // session reachable from the menu still gets its game-over
        // bookkeeping awarded - see #324.
        super::handle_quit_app(state)
    } else {
        Ok(HandlerOutcome::Stay)
    }
}
