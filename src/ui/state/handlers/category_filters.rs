//! Category filters screen message handlers
//!
//! Handles navigation and toggle operations for the category filters screen

use crate::security::UserError;
use crate::ui::state::{
    CategoryFiltersData, HandlerContext, HandlerOutcome, ReturnDestination, TypedScreen,
};

/// Determine return destination based on current context
///
/// If coming from a paused mini-game, return there; otherwise return to menu.
fn determine_return_destination(ctx: &HandlerContext<'_>) -> ReturnDestination {
    if let Some(session) = &ctx.game.minigame_session
        && session.state().is_paused()
    {
        return ReturnDestination::PausedMiniGame;
    }
    ReturnDestination::Menu
}

/// Handle ShowCategoryFilters message
///
/// Navigates to the category filters screen, tracking where to return
pub fn handle_show_category_filters(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let return_to = determine_return_destination(ctx);
    Ok(HandlerOutcome::Transition(Box::new(
        TypedScreen::CategoryFilters(CategoryFiltersData {
            selected_index: 0,
            return_to,
        }),
    )))
}

/// Handle CategoryFilterUp message
///
/// Moves selection up in the category list with wraparound
pub fn handle_category_filter_up(
    data: &mut CategoryFiltersData,
    ctx: &HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let category_count = ctx.game.scenario_collection.get_categories().len();

    if category_count == 0 {
        return Ok(HandlerOutcome::Stay);
    }

    // Bounds check: ensure selected_index is valid before decrement
    if data.selected_index >= category_count {
        data.selected_index = category_count.saturating_sub(1);
    }

    // Move up with wraparound
    if data.selected_index > 0 {
        data.selected_index -= 1;
    } else {
        data.selected_index = category_count.saturating_sub(1);
    }

    Ok(HandlerOutcome::Stay)
}

/// Handle CategoryFilterDown message
///
/// Moves selection down in the category list with wraparound
pub fn handle_category_filter_down(
    data: &mut CategoryFiltersData,
    ctx: &HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let category_count = ctx.game.scenario_collection.get_categories().len();

    if category_count == 0 {
        return Ok(HandlerOutcome::Stay);
    }

    // Bounds check: ensure selected_index is valid
    if data.selected_index >= category_count {
        data.selected_index = 0;
        return Ok(HandlerOutcome::Stay);
    }

    // Move down with wraparound
    data.selected_index = (data.selected_index + 1) % category_count;

    Ok(HandlerOutcome::Stay)
}

/// Handle CategoryFilterToggle message
///
/// Toggles the selected category filter on/off
pub fn handle_category_filter_toggle(
    data: &CategoryFiltersData,
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let categories = ctx.game.scenario_collection.get_categories();

    // Bounds check: ensure selected_index is valid
    let Some(category) = categories.get(data.selected_index) else {
        return Ok(HandlerOutcome::Stay);
    };

    // Toggle this category using the existing filter infrastructure
    let profile = &ctx.progress.profile;
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    let filter_categories = new_filter.categories.get_or_insert_with(Default::default);
    if filter_categories.contains(category) {
        filter_categories.remove(category);
        if filter_categories.is_empty() {
            new_filter.categories = None;
        }
    } else {
        filter_categories.insert(*category);
    }

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(profile));

    Ok(HandlerOutcome::Stay)
}

