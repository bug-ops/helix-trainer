//! Menu interaction message handlers
//!
//! Handles menu navigation and selection
//!
//! Type-safe handlers that receive MenuData directly instead of
//! performing runtime checks on AppState.

use crate::security::UserError;
use crate::ui::state::{AppState, HandlerContext, HandlerOutcome, MenuData};

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
    // Total menu items = filtered scenarios + Review + Profile + Statistics + Quit
    let max_items = ctx.game.scenario_collection.count() + 4;
    if data.selected_item < max_items - 1 {
        data.selected_item += 1;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle MenuSelect message
///
/// Executes action based on currently selected menu item.
/// Uses HandlerContext to delegate to other handlers and perform transitions.
///
/// TODO: Refactor scenario, review, and profile handlers to use HandlerContext
/// For now, we need to work with the old AppState-based handlers
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
        // Quit (index = scenario_count + 3)
        state.ui.running = false;
        Ok(HandlerOutcome::Stay)
    } else {
        Ok(HandlerOutcome::Stay)
    }
}
