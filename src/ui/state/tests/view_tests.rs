//! Tests for view normalization: menu and achievements clamping against list contents and the
//! viewport (FR-022, FR-023)

use super::common::create_test_scenario;
use crate::config::Scenario;
use crate::game::GameSession;
use crate::testing::update_executing_effects as update;
use crate::testing::{ScenarioBuilder, test_app_state_with_scenarios};
use crate::ui::layout::{
    FIXED_MENU_ITEMS, Viewport, achievement_list_rows, menu_list_rows, selection_to_row,
};
use crate::ui::state::{AchievementsData, GameState, MenuData, Message, ResultsData, TypedScreen};
use crate::ui::{AppState, render};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn scenarios(count: usize) -> Vec<Scenario> {
    (0..count)
        .map(|i| {
            ScenarioBuilder::new()
                .id(format!("scenario_{i}"))
                .setup_content("line 1\n")
                .target_content("line 2\n")
                .optimal_count(1)
                .build()
        })
        .collect()
}

fn menu_state(scenario_count: usize) -> AppState {
    let mut state = test_app_state_with_scenarios(scenarios(scenario_count));
    state.screen = TypedScreen::Menu(MenuData::default());
    state
}

fn menu_view(state: &AppState) -> (usize, usize) {
    let TypedScreen::Menu(data) = &state.screen else {
        panic!(
            "expected the menu screen, got {}",
            state.screen.screen_type()
        );
    };
    (data.selected_item, data.scroll_offset)
}

/// The selected row must lie inside the window `[offset, offset + rows)` of the viewport
fn assert_selection_visible(state: &AppState, scenario_count: usize) {
    let (selected, offset) = menu_view(state);
    let rows = menu_list_rows(state.ui.viewport);
    assert!(
        selected < scenario_count + FIXED_MENU_ITEMS,
        "selection {selected} past the list of {scenario_count} scenarios"
    );
    if rows > 0 {
        let row = selection_to_row(selected, scenario_count);
        assert!(
            offset <= row && row < offset + rows,
            "row {row} outside [{offset}, {}) at {rows} visible rows",
            offset + rows
        );
    }
}

fn results_without_index(state: &mut AppState) {
    let abandoned = GameSession::new(create_test_scenario()).unwrap().abandon();
    let feedback = abandoned.feedback();
    state.screen = TypedScreen::Results(ResultsData::from_abandoned(abandoned, feedback, None));
}

fn achievements_offset(state: &AppState) -> usize {
    let TypedScreen::Achievements(data) = &state.screen else {
        panic!("expected the achievements screen");
    };
    data.scroll_offset
}

#[test]
fn test_resize_smaller_keeps_selection_visible() {
    let mut state = menu_state(30);
    update(&mut state, Message::Resize(Viewport::new(100, 60))).unwrap();
    update(&mut state, Message::MenuJumpToLast).unwrap();
    assert_selection_visible(&state, 30);

    update(&mut state, Message::Resize(Viewport::new(80, 24))).unwrap();

    assert_selection_visible(&state, 30);
}

#[test]
fn test_every_viewport_height_keeps_selection_visible() {
    for rows in [0u16, 1, 5, 12, 17, 24, 40, 60] {
        let mut state = menu_state(30);
        update(&mut state, Message::MenuJumpToLast).unwrap();
        update(&mut state, Message::Resize(Viewport::new(80, rows))).unwrap();
        assert_selection_visible(&state, 30);
    }
}

#[test]
fn test_zero_height_viewport_neither_panics_nor_invalidates_selection() {
    let mut state = menu_state(5);
    update(&mut state, Message::MenuJumpToLast).unwrap();

    update(&mut state, Message::Resize(Viewport::new(0, 0))).unwrap();

    let (selected, _) = menu_view(&state);
    assert_eq!(selected, 5 + FIXED_MENU_ITEMS - 1);
}

#[test]
fn test_list_shrink_pulls_selection_back_after_any_update() {
    let mut state = menu_state(30);
    update(&mut state, Message::MenuJumpToLast).unwrap();
    // A filter change or a reload leaves fewer scenarios than the stored selection assumed
    state.game = GameState::new(scenarios(3));

    update(&mut state, Message::CleanupNotifications).unwrap();

    assert_selection_visible(&state, 3);
}

#[test]
fn test_normalize_view_covers_a_background_data_change() {
    let mut state = menu_state(30);
    update(&mut state, Message::MenuJumpToLast).unwrap();
    state.game = GameState::new(scenarios(2));

    state.normalize_view();

    assert_selection_visible(&state, 2);
}

#[test]
fn test_rebuild_from_remembered_selection_is_normalized() {
    let mut state = menu_state(3);
    state.ui.last_menu_selected = 40;
    state.ui.last_menu_scroll = 40;

    results_without_index(&mut state);
    update(&mut state, Message::NextLesson).unwrap();
    assert_selection_visible(&state, 3);

    results_without_index(&mut state);
    update(&mut state, Message::GoToScenarioList).unwrap();
    assert_selection_visible(&state, 3);

    state.screen = TypedScreen::Menu(MenuData::default());
    update(&mut state, Message::CompleteScenario).unwrap();
    assert_selection_visible(&state, 3);
}

#[test]
fn test_achievements_scroll_is_clamped_by_resize_and_scroll() {
    let mut state = menu_state(0);
    state.screen = TypedScreen::Achievements(AchievementsData {
        scroll_offset: 10_000,
        ..AchievementsData::default()
    });
    let total = crate::gamification::AchievementEngine::all_achievements().len();
    let limit = total.saturating_sub(achievement_list_rows(state.ui.viewport));

    update(&mut state, Message::Resize(Viewport::new(80, 24))).unwrap();
    assert_eq!(achievements_offset(&state), limit);

    update(&mut state, Message::ScrollAchievements(i32::MAX)).unwrap();
    assert_eq!(achievements_offset(&state), limit);
}

#[test]
fn test_screens_without_a_list_are_untouched_by_resize() {
    let mut state = test_app_state_with_scenarios(vec![]);
    let before = state.screen.screen_type();

    update(&mut state, Message::Resize(Viewport::new(10, 5))).unwrap();

    assert_eq!(state.screen.screen_type(), before);
    assert_eq!(state.ui.viewport, Viewport::new(10, 5));
}

#[test]
fn test_render_leaves_the_state_as_normalized_at_every_height() {
    for rows in [8u16, 12, 24, 40] {
        let mut state = menu_state(30);
        update(&mut state, Message::Resize(Viewport::new(80, rows))).unwrap();
        update(&mut state, Message::MenuJumpToLast).unwrap();
        let view_before = menu_view(&state);
        let mut terminal = Terminal::new(TestBackend::new(80, rows)).unwrap();

        terminal.draw(|f| render(f, &state)).unwrap();

        assert_eq!(menu_view(&state), view_before);
        if rows >= 24 {
            let text: String = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(text.contains("Quit"), "Quit hidden at {rows} rows");
        }
    }
}
