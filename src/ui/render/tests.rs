//! Tests for rendering functions

use crate::config::Scenario;
use crate::testing::{ScenarioBuilder, test_app_state_with_scenarios};
use crate::ui::state::AppState;

fn create_test_scenario() -> Scenario {
    ScenarioBuilder::new()
        .id("test_001")
        .description("A test scenario for rendering")
        .setup_content("line 1\n")
        .target_content("line 2\n")
        .hint("Test hint")
        .optimal_count(1)
        .build()
}

fn create_test_app_state(scenarios: Vec<Scenario>) -> AppState {
    test_app_state_with_scenarios(scenarios)
}

#[test]
fn test_render_does_not_panic_on_empty_state() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut state = create_test_app_state(vec![]);

    terminal
        .draw(|f| {
            super::super::render(f, &mut state);
        })
        .unwrap();
}

#[test]
fn test_render_task_screen_with_session() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Create a session
    crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

    terminal
        .draw(|f| {
            super::super::render(f, &mut state);
        })
        .unwrap();
}

// ==================== Helper Functions Tests ====================

mod helper_tests {
    #[test]
    fn test_centered_popup_exact_fit() {
        use ratatui::layout::Rect;

        let parent = Rect::new(0, 0, 80, 24);
        let popup = super::super::helpers::centered_popup(parent, 40, 10);

        assert_eq!(popup.x, 20); // (80 - 40) / 2
        assert_eq!(popup.y, 7); // (24 - 10) / 2
        assert_eq!(popup.width, 40);
        assert_eq!(popup.height, 10);
    }

    #[test]
    fn test_centered_popup_larger_than_parent() {
        use ratatui::layout::Rect;

        let parent = Rect::new(0, 0, 40, 10);
        let popup = super::super::helpers::centered_popup(parent, 80, 24);

        // Should saturate to 0 when popup is larger
        assert_eq!(popup.x, 0);
        assert_eq!(popup.y, 0);
        assert_eq!(popup.width, 80);
        assert_eq!(popup.height, 24);
    }

    #[test]
    fn test_centered_popup_small() {
        use ratatui::layout::Rect;

        let parent = Rect::new(0, 0, 100, 50);
        let popup = super::super::helpers::centered_popup(parent, 20, 10);

        assert_eq!(popup.x, 40); // (100 - 20) / 2
        assert_eq!(popup.y, 20); // (50 - 10) / 2
    }

    #[test]
    fn test_centered_popup_odd_dimensions() {
        use ratatui::layout::Rect;

        let parent = Rect::new(0, 0, 81, 25);
        let popup = super::super::helpers::centered_popup(parent, 40, 10);

        // Integer division truncates
        assert_eq!(popup.x, 20); // (81 - 40) / 2 = 20
        assert_eq!(popup.y, 7); // (25 - 10) / 2 = 7
    }

    #[test]
    fn test_inner_rect_standard() {
        use ratatui::layout::Rect;

        let outer = Rect::new(10, 5, 40, 20);
        let inner = super::super::helpers::inner_rect(outer);

        assert_eq!(inner.x, 11); // outer.x + 1
        assert_eq!(inner.y, 6); // outer.y + 1
        assert_eq!(inner.width, 38); // outer.width - 2
        assert_eq!(inner.height, 18); // outer.height - 2
    }

    #[test]
    fn test_inner_rect_minimal() {
        use ratatui::layout::Rect;

        let outer = Rect::new(0, 0, 2, 2);
        let inner = super::super::helpers::inner_rect(outer);

        assert_eq!(inner.x, 1);
        assert_eq!(inner.y, 1);
        assert_eq!(inner.width, 0); // 2 - 2 = 0
        assert_eq!(inner.height, 0);
    }

    #[test]
    fn test_inner_rect_too_small() {
        use ratatui::layout::Rect;

        let outer = Rect::new(0, 0, 1, 1);
        let inner = super::super::helpers::inner_rect(outer);

        // Should saturate to 0
        assert_eq!(inner.width, 0);
        assert_eq!(inner.height, 0);
    }

    #[test]
    fn test_inner_rect_zero_size() {
        use ratatui::layout::Rect;

        let outer = Rect::new(5, 5, 0, 0);
        let inner = super::super::helpers::inner_rect(outer);

        assert_eq!(inner.x, 6);
        assert_eq!(inner.y, 6);
        assert_eq!(inner.width, 0);
        assert_eq!(inner.height, 0);
    }

