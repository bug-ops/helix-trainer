//! Category filters screen rendering

use crate::config::ScenarioCategory;
use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

/// Get a human-readable display name for a category
///
/// `pub(super)` so `end_game.rs` can reuse it for the category-mastery
/// breakdown instead of duplicating the match.
pub(super) fn category_display_name(category: &ScenarioCategory) -> &'static str {
    match category {
        ScenarioCategory::Movement => "Movement",
        ScenarioCategory::Editing => "Editing",
        ScenarioCategory::Clipboard => "Clipboard",
        ScenarioCategory::Search => "Search",
        ScenarioCategory::Selection => "Selection",
        ScenarioCategory::TextObjects => "Text Objects",
        ScenarioCategory::Advanced => "Advanced",
        ScenarioCategory::Registers => "Registers",
        ScenarioCategory::Multi => "Multi",
        ScenarioCategory::Other => "Other",
    }
}

/// Get an icon for a category
fn category_icon(category: &ScenarioCategory) -> &'static str {
    match category {
        ScenarioCategory::Movement => "->",
        ScenarioCategory::Editing => "ed",
        ScenarioCategory::Clipboard => "[]",
        ScenarioCategory::Search => "??",
        ScenarioCategory::Selection => "##",
        ScenarioCategory::TextObjects => "{}",
        ScenarioCategory::Advanced => "++",
        ScenarioCategory::Registers => "\"\"",
        ScenarioCategory::Multi => "**",
        ScenarioCategory::Other => "..",
    }
}

/// Render the category filters as a centered popup overlay
pub(super) fn render_category_filters(frame: &mut Frame, state: &AppState) {
    let TypedScreen::CategoryFilters(filters_data) = &state.screen else {
        return;
    };

    let area = frame.area();

    // Calculate popup dimensions based on content
    let categories = state.game.scenario_collection.get_categories();
    // Height: categories + 2 padding lines + 1 summary + 1 spacing + 1 instructions
    let content_height = categories.len() as u16 + 5;
    // Add 2 for popup borders
    let popup_height = (content_height + 2).min(20);
    let popup_width = 52u16;

    // Constrain popup dimensions to terminal size
    let popup_width = popup_width.min(area.width.saturating_sub(4));
    let popup_height = popup_height.min(area.height.saturating_sub(4));

    // Create centered popup area
    let popup_area = super::helpers::centered_popup(area, popup_width, popup_height);

    // Clear popup area with black background
    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(clear_block, popup_area);

    // Render popup block with title
    let block = super::helpers::popup_block(Some(" CATEGORY FILTERS "), Color::Cyan);
    frame.render_widget(&block, popup_area);

    // Get inner area for content
    let inner_area = super::helpers::inner_rect(popup_area);

    // Render content inside popup
    render_popup_content(frame, state, filters_data.selected_index, inner_area);
}

