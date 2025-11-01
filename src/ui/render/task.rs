//! Task screen rendering

use super::editor::{render_editor_with_diff, render_editor_with_selection};
use super::popups::{render_hint_popup, render_key_history_popup, render_success_popup};
use crate::ui::state::AppState;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use rust_i18n::t;

/// Render the task screen where user plays a scenario
pub(super) fn render_task_screen(frame: &mut Frame, state: &AppState) {
    let area = frame.area();

    if let Some(session) = &state.session {
        let scenario = session.scenario();

        // Layout: title | description | editor view | stats | instructions
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(4), // Description
                Constraint::Min(8),    // Editor view
                Constraint::Length(3), // Stats
                Constraint::Length(3), // Instructions
            ])
            .split(area);

        // Title
        let title = Paragraph::new(format!("Scenario: {}", scenario.name))
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, chunks[0]);

        // Description
        let description = Paragraph::new(scenario.description.as_str())
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(t!("task.title").to_string())
                    .borders(Borders::ALL),
            );
        frame.render_widget(description, chunks[1]);

        // Editor view - split into current and target
        let editor_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Current state with cursor and diff highlighting
        let current_state = session.current_state();
        let target_state = session.target_state();
        let current_lines = render_editor_with_diff(current_state, target_state);
        let current = Paragraph::new(current_lines)
            .block(
                Block::default()
                    .title(t!("editor.current_state").to_string())
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(current, editor_chunks[0]);

        // Target state with selection highlighting (if any)
        let target_state = session.target_state();
        let target_lines = render_editor_with_selection(target_state);
        let target = Paragraph::new(target_lines)
            .block(
                Block::default()
                    .title(t!("editor.target_state").to_string())
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(target, editor_chunks[1]);

        // Stats with mode indicator and progress
        let optimal = scenario.scoring.optimal_count;
        let actions = session.action_count();
        let elapsed = session.elapsed();
        let elapsed_secs = elapsed.as_secs_f32();
        let mode = session.mode_name();
        let progress = session.completion_progress();

        // Color code mode: green for Normal, yellow for Insert
        let mode_color = if mode == "NORMAL" {
            Color::Green
        } else {
            Color::Yellow
        };

        // Color code progress: green if 100%, yellow if >50%, red otherwise
        let progress_color = if progress == 100 {
            Color::Green
        } else if progress > 50 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Translate mode name for display
        let mode_display = if mode == "NORMAL" {
            t!("task.mode_normal")
        } else {
            t!("task.mode_insert")
        };

        // Create colored mode indicator
        let mode_span = Span::styled(
            format!("Mode: {} ", mode_display),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        );

        // Create colored progress indicator
        let progress_span = Span::styled(
            format!("| Progress: {}% ", progress),
            Style::default()
                .fg(progress_color)
                .add_modifier(Modifier::BOLD),
        );

        // Create rest of stats
        let rest_of_stats = if actions <= optimal {
            format!(
                "| {}: {} ({}: {}) | Time: {:.1}s",
                t!("task.actions"),
                actions,
                t!("task.optimal"),
                optimal,
                elapsed_secs
            )
        } else {
            format!(
                "| {}: {} ({}: {}) - {} extra | Time: {:.1}s",
                t!("task.actions"),
                actions,
                t!("task.optimal"),
                optimal,
                actions - optimal,
                elapsed_secs
            )
        };
        let rest_span = Span::styled(rest_of_stats, Style::default().fg(Color::White));

        let stats = Paragraph::new(Line::from(vec![mode_span, progress_span, rest_span]))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(stats, chunks[3]);

        // Instructions with hint indicator and last command
        let hint_indicator = if state.show_hint_panel && state.current_hint.is_some() {
            " [h: Next Hint] "
        } else {
            " [h: Show Hint] "
        };

        let last_cmd_text = if let Some(cmd) = &state.last_command {
            format!(" Last: {} |", cmd)
        } else {
            String::new()
        };

        let instructions = Paragraph::new(format!(
            "{}{}| Esc: Abandon | Ctrl-c: Quit",
            hint_indicator, last_cmd_text
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[4]);

        // Render hint panel if visible
        if state.show_hint_panel {
            render_hint_popup(frame, state);
        }

        // Show key history popup if visible
        if state.show_key_history {
            render_key_history_popup(frame, state);
        }

        // Show success message if scenario just completed
        if state.completion_time.is_some() {
            render_success_popup(frame);
        }
    }
}