    #[test]
    fn test_popup_block_with_title() {
        use ratatui::style::Color;

        let block = super::super::helpers::popup_block(Some("Test Title"), Color::Red);

        // Block should be created without panicking
        // We can't easily inspect the block's internal state, but we can verify it renders
        assert!(std::mem::size_of_val(&block) > 0);
    }

    #[test]
    fn test_popup_block_without_title() {
        use ratatui::style::Color;

        let block = super::super::helpers::popup_block(None, Color::Green);

        assert!(std::mem::size_of_val(&block) > 0);
    }
}

// ==================== Editor Rendering Tests ====================

mod editor_tests {
    use crate::helix::SelectionBounds;
    use crate::ui::render::editor::CursorInfo;

    fn primary_cursor(row: usize, col: usize) -> Vec<CursorInfo> {
        vec![CursorInfo {
            row,
            col,
            is_primary: true,
        }]
    }

    #[test]
    fn test_render_editor_with_diff_matching_lines() {
        let lines = super::super::editor::render_editor_with_diff(
            "line 1\nline 2\n",
            "line 1\nline 2\n",
            &primary_cursor(0, 0),
            &[],
            None,
        );
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_render_editor_with_diff_different_lines() {
        let lines = super::super::editor::render_editor_with_diff(
            "line 1\nline 2\n",
            "line 1\nline X\n",
            &primary_cursor(0, 0),
            &[],
            None,
        );
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_render_editor_with_diff_cursor_on_first_line() {
        let lines = super::super::editor::render_editor_with_diff(
            "hello world\nsecond line\n",
            "hello world\nsecond line\n",
            &primary_cursor(0, 5),
            &[],
            None,
        );
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_render_editor_with_diff_cursor_at_end_of_line() {
        let lines = super::super::editor::render_editor_with_diff(
            "hello\n",
            "hello\n",
            &primary_cursor(0, 5),
            &[],
            None,
        );
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_editor_with_diff_empty_content() {
        let lines =
            super::super::editor::render_editor_with_diff("", "", &primary_cursor(0, 0), &[], None);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_editor_with_diff_single_empty_line() {
        let lines = super::super::editor::render_editor_with_diff(
            "\n",
            "\n",
            &primary_cursor(0, 0),
            &[],
            None,
        );
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_editor_with_diff_selection_present() {
        let selection = SelectionBounds::new(0, 0, 0, 5);
        let lines = super::super::editor::render_editor_with_diff(
            "hello world\n",
            "hello world\n",
            &primary_cursor(0, 0),
            &[selection],
            None,
        );
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_editor_with_diff_target_shorter() {
        let lines = super::super::editor::render_editor_with_diff(
            "line 1\nline 2\nline 3\n",
            "line 1\n",
            &primary_cursor(0, 0),
            &[],
            None,
        );
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_render_editor_with_diff_target_longer() {
        let lines = super::super::editor::render_editor_with_diff(
            "line 1\n",
            "line 1\nline 2\nline 3\n",
            &primary_cursor(0, 0),
            &[],
            None,
        );
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_editor_cursor_middle_of_line() {
        let lines = super::super::editor::render_editor_with_diff(
            "hello world\n",
            "hello world\n",
            &primary_cursor(0, 6),
            &[],
            None,
        );
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_render_editor_unicode_content() {
        let lines = super::super::editor::render_editor_with_diff(
            "héllo wörld\n",
            "héllo wörld\n",
            &primary_cursor(0, 0),
            &[],
            None,
        );
        assert_eq!(lines.len(), 1);
    }
}

// ==================== Screen Rendering Integration Tests ====================

mod screen_render_tests {
    use super::*;
    use crate::ui::state::{
        AchievementsData, MenuData, MiniGameData, ModeSelectionData, ProfileData,
        ReturnDestination, StatisticsData, TypedScreen,
    };
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_mode_selection_screen() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::ModeSelection(ModeSelectionData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_menu_screen() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Menu(MenuData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_profile_screen() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_achievements_screen() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Achievements(AchievementsData {
            return_to: ReturnDestination::Menu,
            scroll_offset: 0,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    /// Regression test for #292 finding F4 (impl-critic): at the repo's own render-test
    /// size (80x24), `margin(2)` plus `Length(3)+Min(15)+Length(3)` needs 21 rows but
    /// only 20 are available, so ratatui's layout solver silently squeezed the
    /// Instructions section down to 2 rows - just its top/bottom border, with zero rows
    /// left for the text inside. That swallowed both the key hints and the scroll
    /// position indicator with no visible sign anything was cut off.
    #[test]
    fn test_render_achievements_screen_instructions_bar_visible_at_min_size() {
        use ratatui::buffer::Buffer;

        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Achievements(AchievementsData {
            return_to: ReturnDestination::Menu,
            scroll_offset: 0,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();

        let buffer: Buffer = terminal.backend().buffer().clone();
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            text.contains("menu"),
            "instructions bar text must be rendered at 80x24, got buffer text: {text}"
        );
    }

    /// Regression test for #292 finding F4 (impl-critic): at the repo's own render-test
    /// size (80x24), the achievement list's content area is too short to fit all 18
    /// entries. Before scrolling was added, entries past the visible window were simply
    /// invisible with no indicator - indistinguishable from an empty list on a fresh
    /// profile where every entry renders the same gray color. Scrolling past the end
    /// must clamp instead of panicking or scrolling into blank space.
    #[test]
    fn test_render_achievements_screen_clamps_scroll_past_content() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Achievements(AchievementsData {
            return_to: ReturnDestination::Menu,
            // Deliberately far past the end of the 18-achievement list.
            scroll_offset: 9999,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();

        let TypedScreen::Achievements(data) = &state.screen else {
            panic!("expected Achievements screen");
        };
        assert!(
            data.scroll_offset < 18,
            "scroll offset must be clamped to the actual content length, got {}",
            data.scroll_offset
        );
    }

    // Note: Review screen test omitted as ReviewData requires ReviewSessionState
    // which needs complex setup. The render function is tested via integration tests.

    #[test]
    fn test_render_task_screen_with_active_session() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Start a scenario to get a Task screen with active session
        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    // Note: Results screen test omitted as ResultsData requires completed/abandoned
    // GameSession which needs complex setup. The render function is covered by
    // integration tests that go through the full game flow.

    #[test]
    fn test_render_minigame_screen() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_notifications() {
        use crate::ui::notification::{Notification, NotificationType};

        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        // Add a notification
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::LevelUp {
                new_level: 5,
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_multiple_notifications() {
        use crate::ui::notification::{Notification, NotificationType};

        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        // Add multiple notifications
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::LevelUp {
                new_level: 2,
            }));
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::QuestComplete {
                description: "Test Quest".to_string(),
                xp_reward: 100,
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_small_terminal() {
        // Test rendering with minimal terminal size
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_large_terminal() {
        // Test rendering with large terminal
        let backend = TestBackend::new(200, 60);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_profile_from_paused_minigame() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);
        state.screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::PausedMiniGame,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_high_level_profile() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        // Set high level profile
        state.progress.profile.level = 99;
        state.progress.profile.total_xp = 1_000_000;
        state.progress.profile.scenarios_completed = 500;
        state.progress.profile.perfect_scenarios = 100;

        state.screen = TypedScreen::Profile(ProfileData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_menu_with_many_scenarios() {
        let scenarios: Vec<_> = (0..100)
            .map(|i| create_test_scenario_with_id(&format!("scenario_{}", i)))
            .collect();
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(scenarios);
        state.screen = TypedScreen::Menu(MenuData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}

fn create_test_scenario_with_id(id: &str) -> Scenario {
    ScenarioBuilder::new()
        .id(id)
        .description("A test scenario for rendering")
        .setup_content("line 1\n")
        .target_content("line 2\n")
        .hint("Test hint")
        .optimal_count(1)
        .build()
}

// Note: Results and Review rendering tests require creating complete session state
// which is covered by existing integration tests. The render functions themselves
// have complex dependencies on session state that are better tested through
// the integration test suite.

// ============================================================================
// Review Screen Tests - using proper session state
// ============================================================================

mod review_tests {
    use super::*;
    use crate::ui::state::{ReviewData, ReviewSessionState, TypedScreen};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::time::Instant;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    fn create_review_session(commands: Vec<&str>) -> ReviewSessionState {
        ReviewSessionState {
            due_commands: commands.into_iter().map(String::from).collect(),
            current_index: 0,
            current_command: Some("h".to_string()),
            session_started_at: Instant::now(),
            completed_reviews: vec![],
        }
    }

    #[test]
    fn test_render_review_screen_basic() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        let session = create_review_session(vec!["h", "j", "k", "l"]);
        state.game.review_session = Some(session.clone());
        state.screen = TypedScreen::Review(ReviewData { session });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_review_screen_single_command() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        let session = create_review_session(vec!["w"]);
        state.game.review_session = Some(session.clone());
        state.screen = TypedScreen::Review(ReviewData { session });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_review_screen_progress_midway() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        let mut session = create_review_session(vec!["h", "j", "k", "l"]);
        session.current_index = 2; // Midway through
        session.current_command = Some("k".to_string());
        state.game.review_session = Some(session.clone());
        state.screen = TypedScreen::Review(ReviewData { session });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}

// ============================================================================
// Popup Rendering Tests
// ============================================================================

mod popup_tests {
    use super::*;
    use crate::ui::notification::{Notification, NotificationType};
    use crate::ui::state::TypedScreen;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_hint_popup() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        // Start scenario and show hint
        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();
        crate::ui::update(&mut state, crate::ui::Message::ShowHint).unwrap();

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_key_history_popup_empty() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        // Key history should be empty initially
        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_key_history_popup_with_keys() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        // Add some key history (simulate key presses)
        if let TypedScreen::Task(ref mut task_data) = state.screen {
            task_data.key_history.push("h".to_string());
            task_data.key_history.push("j".to_string());
            task_data.key_history.push("k".to_string());
        }

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_level_up() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::LevelUp {
                new_level: 10,
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_quest_complete() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::QuestComplete {
                description: "Complete 5 scenarios".to_string(),
                xp_reward: 200,
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_achievement() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::Achievement {
                name: "Speed Demon".to_string(),
                description: "Complete a scenario in under 5 seconds".to_string(),
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_streak_milestone() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::StreakMilestone {
                streak: 7,
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_mastery_level_up() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::MasteryLevelUp {
                command: "dd".to_string(),
                new_level: "Intermediate".to_string(),
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_max_visible() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        // Add more than max visible notifications
        for i in 0..5 {
            state
                .ui
                .notifications
                .push(Notification::new(NotificationType::LevelUp {
                    new_level: i,
                }));
        }

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_info() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::Info {
                message: "Welcome to the training session!".to_string(),
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_review_session_complete() {
        let mut terminal = create_terminal();
        let mut state = create_test_app_state(vec![create_test_scenario()]);

        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::ReviewSessionComplete {
                completed: 10,
                success_count: 8,
                xp_earned: 250,
            }));

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_success_popup() {
        // Success popup is only shown briefly after completing a scenario
        // This test ensures it doesn't panic when rendered
        let mut terminal = create_terminal();
        let _state = create_test_app_state(vec![create_test_scenario()]);

        // The success popup helper can be called directly for testing
        terminal
            .draw(|f| {
                super::super::popups::render_success_popup(f);
            })
            .unwrap();
    }

    #[test]
    fn test_render_result_popup_custom() {
        use ratatui::style::Color;

        let mut terminal = create_terminal();

        terminal
            .draw(|f| {
                super::super::popups::render_result_popup(f, "TIMEOUT", "Time's up!", Color::Red);
            })
            .unwrap();
    }
}

// ============================================================================
// Results Screen Tests - full coverage for results.rs (0% -> target 90%)
// ============================================================================

mod results_tests {
    use super::*;
    use crate::game::GameSession;
    use crate::learning::ScenarioMastery;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{QuestProgressChange, ResultsData, TypedScreen, XPBreakdown};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 40);
        Terminal::new(backend).unwrap()
    }

    /// Create a scenario that can be easily completed for testing
    fn create_completable_scenario() -> Scenario {
        // Scenario where deleting line 2 (dd) completes it
        ScenarioBuilder::new()
            .id("test_results_001")
            .description("Delete second line")
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0) // Start at line 2
            .target_content("line 1\n")
            .target_cursor(0, 0)
            .hint("Use dd to delete the line")
            .optimal_count(1)
            .build()
    }

    /// Create an abandoned session and ResultsData from it
    fn create_abandoned_results_data(scenario: Scenario) -> ResultsData {
        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();
        let feedback = abandoned.feedback();
        ResultsData::from_abandoned(abandoned, feedback, Some(0))
    }

    fn create_xp_breakdown(
        base: u64,
        perfect_bonus: u64,
        first_today: u64,
        mastery_factor: f64,
        repeat_penalty: f64,
    ) -> XPBreakdown {
        let total =
            ((base + perfect_bonus + first_today) as f64 * mastery_factor * repeat_penalty) as u64;
        XPBreakdown {
            base_xp: base,
            perfect_bonus,
            first_today_bonus: first_today,
            mastery_multiplier: mastery_factor * repeat_penalty,
            mastery_factor,
            repeat_penalty,
            quest_bonuses: vec![],
            total_xp: total,
        }
    }

    #[test]
    fn test_render_results_screen_abandoned() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let results_data = create_abandoned_results_data(scenario);
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_with_xp_breakdown() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 25, 10, 1.0, 1.0));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_with_mastery_reduction() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 0, 0, 0.5, 1.0));
        results_data.scenario_mastery = Some((ScenarioMastery::Proficient, 0.5));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_with_repeat_penalty() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 0, 0, 1.0, 0.7));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_with_quest_bonuses() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        let mut xp = create_xp_breakdown(50, 25, 0, 1.0, 1.0);
        xp.quest_bonuses = vec![
            ("Complete 5 scenarios".to_string(), 100),
            ("Use movement commands".to_string(), 50),
        ];
        xp.total_xp += 150;
        results_data.xp_breakdown = Some(xp);
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_with_quest_changes() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 0, 0, 1.0, 1.0));
        results_data.quest_changes = vec![
            QuestProgressChange {
                quest_description: "Complete 5 scenarios".to_string(),
                old_progress: 2,
                new_progress: 3,
            },
            QuestProgressChange {
                quest_description: "Practice movement keys".to_string(),
                old_progress: 5,
                new_progress: 8,
            },
        ];
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_with_profile_stats() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        // Set up profile with various stats
        state.progress.profile.level = 15;
        state.progress.profile.total_xp = 5000;
        state.progress.profile.scenarios_completed = 50;
        state.progress.profile.perfect_scenarios = 20;
        state.progress.profile.current_streak = 7;

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 25, 10, 1.0, 1.0));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_no_streak() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        // No current streak
        state.progress.profile.current_streak = 0;

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 0, 0, 1.0, 1.0));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_small_terminal() {
        // Test rendering on small terminal
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 0, 0, 1.0, 1.0));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_wrong_screen_type() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Render with Menu screen - results render function should early return
        state.screen = TypedScreen::Menu(crate::ui::state::MenuData::default());

        // Call render directly - should not panic on wrong screen type
        terminal
            .draw(|f| {
                super::super::results::render_results_screen(f, &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_mastered_scenario() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.scenario_mastery = Some((ScenarioMastery::Mastered, 0.25));
        results_data.xp_breakdown = Some(create_xp_breakdown(50, 25, 0, 0.25, 1.0));
        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_results_screen_all_bonuses_combined() {
        let mut terminal = create_terminal();
        let scenario = create_completable_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        // Set profile for streak display
        state.progress.profile.current_streak = 5;
        state.progress.profile.level = 10;
        state.progress.profile.total_xp = 3000;

        // XP with all bonuses and penalties
        let xp = XPBreakdown {
            base_xp: 50,
            perfect_bonus: 25,
            first_today_bonus: 10,
            mastery_multiplier: 0.5 * 0.8,
            mastery_factor: 0.5,
            repeat_penalty: 0.8,
            quest_bonuses: vec![("Quest bonus".to_string(), 30)],
            total_xp: 46, // (50+25+10) * 0.5 * 0.8 + 30
        };

        let mut results_data = create_abandoned_results_data(scenario);
        results_data.scenario_mastery = Some((ScenarioMastery::Mastered, 0.5));
        results_data.quest_changes = vec![QuestProgressChange {
            quest_description: "Daily quest".to_string(),
            old_progress: 4,
            new_progress: 5,
        }];
        results_data.xp_breakdown = Some(xp);

        state.screen = TypedScreen::Results(results_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}

// ============================================================================
// Statistics Screen Tests - improve coverage for statistics.rs (66% -> target 90%)
// ============================================================================

mod statistics_tests {
    use super::*;
    use crate::ui::state::{ReturnDestination, StatisticsData, TypedScreen};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(100, 50);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_statistics_screen_no_commands_tracked() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_tracked_commands() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add some tracked commands via record_attempt
        let dur = std::time::Duration::from_secs(1);
        let optimal = std::time::Duration::from_secs(2);
        state
            .progress
            .performance_tracker
            .record_attempt("h", dur, true, optimal);
        state
            .progress
            .performance_tracker
            .record_attempt("j", dur, true, optimal);
        state
            .progress
            .performance_tracker
            .record_attempt("k", dur, true, optimal);
        state
            .progress
            .performance_tracker
            .record_attempt("l", dur, true, optimal);

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_weak_commands() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        let dur = std::time::Duration::from_secs(1);
        let optimal = std::time::Duration::from_secs(2);

        // Create weak commands (low success rate)
        for _ in 0..5 {
            state
                .progress
                .performance_tracker
                .record_attempt("dd", dur, false, optimal);
        }
        state
            .progress
            .performance_tracker
            .record_attempt("dd", dur, true, optimal);

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_many_weak_commands() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        let dur = std::time::Duration::from_secs(1);
        let optimal = std::time::Duration::from_secs(2);

        // Create more than 5 weak commands to test "... and N more" display
        let commands = ["dd", "yy", "cc", "pp", "x", "r", "s"];
        for cmd in &commands {
            for _ in 0..3 {
                state
                    .progress
                    .performance_tracker
                    .record_attempt(cmd, dur, false, optimal);
            }
        }

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_due_reviews() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        let dur = std::time::Duration::from_secs(1);
        let optimal = std::time::Duration::from_secs(2);

        // Record some commands to create due reviews
        for _ in 0..10 {
            state
                .progress
                .performance_tracker
                .record_attempt("h", dur, true, optimal);
        }

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_scenario_mastery() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add some scenario history
        state.progress.profile.scenarios_completed = 10;
        state.progress.profile.perfect_scenarios = 5;

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_quest_stats() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Set up streaks
        state.progress.profile.current_streak = 7;
        state.progress.profile.longest_streak = 14;

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_with_arcade_stats() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Set up arcade mode stats
        state.progress.profile.minigame_high_score = 5000;
        state.progress.profile.minigame_best_streak = 15;
        state.progress.profile.minigame_games_played = 25;

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_return_from_paused_minigame() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::PausedMiniGame,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_mixed_success_rates() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        let dur = std::time::Duration::from_secs(1);
        let optimal = std::time::Duration::from_secs(2);

        // High success rate command
        for _ in 0..10 {
            state
                .progress
                .performance_tracker
                .record_attempt("h", dur, true, optimal);
        }

        // Medium success rate command
        for _ in 0..5 {
            state
                .progress
                .performance_tracker
                .record_attempt("j", dur, true, optimal);
        }
        for _ in 0..3 {
            state
                .progress
                .performance_tracker
                .record_attempt("j", dur, false, optimal);
        }

        // Low success rate command
        for _ in 0..2 {
            state
                .progress
                .performance_tracker
                .record_attempt("dd", dur, true, optimal);
        }
        for _ in 0..8 {
            state
                .progress
                .performance_tracker
                .record_attempt("dd", dur, false, optimal);
        }

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_statistics_screen_session_time_hours() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Simulate session that's been running for a while
        // We can't directly modify session_start_time easily, but the render should handle any duration

        state.screen = TypedScreen::Statistics(StatisticsData {
            return_to: ReturnDestination::Menu,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}

// ============================================================================
// Mode Selection Tests - improve coverage for mode_selection.rs (59% -> target 90%)
// ============================================================================

mod mode_selection_extended_tests {
    use super::*;
    use crate::ui::state::{MiniGameModeSelection, ModeSelectionData, TypedScreen};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_mode_selection_training_selected() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 0,
            minigame_mode_selection: None,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mode_selection_arcade_selected() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: None,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mode_selection_submenu_arcade() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: Some(MiniGameModeSelection { selected_index: 0 }),
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mode_selection_submenu_survival() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: Some(MiniGameModeSelection { selected_index: 1 }),
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mode_selection_submenu_challenge() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: Some(MiniGameModeSelection { selected_index: 2 }),
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mode_selection_small_terminal() {
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 1,
            minigame_mode_selection: Some(MiniGameModeSelection { selected_index: 0 }),
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_mode_selection_large_terminal() {
        let backend = TestBackend::new(200, 60);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        state.screen = TypedScreen::ModeSelection(ModeSelectionData {
            selected_mode: 0,
            minigame_mode_selection: None,
        });

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}

// ============================================================================
// Mini-game Screen Tests - improve coverage for minigame.rs (61% -> target 85%)
// ============================================================================

mod minigame_screen_tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::minigame::MiniGameSession;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{MiniGameData, TypedScreen};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::sync::Arc;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(100, 40);
        Terminal::new(backend).unwrap()
    }

    fn create_minigame_scenario(id: &str, difficulty: Difficulty) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(0, 0)
            .optimal_count(1)
            .difficulty(difficulty)
            .build()
    }

    #[test]
    fn test_render_minigame_playing_state() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        // Advance past countdown
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_paused_state() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }
        session.pause();

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_countdown_state() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        // Only tick once - still in countdown
        session.tick_countdown();

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_with_queue() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![
            create_minigame_scenario("s1", Difficulty::Beginner),
            create_minigame_scenario("s2", Difficulty::Intermediate),
            create_minigame_scenario("s3", Difficulty::Advanced),
            create_minigame_scenario("s4", Difficulty::Beginner),
        ]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_with_key_history() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        let mut minigame_data = MiniGameData::default();
        minigame_data.key_history.push("h".to_string());
        minigame_data.key_history.push("j".to_string());
        minigame_data.key_history.push("k".to_string());
        state.screen = TypedScreen::MiniGame(minigame_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_with_xp_earned() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        let minigame_data = MiniGameData {
            last_xp_earned: Some(75),
            ..Default::default()
        };
        state.screen = TypedScreen::MiniGame(minigame_data);

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_intermediate_difficulty() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario(
            "s1",
            Difficulty::Intermediate,
        )]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_advanced_difficulty() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Advanced)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_empty_queue() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        // Single scenario - queue will be empty after first
        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_small_terminal() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_minigame_large_terminal() {
        let backend = TestBackend::new(200, 60);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario.clone()]);

        let scenarios = Arc::new(vec![create_minigame_scenario("s1", Difficulty::Beginner)]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        for _ in 0..4 {
            session.tick_countdown();
        }

        state.game.minigame_session = Some(session);
        state.screen = TypedScreen::MiniGame(MiniGameData::default());

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}

// ============================================================================
// Popup Tests - improve coverage for popups.rs (69% -> target 90%)
// ============================================================================

mod popup_additional_tests {
    use super::*;
    use crate::ui::state::TypedScreen;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn create_terminal() -> Terminal<TestBackend> {
        let backend = TestBackend::new(80, 24);
        Terminal::new(backend).unwrap()
    }

    #[test]
    fn test_render_key_history_popup_max_keys() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        // Add maximum keys
        if let TypedScreen::Task(ref mut task_data) = state.screen {
            for key in ["h", "j", "k", "l", "w"] {
                task_data.key_history.push(key.to_string());
            }
        }

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_key_history_popup_special_keys() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        // Add special multi-character keys
        if let TypedScreen::Task(ref mut task_data) = state.screen {
            task_data.key_history.push("gg".to_string());
            task_data.key_history.push("dd".to_string());
            task_data.key_history.push("yy".to_string());
        }

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_hint_popup_long_hint() {
        let mut terminal = create_terminal();

        // Create scenario with very long hint
        let scenario = crate::testing::ScenarioBuilder::new()
            .id("long_hint_001")
            .description("Test long hint")
            .setup_content("line 1\n")
            .target_content("line 2\n")
            .hint("This is a very long hint that should wrap across multiple lines in the popup window to test the text wrapping functionality of the hint popup rendering.")
            .optimal_count(1)
            .build();

        let mut state = create_test_app_state(vec![scenario]);
        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();
        crate::ui::update(&mut state, crate::ui::Message::ShowHint).unwrap();

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_hint_popup_wrong_screen() {
        let mut terminal = create_terminal();
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Set to menu screen - hint popup should not render
        state.screen = TypedScreen::Menu(crate::ui::state::MenuData::default());

        terminal
            .draw(|f| {
                super::super::popups::render_hint_popup(f, &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_notifications_off_screen() {
        // Very small terminal - notifications might overflow
        let backend = TestBackend::new(30, 8);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);

        // Add multiple notifications
        for i in 0..5 {
            state
                .ui
                .notifications
                .push(crate::ui::notification::Notification::new(
                    crate::ui::notification::NotificationType::LevelUp { new_level: i },
                ));
        }

        terminal
            .draw(|f| {
                super::super::render(f, &mut state);
            })
            .unwrap();
    }
}