/// Render popup content: category list, summary, and instructions
fn render_popup_content(
    frame: &mut Frame,
    state: &AppState,
    selected_index: usize,
    area: ratatui::layout::Rect,
) {
    let categories = state.game.scenario_collection.get_categories();

    if categories.is_empty() {
        let empty_msg = Paragraph::new("No categories available")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, area);
        return;
    }

    // Get currently active category filter
    let active_filter = state.game.scenario_collection.active_filter();
    let enabled_categories = active_filter.categories.as_ref();

    let mut lines = vec![];
    lines.push(Line::from("")); // Top padding

    for (idx, category) in categories.iter().enumerate() {
        let is_selected = idx == selected_index;

        // Determine if this category is enabled:
        // - If no categories filter is set (None), all categories are enabled
        // - If categories filter is set, check if this category is in the set
        let is_enabled = match enabled_categories {
            None => true, // All enabled when no filter
            Some(set) => set.contains(category),
        };

        // Build the checkbox indicator
        let checkbox = if is_enabled { "[x]" } else { "[ ]" };

        // Build the line with highlighting for selected item
        let (prefix, base_style) = if is_selected {
            (
                " > ",
                Style::default()
                    .bg(super::SELECTION_BG_COLOR)
                    .fg(Color::White),
            )
        } else {
            ("   ", Style::default().fg(Color::White))
        };

        let icon = category_icon(category);
        let name = category_display_name(category);

        // Checkbox color: green if enabled, gray if disabled
        let checkbox_style = if is_enabled {
            base_style.fg(Color::Green)
        } else {
            base_style.fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, base_style),
            Span::styled(format!("{} ", checkbox), checkbox_style),
            Span::styled(format!("{} ", icon), base_style.fg(Color::Cyan)),
            Span::styled(name, base_style),
        ]));
    }

    // Add summary line
    lines.push(Line::from("")); // Spacing

    let enabled_count = match enabled_categories {
        None => categories.len(),
        Some(set) => set.len(),
    };

    let summary_text = if enabled_count == categories.len() {
        "All categories enabled".to_string()
    } else if enabled_count == 0 {
        "No categories enabled (showing all)".to_string()
    } else {
        format!(
            "{} of {} categories enabled",
            enabled_count,
            categories.len()
        )
    };

    lines.push(Line::from(vec![
        Span::raw("   "),
        Span::styled(summary_text, Style::default().fg(Color::Gray)),
    ]));

    // Add compact instructions at bottom
    lines.push(Line::from("")); // Spacing
    lines.push(Line::from(vec![Span::styled(
        "   j/k Toggle  a:All  Esc:Back",
        Style::default().fg(Color::DarkGray),
    )]));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{
        AppState, CategoryFiltersData, ConfigState, GameState, ProgressState, TypedScreen, UIState,
    };
    use ratatui::{Terminal, backend::TestBackend};

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

        let mut state = AppState {
            screen: TypedScreen::CategoryFilters(CategoryFiltersData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        };
        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());
        state
    }

    fn create_empty_state() -> AppState {
        let mut state = AppState {
            screen: TypedScreen::CategoryFilters(CategoryFiltersData::default()),
            ui: UIState::new(),
            game: GameState::new(vec![]),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        };
        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());
        state
    }

    #[test]
    fn test_render_category_filters_no_panic() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = create_test_state_with_categories();
        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_wrong_screen() {
        use crate::ui::state::MenuData;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = create_test_state_with_categories();
        state.screen = TypedScreen::Menu(MenuData::default());

        // Should not panic when called with wrong screen
        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_empty_categories() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = create_empty_state();
        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_with_selection() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = create_test_state_with_categories();
        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData {
            selected_index: 1, // Select second item
            return_to: crate::ui::state::ReturnDestination::Menu,
        });

        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_category_display_name_all_variants() {
        assert_eq!(
            category_display_name(&ScenarioCategory::Movement),
            "Movement"
        );
        assert_eq!(category_display_name(&ScenarioCategory::Editing), "Editing");
        assert_eq!(
            category_display_name(&ScenarioCategory::Clipboard),
            "Clipboard"
        );
        assert_eq!(category_display_name(&ScenarioCategory::Search), "Search");
        assert_eq!(
            category_display_name(&ScenarioCategory::Selection),
            "Selection"
        );
        assert_eq!(
            category_display_name(&ScenarioCategory::TextObjects),
            "Text Objects"
        );
        assert_eq!(
            category_display_name(&ScenarioCategory::Advanced),
            "Advanced"
        );
        assert_eq!(
            category_display_name(&ScenarioCategory::Registers),
            "Registers"
        );
        assert_eq!(category_display_name(&ScenarioCategory::Multi), "Multi");
        assert_eq!(category_display_name(&ScenarioCategory::Other), "Other");
    }

    #[test]
    fn test_category_icon_all_variants() {
        // Just verify all variants return non-empty strings
        assert!(!category_icon(&ScenarioCategory::Movement).is_empty());
        assert!(!category_icon(&ScenarioCategory::Editing).is_empty());
        assert!(!category_icon(&ScenarioCategory::Clipboard).is_empty());
        assert!(!category_icon(&ScenarioCategory::Search).is_empty());
        assert!(!category_icon(&ScenarioCategory::Selection).is_empty());
        assert!(!category_icon(&ScenarioCategory::TextObjects).is_empty());
        assert!(!category_icon(&ScenarioCategory::Advanced).is_empty());
        assert!(!category_icon(&ScenarioCategory::Registers).is_empty());
        assert!(!category_icon(&ScenarioCategory::Multi).is_empty());
        assert!(!category_icon(&ScenarioCategory::Other).is_empty());
    }

    #[test]
    fn test_category_icon_values() {
        assert_eq!(category_icon(&ScenarioCategory::Movement), "->");
        assert_eq!(category_icon(&ScenarioCategory::Editing), "ed");
        assert_eq!(category_icon(&ScenarioCategory::Clipboard), "[]");
        assert_eq!(category_icon(&ScenarioCategory::Search), "??");
        assert_eq!(category_icon(&ScenarioCategory::Selection), "##");
        assert_eq!(category_icon(&ScenarioCategory::TextObjects), "{}");
        assert_eq!(category_icon(&ScenarioCategory::Advanced), "++");
        assert_eq!(category_icon(&ScenarioCategory::Registers), "\"\"");
        assert_eq!(category_icon(&ScenarioCategory::Multi), "**");
        assert_eq!(category_icon(&ScenarioCategory::Other), "..");
    }

    #[test]
    fn test_render_category_filters_with_partial_filter() {
        use crate::config::ScenarioFilter;
        use std::collections::HashSet;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = create_test_state_with_categories();

        // Apply filter to enable only Movement category
        let mut categories = HashSet::new();
        categories.insert(ScenarioCategory::Movement);
        let filter = ScenarioFilter {
            categories: Some(categories),
            ..Default::default()
        };
        state.game.scenario_collection.apply_filter(&filter, None);

        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());

        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_small_terminal() {
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = create_test_state_with_categories();
        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_large_terminal() {
        let backend = TestBackend::new(200, 60);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = create_test_state_with_categories();
        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    fn create_state_with_all_categories() -> AppState {
        let scenarios = vec![
            create_test_scenario_with_category("s1", ScenarioCategory::Movement),
            create_test_scenario_with_category("s2", ScenarioCategory::Editing),
            create_test_scenario_with_category("s3", ScenarioCategory::Clipboard),
            create_test_scenario_with_category("s4", ScenarioCategory::Search),
            create_test_scenario_with_category("s5", ScenarioCategory::Selection),
            create_test_scenario_with_category("s6", ScenarioCategory::TextObjects),
            create_test_scenario_with_category("s7", ScenarioCategory::Advanced),
            create_test_scenario_with_category("s8", ScenarioCategory::Registers),
            create_test_scenario_with_category("s9", ScenarioCategory::Multi),
            create_test_scenario_with_category("s10", ScenarioCategory::Other),
        ];

        let mut state = AppState {
            screen: TypedScreen::CategoryFilters(CategoryFiltersData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        };
        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());
        state
    }

    #[test]
    fn test_render_category_filters_all_categories() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = create_state_with_all_categories();
        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_selection_at_last() {
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = create_state_with_all_categories();
        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData {
            selected_index: 9, // Last item (10 categories, 0-indexed)
            return_to: crate::ui::state::ReturnDestination::Menu,
        });

        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_no_categories_enabled() {
        use crate::config::ScenarioFilter;
        use std::collections::HashSet;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = create_test_state_with_categories();

        // Apply empty filter (no categories enabled)
        let filter = ScenarioFilter {
            categories: Some(HashSet::new()),
            ..Default::default()
        };
        state.game.scenario_collection.apply_filter(&filter, None);

        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData::default());

        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_category_filters_return_to_paused_minigame() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = create_test_state_with_categories();
        state.screen = TypedScreen::CategoryFilters(CategoryFiltersData {
            selected_index: 0,
            return_to: crate::ui::state::ReturnDestination::PausedMiniGame,
        });

        let result = terminal.draw(|f| render_category_filters(f, &state));
        assert!(result.is_ok());
    }
}
