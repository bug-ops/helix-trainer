//! Task screen rendering

use super::editor::{PreviewHighlight, PreviewType, render_editor_pair};
use super::popups::{render_hint_popup, render_key_history_popup, render_success_popup};
use crate::game::PlayableScenario;
use crate::ui::state::{AppState, TypedScreen};
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
    // Extract TaskData from TypedScreen::Task
    let TypedScreen::Task(task_data) = &state.screen else {
        return; // Wrong screen type
    };

    let area = frame.area();

    {
        let session = &task_data.session;
        let scenario = session.scenario();

        // Check if scenario just completed - use pending session for final state display
        let is_completed = state.ui.completion_time.is_some();

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

        // Title with scenario number for progress tracking
        let title_text = if let Some(index) = task_data.scenario_index {
            let total = state.game.scenario_collection.count();
            format!("Scenario {}/{}: {}", index + 1, total, scenario.name)
        } else {
            format!("Scenario: {}", scenario.name)
        };
        let title = Paragraph::new(title_text)
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

        // Editor view - get current and target states
        // When completed, use pending_completed_session for final state display
        // Use PlayableScenario trait to get states from either Active or Completed session
        let playable: &dyn PlayableScenario = if is_completed {
            if let Some(completed) = &state.game.pending_completed_session {
                completed as &dyn PlayableScenario
            } else {
                session as &dyn PlayableScenario
            }
        } else {
            session as &dyn PlayableScenario
        };

        // Calculate preview highlight for surround operations
        let preview =
            task_data
                .input_state
                .pending_surround_preview()
                .and_then(|surround_preview| {
                    use crate::input::typestate::SurroundPreview;
                    let current_content = playable.current_content();
                    let (cursor_row, cursor_col) = playable.current_cursor();
                    let (bracket_char, preview_type) = match surround_preview {
                        SurroundPreview::Replace(ch) => (ch, PreviewType::Replace),
                        SurroundPreview::Delete(ch) => (ch, PreviewType::Delete),
                    };
                    PreviewHighlight::from_surround_char(
                        &current_content,
                        cursor_row,
                        cursor_col,
                        bracket_char,
                        preview_type,
                    )
                });

        // Render editor pair using shared function
        let current_title = t!("editor.current_state");
        let target_title = t!("editor.target_state");
        render_editor_pair(
            frame,
            chunks[2],
            playable,
            &current_title,
            &target_title,
            preview,
        );

        // Stats with mode indicator and progress
        // Use PlayableScenario trait for common stats, completion_progress is specific to Active
        let optimal = scenario.scoring.optimal_count.get();
        let actions = playable.action_count();
        let elapsed = playable.elapsed();
        let mode = playable.mode_name();
        let progress = if is_completed {
            100u8 // Completed = 100%
        } else {
            session.completion_progress()
        };
        let elapsed_secs = elapsed.as_secs_f32();

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
        let hint_indicator = if task_data.show_hint_panel && task_data.current_hint.is_some() {
            " [?: Next Hint] "
        } else {
            " [?: Hint | F1] "
        };

        let last_cmd_text = if let Some(cmd) = &task_data.last_command {
            format!(" Last: {} |", cmd)
        } else {
            String::new()
        };

        // Live feedback for an in-progress '"'-register selection,
        // ':'-command-line buffer, or 's'/'S' regex-selection prompt, so the
        // user can see what they're typing.
        let pending_text = super::helpers::pending_input_indicator(&task_data.input_state);

        let macro_indicator = if task_data.session.is_recording_macro() {
            " [q: REC] "
        } else {
            ""
        };

        let instructions = Paragraph::new(format!(
            "{}{}{}{}| Ctrl-Q: Abandon | Ctrl-C: Quit",
            pending_text, macro_indicator, hint_indicator, last_cmd_text
        ))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(instructions, chunks[4]);

        // Render hint panel if visible
        if task_data.show_hint_panel {
            render_hint_popup(frame, state);
        }

        // Show key history popup if visible
        // Reserve space for the Stats + Instructions bars (3 + 3) plus the
        // screen's outer margin(1) so the popup never overlaps them.
        if state.ui.show_key_history {
            render_key_history_popup(frame, task_data.key_history.keys(), 7);
        }

        // Show success message if scenario just completed
        if state.ui.completion_time.is_some() {
            render_success_popup(frame);
        }
    }
}
