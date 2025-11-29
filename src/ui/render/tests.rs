//! Tests for rendering functions

use crate::config::{ScoringConfig, Setup, Solution, TargetState};
use crate::gamification::{ProfileStorage, UserProfile};
use crate::learning::PerformanceTracker;
use crate::ui::state::AppState;

fn create_test_scenario() -> crate::config::Scenario {
    crate::config::Scenario {
        id: "test_001".to_string(),
        name: "Test Scenario".to_string(),
        description: "A test scenario for rendering".to_string(),
        setup: Setup {
            file_content: "line 1\n".to_string(),
            cursor_position: (0, 0),
        },
        target: TargetState {
            file_content: "line 2\n".to_string(),
            cursor_position: (0, 0),
            selection: None,
        },
        solution: Solution {
            commands: vec!["dd".to_string()],
            description: "Delete line".to_string(),
        },
        alternatives: vec![],
        hints: vec!["Test hint".to_string()],
        scoring: ScoringConfig {
            optimal_count: 1,
            max_points: 100,
            tolerance: 0,
        },
        metadata: None,
    }
}

fn create_test_app_state(scenarios: Vec<crate::config::Scenario>) -> AppState {
    let profile = UserProfile::new();
    let storage = ProfileStorage::new();
    let tracker = PerformanceTracker::new();
    AppState::new(scenarios, profile, storage, tracker)
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
