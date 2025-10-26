//! Pure rendering functions for the TUI
//!
//! This module contains all rendering logic for the terminal user interface.
//! All functions are pure (no side effects) and take an immutable reference
//! to the application state.

use crate::ui::state::{AppState, Screen};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Main render function dispatches to screen-specific renderers
///
/// This is the entry point for all rendering. It's pure and has no side effects.
///
/// # Arguments
///
/// * `frame` - The ratatui frame to render to
/// * `state` - The application state (immutable)
pub fn render(frame: &mut Frame, state: &AppState) {
    match state.screen {
        Screen::MainMenu => render_main_menu(frame, state),
        Screen::Task => render_task_screen(frame, state),
        Screen::Results => render_results_screen(frame, state),
    }
}

/// Render the main menu screen
fn render_main_menu(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Create layout: title | menu | instructions
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Min(4),     // Menu items
            Constraint::Length(3),  // Instructions
        ])
        .split(area);

    // Title
    let title = Paragraph::new("Helix Keybindings Trainer")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Menu items
    let menu_items: Vec<ListItem> = AppState::menu_items()
        .iter()
        .enumerate()
        .map(|(i, &item)| {
            let selected = i == state.selected_menu_item;
            let style = if selected {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if selected { "> " } else { "  " };
            ListItem::new(format!("{}{}", prefix, item)).style(style)
        })
        .collect();

    let menu = List::new(menu_items)
        .block(Block::default().title("Main Menu").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(menu, chunks[1]);

    // Instructions
    let instructions = Paragraph::new("↑/↓: Navigate | Enter: Select | q: Quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(instructions, chunks[2]);
}

/// Render the task screen where user plays a scenario
fn render_task_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if let Some(session) = &state.session {
        let scenario = session.scenario();

        // Layout: title | description | editor view | stats | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),    // Title
                Constraint::Length(4),    // Description
                Constraint::Min(8),       // Editor view
                Constraint::Length(3),    // Stats
                Constraint::Length(3),    // Instructions
            ])
            .split(area);

        // Title
        let title = Paragraph::new(format!("Scenario: {}", scenario.name))
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Description
        let description = Paragraph::new(scenario.description.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White))
            .block(Block::default().title("Task").borders(Borders::ALL));
        frame.render_widget(description, chunks[1]);

        // Editor view - split into current and target
        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Current state
        let current_content = session.current_state().content();
        let current = Paragraph::new(current_content)
            .block(Block::default().title("Current State").borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan))
            .wrap(Wrap { trim: true });
        frame.render_widget(current, editor_chunks[0]);

        // Target state
        let target_content = session.target_state().content();
        let target = Paragraph::new(target_content)
            .block(Block::default().title("Target State").borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: true });
        frame.render_widget(target, editor_chunks[1]);

        // Stats
        let optimal = scenario.scoring.optimal_count;
        let actions = session.action_count();
        let stats_text = if actions <= optimal {
            format!("Actions: {} (optimal: {})", actions, optimal)
        } else {
            format!("Actions: {} (optimal: {}) - {} extra", actions, optimal, actions - optimal)
        };

        let stats = Paragraph::new(stats_text)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(stats, chunks[3]);

        // Instructions with hint indicator
        let hint_indicator = if state.show_hint_panel && state.current_hint.is_some() {
            " [h: Next Hint] "
        } else {
            " [h: Show Hint] "
        };

        let instructions = Paragraph::new(format!("{}| Esc: Abandon | Ctrl-c: Quit", hint_indicator))
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[4]);

        // Render hint panel if visible
        if state.show_hint_panel {
            render_hint_popup(frame, state);
        }
    }
}

/// Render the results screen showing scenario completion
fn render_results_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if let Some(session) = &state.session {
        if let Ok(feedback) = session.get_feedback() {
            // Layout: title | results | instructions
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Min(10),    // Results
                    Constraint::Length(3),  // Instructions
                ])
                .split(area);

            // Title
            let title_text = if feedback.success { "✓ Completed!" } else { "✗ Not Completed" };
            let title_color = if feedback.success { Color::Green } else { Color::Red };
            let title = Paragraph::new(title_text)
                .style(Style::default().fg(title_color).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(title, chunks[0]);

            // Results content
            let mut result_lines = vec![];

            // Rating and score
            result_lines.push(Line::from(vec![
                Span::styled(
                    format!("{} {}", feedback.rating.emoji(), feedback.rating.description()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            result_lines.push(Line::from(""));

            // Score
            result_lines.push(Line::from(vec![
                Span::raw("Score: "),
                Span::styled(
                    format!("{}/{}", feedback.score, feedback.max_points),
                    Style::default().fg(Color::Cyan),
                ),
            ]));

            // Actions
            let action_color = if feedback.is_optimal {
                Color::Green
            } else {
                Color::Yellow
            };
            result_lines.push(Line::from(vec![
                Span::raw("Actions: "),
                Span::styled(
                    format!("{}", feedback.actions_taken),
                    Style::default().fg(action_color),
                ),
                Span::raw(format!(" (optimal: {})", feedback.optimal_actions)),
            ]));

            // Duration
            result_lines.push(Line::from(vec![
                Span::raw("Time: "),
                Span::styled(
                    format!("{:.1}s", feedback.duration.as_secs_f32()),
                    Style::default().fg(Color::Blue),
                ),
            ]));

            // Hint if provided
            if let Some(hint) = &feedback.hint {
                result_lines.push(Line::from(""));
                result_lines.push(Line::from(vec![
                    Span::styled("Tip: ", Style::default().fg(Color::Magenta)),
                    Span::raw(hint),
                ]));
            }

            let results = Paragraph::new(result_lines)
                .block(Block::default().title("Performance").borders(Borders::ALL))
                .alignment(Alignment::Left)
                .style(Style::default().fg(Color::White));
            frame.render_widget(results, chunks[1]);

            // Instructions
            let instructions = Paragraph::new("[r] Retry  [m] Menu  [q] Quit")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(instructions, chunks[2]);
        }
    }
}

/// Render a centered hint popup
fn render_hint_popup(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    // Create a centered popup (centered horizontally and vertically)
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(4));

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Render semi-transparent background (using a block)
    let background = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White))
        .title("Hint");
    frame.render_widget(&background, popup_area);

    // Render hint text inside popup
    if let Some(hint) = &state.current_hint {
        let inner = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        let hint_text = Paragraph::new(hint.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        frame.render_widget(hint_text, inner);
    }
}

#[cfg(test)]
#[allow(unused_variables)] // Test backends don't use all variables
mod tests {
    use super::*;
    use crate::config::{ScoringConfig, Setup, Solution, TargetState};
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
        }
    }

    #[test]
    fn test_main_menu_items_count() {
        let items = AppState::menu_items();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_render_does_not_panic_on_empty_state() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let state = AppState::new(vec![]);

        terminal
            .draw(|f| {
                render(f, &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_task_screen_with_session() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenario = create_test_scenario();
        let mut state = AppState::new(vec![scenario]);

        // Create a session
        crate::ui::update(&mut state, crate::ui::Message::StartScenario(0)).unwrap();

        terminal
            .draw(|f| {
                render(f, &state);
            })
            .unwrap();
    }
}