/// Handle CategoryFilterSelectAll message
///
/// Resets category filters to show all categories (clears category filter)
pub fn handle_category_filter_select_all(
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    let profile = &ctx.progress.profile;
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    // Clear category filter to show all
    new_filter.categories = None;

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(profile));

    Ok(HandlerOutcome::Stay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario, ScenarioCategory};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::handlers::minigame::handle_start_minigame;
    use crate::ui::state::{
        AppState, ConfigState, GameState, MenuData, ModeSelectionData, ProgressState, UIState,
    };

    fn create_test_scenario_with_category(id: &str, category: ScenarioCategory) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .difficulty(Difficulty::Beginner)
            .category(category)
            .build()
    }

    fn create_test_state_with_categories() -> AppState {
        let scenarios = vec![
            create_test_scenario_with_category("s1", ScenarioCategory::Movement),
            create_test_scenario_with_category("s2", ScenarioCategory::Editing),
            create_test_scenario_with_category("s3", ScenarioCategory::Selection),
        ];

        AppState {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        }
    }

    fn create_empty_state() -> AppState {
        AppState {
            screen: TypedScreen::ModeSelection(ModeSelectionData::default()),
            ui: UIState::new(),
            game: GameState::new(vec![]),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        }
    }

    fn start_minigame(state: &mut AppState) {
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_start_minigame(&mut ctx).unwrap();
        crate::ui::state::apply_outcome(state, outcome);
    }

    #[test]
    fn test_show_category_filters_from_menu() {
        let mut state = create_test_state_with_categories();
        state.screen = TypedScreen::Menu(MenuData::default());

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_category_filters(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::CategoryFilters(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::Menu);
                assert_eq!(data.selected_index, 0);
            } else {
                panic!("Expected CategoryFilters screen");
            }
        }
    }

    #[test]
    fn test_show_category_filters_from_paused_minigame() {
        let mut state = create_test_state_with_categories();

        start_minigame(&mut state);

        if let Some(ref mut session) = state.game.minigame_session {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.pause();
        }

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_show_category_filters(&mut ctx).unwrap();

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::CategoryFilters(data) = *boxed {
                assert_eq!(data.return_to, ReturnDestination::PausedMiniGame);
            } else {
                panic!("Expected CategoryFilters screen");
            }
        }
    }

    #[test]
    fn test_category_filter_up_basic() {
        let mut state = create_test_state_with_categories();
        let mut data = CategoryFiltersData {
            selected_index: 1,
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_up(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_index, 0);
    }

    #[test]
    fn test_category_filter_up_wraparound() {
        let mut state = create_test_state_with_categories();
        let mut data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_up(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        // Should wrap to last category (index 2, since we have 3 categories)
        assert_eq!(data.selected_index, 2);
    }

    #[test]
    fn test_category_filter_up_empty_collection() {
        let mut state = create_empty_state();
        let mut data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_up(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_index, 0);
    }

    #[test]
    fn test_category_filter_up_out_of_bounds_index() {
        let mut state = create_test_state_with_categories();
        let mut data = CategoryFiltersData {
            selected_index: 100, // Invalid index
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_up(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        // Should be clamped and then decremented
        assert!(data.selected_index < 100);
    }

    #[test]
    fn test_category_filter_down_basic() {
        let mut state = create_test_state_with_categories();
        let mut data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_down(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_index, 1);
    }

    #[test]
    fn test_category_filter_down_wraparound() {
        let mut state = create_test_state_with_categories();
        let category_count = state.game.scenario_collection.get_categories().len();
        let mut data = CategoryFiltersData {
            selected_index: category_count - 1,
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_down(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_index, 0);
    }

    #[test]
    fn test_category_filter_down_empty_collection() {
        let mut state = create_empty_state();
        let mut data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_down(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        assert_eq!(data.selected_index, 0);
    }

    #[test]
    fn test_category_filter_down_out_of_bounds_index() {
        let mut state = create_test_state_with_categories();
        let mut data = CategoryFiltersData {
            selected_index: 100, // Invalid index
            return_to: ReturnDestination::Menu,
        };

        let ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );
        let outcome = handle_category_filter_down(&mut data, &ctx).unwrap();

        assert!(outcome.is_stay());
        // Should be reset to 0
        assert_eq!(data.selected_index, 0);
    }

    #[test]
    fn test_category_filter_toggle_enables() {
        let mut state = create_test_state_with_categories();
        let data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Verify no filter initially
        assert!(
            ctx.game
                .scenario_collection
                .active_filter()
                .categories
                .is_none()
        );

        let outcome = handle_category_filter_toggle(&data, &mut ctx).unwrap();
        assert!(outcome.is_stay());

        // Category filter should now be set
        let filter = ctx.game.scenario_collection.active_filter();
        assert!(filter.categories.is_some());
    }

    #[test]
    fn test_category_filter_toggle_disables() {
        let mut state = create_test_state_with_categories();
        let data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Enable filter first
        handle_category_filter_toggle(&data, &mut ctx).unwrap();

        // Toggle again to disable
        let outcome = handle_category_filter_toggle(&data, &mut ctx).unwrap();
        assert!(outcome.is_stay());

        // Category filter should be cleared
        let filter = ctx.game.scenario_collection.active_filter();
        assert!(filter.categories.is_none());
    }

    #[test]
    fn test_category_filter_toggle_out_of_bounds() {
        let mut state = create_test_state_with_categories();
        let data = CategoryFiltersData {
            selected_index: 100, // Invalid index
            return_to: ReturnDestination::Menu,
        };

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_category_filter_toggle(&data, &mut ctx).unwrap();
        assert!(outcome.is_stay());

        // No change should have occurred
        assert!(
            ctx.game
                .scenario_collection
                .active_filter()
                .categories
                .is_none()
        );
    }

    #[test]
    fn test_category_filter_select_all() {
        let mut state = create_test_state_with_categories();
        let data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Enable a filter first
        handle_category_filter_toggle(&data, &mut ctx).unwrap();
        assert!(
            ctx.game
                .scenario_collection
                .active_filter()
                .categories
                .is_some()
        );

        // Select all should clear the filter
        let outcome = handle_category_filter_select_all(&mut ctx).unwrap();
        assert!(outcome.is_stay());

        // Category filter should be cleared
        let filter = ctx.game.scenario_collection.active_filter();
        assert!(filter.categories.is_none());
    }

    #[test]
    fn test_category_filter_select_all_preserves_other_filters() {
        let mut state = create_test_state_with_categories();

        // Set up a difficulty filter first
        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            crate::ui::state::handlers::handle_toggle_difficulty_filter(
                &mut ctx,
                Difficulty::Beginner,
            )
            .unwrap();
        }

        // Enable a category filter
        let data = CategoryFiltersData {
            selected_index: 0,
            return_to: ReturnDestination::Menu,
        };
        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handle_category_filter_toggle(&data, &mut ctx).unwrap();
        }

        // Select all should clear only category filter
        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handle_category_filter_select_all(&mut ctx).unwrap();

            let filter = ctx.game.scenario_collection.active_filter();
            assert!(filter.categories.is_none());
            // Difficulty filter should still be present
            assert!(filter.difficulties.is_some());
        }
    }
}
