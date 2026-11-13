//! Category filters screen message handlers
//!
//! Handles navigation and toggle operations for the category filters screen

use crate::game::Criterion;
use crate::ui::state::{CategoryFiltersData, HandlerContext, HandlerOutcome, TypedScreen};

/// Handle `ShowCategoryFilters` message
///
/// Navigates to the category filters screen, tracking where to return
pub(crate) fn handle_show_category_filters(screen: &mut TypedScreen) -> HandlerOutcome {
    let return_to = screen.detour_return_destination();
    HandlerOutcome::Transition(Box::new(TypedScreen::CategoryFilters(
        CategoryFiltersData {
            selected_index: 0,
            return_to,
        },
    )))
}

/// Handle `CategoryFilterUp` message
///
/// Moves selection up in the category list with wraparound
pub(crate) fn handle_category_filter_up(
    data: &mut CategoryFiltersData,
    ctx: &HandlerContext<'_>,
) -> HandlerOutcome {
    let category_count = ctx.game.scenario_collection.categories().len();

    if category_count == 0 {
        return HandlerOutcome::Stay;
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

    HandlerOutcome::Stay
}

/// Handle `CategoryFilterDown` message
///
/// Moves selection down in the category list with wraparound
pub(crate) fn handle_category_filter_down(
    data: &mut CategoryFiltersData,
    ctx: &HandlerContext<'_>,
) -> HandlerOutcome {
    let category_count = ctx.game.scenario_collection.categories().len();

    if category_count == 0 {
        return HandlerOutcome::Stay;
    }

    // Bounds check: ensure selected_index is valid
    if data.selected_index >= category_count {
        data.selected_index = 0;
        return HandlerOutcome::Stay;
    }

    // Move down with wraparound
    data.selected_index = (data.selected_index + 1) % category_count;

    HandlerOutcome::Stay
}

/// Handle `CategoryFilterToggle` message
///
/// Toggles the selected category filter on/off
pub(crate) fn handle_category_filter_toggle(
    data: &CategoryFiltersData,
    ctx: &mut HandlerContext<'_>,
) -> HandlerOutcome {
    let categories = ctx.game.scenario_collection.categories();

    // Bounds check: ensure selected_index is valid
    let Some(category) = categories.get(data.selected_index) else {
        return HandlerOutcome::Stay;
    };

    // Toggle this category using the existing filter infrastructure
    let profile = &ctx.progress.profile;
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    // Toggling the last enabled category off shows all categories again.
    new_filter.categories = match new_filter.categories {
        Criterion::Any => Criterion::one_of(*category, []),
        Criterion::OneOf(enabled) if enabled.contains(category) => enabled
            .without(category)
            .map_or(Criterion::Any, Criterion::OneOf),
        Criterion::OneOf(enabled) => Criterion::OneOf(enabled.with(*category)),
    };

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(profile));

    HandlerOutcome::Stay
}

/// Handle `CategoryFilterSelectAll` message
///
/// Resets category filters to show all categories (clears category filter)
pub(crate) fn handle_category_filter_select_all(ctx: &mut HandlerContext<'_>) -> HandlerOutcome {
    let profile = &ctx.progress.profile;
    let current_filter = ctx.game.scenario_collection.active_filter();
    let mut new_filter = current_filter.clone();

    // Clear category filter to show all
    new_filter.categories = Criterion::Any;

    ctx.game
        .scenario_collection
        .apply_filter(&new_filter, Some(profile));

    HandlerOutcome::Stay
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario, ScenarioCategory};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::ReturnDestination;
    use crate::ui::state::handlers::mode_selection::handle_select_arcade_mode;
    use crate::ui::state::{
        AppState, ConfigState, GameState, MenuData, ModeSelectionData, ProgressState, UIState,
    };
    use std::assert_matches;

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
        let outcome = handle_select_arcade_mode(&mut ctx);
        crate::ui::state::apply_outcome(state, outcome);
    }

    #[test]
    fn test_show_category_filters_from_menu() {
        let mut state = create_test_state_with_categories();
        state.screen = TypedScreen::Menu(MenuData::default());

        let outcome = handle_show_category_filters(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::CategoryFilters(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::Menu);
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

        if let Some(session) = state.minigame_session_mut() {
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            session.pause();
        }

        let outcome = handle_show_category_filters(&mut state.screen);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(boxed) = outcome {
            if let TypedScreen::CategoryFilters(data) = *boxed {
                assert_matches!(data.return_to, ReturnDestination::PausedMiniGame(_));
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
        let outcome = handle_category_filter_up(&mut data, &ctx);

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
        let outcome = handle_category_filter_up(&mut data, &ctx);

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
        let outcome = handle_category_filter_up(&mut data, &ctx);

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
        let outcome = handle_category_filter_up(&mut data, &ctx);

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
        let outcome = handle_category_filter_down(&mut data, &ctx);

        assert!(outcome.is_stay());
        assert_eq!(data.selected_index, 1);
    }

    #[test]
    fn test_category_filter_down_wraparound() {
        let mut state = create_test_state_with_categories();
        let category_count = state.game.scenario_collection.categories().len();
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
        let outcome = handle_category_filter_down(&mut data, &ctx);

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
        let outcome = handle_category_filter_down(&mut data, &ctx);

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
        let outcome = handle_category_filter_down(&mut data, &ctx);

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
                .is_any()
        );

        let outcome = handle_category_filter_toggle(&data, &mut ctx);
        assert!(outcome.is_stay());

        // Category filter should now be set
        let filter = ctx.game.scenario_collection.active_filter();
        assert!(filter.categories.is_restricted());
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
        handle_category_filter_toggle(&data, &mut ctx);

        // Toggle again to disable
        let outcome = handle_category_filter_toggle(&data, &mut ctx);
        assert!(outcome.is_stay());

        // Category filter should be cleared
        let filter = ctx.game.scenario_collection.active_filter();
        assert!(filter.categories.is_any());
    }

    /// Regression test for #312/S1: `handle_category_filter_toggle` runs while
    /// `state.screen == CategoryFilters`, not `Menu`, so it has no way to touch
    /// `MenuData::selected_item` itself even though it can shrink the filtered list. The
    /// read-only clamp in `render_main_menu` keeps such a stale `selected_item` from breaking
    /// the frame, and `AppState::normalize_view` (run after every `update`) repairs the stored
    /// value (#529); the same paths guard the `ui.last_menu_selected` restore paths in
    /// `src/ui/state/handlers/scenario.rs`.
    #[test]
    fn test_category_filter_toggle_live_path_leaves_stale_selection_for_normalization() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut state = create_test_state_with_categories(); // 3 scenarios, one per category
        state.screen = TypedScreen::Menu(MenuData {
            selected_item: 6, // last valid index: 3 scenarios + 4 fixed entries - 1
            ..MenuData::default()
        });

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
        // The real production handler behind the live CategoryFilterToggle message.
        handle_category_filter_toggle(&data, &mut ctx);

        let filtered_count = state.game.scenario_collection.count();
        assert_eq!(
            filtered_count, 1,
            "toggling one of three single-scenario categories should shrink to 1"
        );

        // state.screen is still Menu with the stale selected_item=6 here — nothing about
        // handle_category_filter_toggle touches it. Rendering must survive it read-only,
        // and normalization repairs it.
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| crate::ui::render::render(f, &state))
            .unwrap();

        let TypedScreen::Menu(menu_data) = &state.screen else {
            panic!("expected TypedScreen::Menu");
        };
        assert_eq!(
            menu_data.selected_item, 6,
            "render must not mutate the state"
        );

        state.normalize_view();

        let TypedScreen::Menu(menu_data) = &state.screen else {
            panic!("expected TypedScreen::Menu");
        };
        assert_eq!(menu_data.selected_item, 4); // filtered_count(1) + FIXED_MENU_ITEMS(4) - 1
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

        let outcome = handle_category_filter_toggle(&data, &mut ctx);
        assert!(outcome.is_stay());

        // No change should have occurred
        assert!(
            ctx.game
                .scenario_collection
                .active_filter()
                .categories
                .is_any()
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
        handle_category_filter_toggle(&data, &mut ctx);
        assert!(
            ctx.game
                .scenario_collection
                .active_filter()
                .categories
                .is_restricted()
        );

        // Select all should clear the filter
        let outcome = handle_category_filter_select_all(&mut ctx);
        assert!(outcome.is_stay());

        // Category filter should be cleared
        let filter = ctx.game.scenario_collection.active_filter();
        assert!(filter.categories.is_any());
    }

    #[test]
    fn test_category_filter_select_all_preserves_other_filters() {
        let mut state = create_test_state_with_categories();

        // Set up a difficulty filter first
        {
            let ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            let mut filter = ctx.game.scenario_collection.active_filter().clone();
            filter.difficulties = Criterion::one_of(Difficulty::Beginner, []);
            ctx.game
                .scenario_collection
                .apply_filter(&filter, Some(&ctx.progress.profile));
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
            handle_category_filter_toggle(&data, &mut ctx);
        }

        // Select all should clear only category filter
        {
            let mut ctx = HandlerContext::new(
                &mut state.ui,
                &mut state.game,
                &mut state.progress,
                &state.config,
            );
            handle_category_filter_select_all(&mut ctx);

            let filter = ctx.game.scenario_collection.active_filter();
            assert!(filter.categories.is_any());
            // Difficulty filter should still be present
            assert!(filter.difficulties.is_restricted());
        }
    }
}
