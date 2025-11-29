//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use crate::security::UserError;
use crate::ui::state::{AppState, Message, TypedScreen, update};
use std::time::Duration;

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings
fn format_key_for_display(command: &str) -> String {
    match command {
        "Left" => "←".to_string(),
        "Right" => "→".to_string(),
        "Up" => "↑".to_string(),
        "Down" => "↓".to_string(),
        "Backspace" => "⌫".to_string(),
        "Escape" => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
    }
}

/// Handle ShowHint message
///
/// Toggles hint panel visibility and fetches next hint
pub fn handle_show_hint(state: &mut AppState) -> Result<(), UserError> {
    // Only handle if we're on Task screen
    let TypedScreen::Task(task_data) = &mut state.screen else {
        return Ok(()); // Not on task screen
    };

    // If hint panel is already visible, close it (toggle behavior)
    if task_data.show_hint_panel {
        task_data.show_hint_panel = false;
        task_data.current_hint = None;
        return Ok(());
    }

    // Otherwise, try to show next hint
    if let Some(hint) = task_data.session.get_hint() {
        task_data.current_hint = Some(hint.clone());
        task_data.show_hint_panel = true;
    }
    Ok(())
}

/// Handle ExecuteCommand message
///
/// Processes user commands in normal or insert mode, tracks for quests
pub fn handle_execute_command(
    state: &mut AppState,
    command: std::borrow::Cow<'static, str>,
) -> Result<(), UserError> {
    use crate::game::SessionAfterAction;
    use crate::ui::state::ResultsData;

    // Only handle if we're on Task screen
    if !matches!(state.screen, TypedScreen::Task(_)) {
        return Ok(()); // Not on task screen
    }

    // Temporarily replace screen with a placeholder to get ownership
    // We'll replace it back at the end
    let placeholder = TypedScreen::Menu(crate::ui::state::MenuData::default());
    let old_screen = std::mem::replace(&mut state.screen, placeholder);

    let TypedScreen::Task(mut task_data) = old_screen else {
        unreachable!("Already checked above")
    };

    // Add key to history for display (format for readability)
    let display_key = format_key_for_display(command.as_ref());
    task_data.add_key_to_history(display_key);

    // Show key history popup after first keypress
    state.ui.show_key_history = true;

    // Track command for quest progress (only execute once per complete command)
    let mut executed_command: Option<String> = None;
    let mut session_completed = false;

    // Take ownership of session for state transition
    let session = task_data.session;

    // In Insert mode, execute commands directly
    if session.is_insert_mode() {
        // Store last command for display (skip special commands and single chars)
        if command.as_ref() == "Escape" {
            task_data.last_command = Some(command.to_string());
        }

        // Execute command through session (consumes session)
        match session.record_action(command.to_string())? {
            SessionAfterAction::StillActive(s) => {
                // Session still active, put it back in task_data
                task_data.session = s;
                // Restore Task screen
                state.screen = TypedScreen::Task(task_data);
            }
            SessionAfterAction::Completed(s) => {
                // Session completed, get feedback and mark completion time
                let feedback = s.feedback().map_err(|_| UserError::OperationFailed)?;
                state.ui.last_feedback = Some(feedback.clone());
                state.ui.completion_time = Some(std::time::Instant::now());
                session_completed = true;

                // Transition to Results screen
                state.screen = TypedScreen::Results(ResultsData::from_completed(s, feedback)?);
            }
        }
    } else {
        // Normal mode: handle command buffer for multi-key commands
        task_data.command_buffer.push_str(&command);

        // Try to match a complete command
        let final_command = match task_data.command_buffer.as_str() {
            // Multi-key commands
            "dd" => Some("dd"),
            "gg" => Some("gg"),

            // Replace character command: r + any char
            cmd if cmd.starts_with('r') && cmd.len() == 2 => {
                Some(task_data.command_buffer.as_str())
            }

            // Partial commands - wait for more input
            "d" | "g" | "r" => None,

            // Single-key commands (clear buffer and execute)
            _ if task_data.command_buffer.len() == 1 => Some(task_data.command_buffer.as_str()),

            // Invalid sequence - clear buffer
            _ => {
                task_data.command_buffer.clear();
                // Put task_data back with session
                task_data.session = session;
                state.screen = TypedScreen::Task(task_data);
                return Ok(());
            }
        };

        if let Some(cmd) = final_command {
            // We have a complete command
            let cmd_string = cmd.to_string();
            task_data.command_buffer.clear();

            // Store for display
            task_data.last_command = Some(cmd_string.clone());

            // Track for quest progress
            executed_command = Some(cmd_string.clone());

            // Execute command through session (consumes session)
            match session.record_action(cmd_string)? {
                SessionAfterAction::StillActive(s) => {
                    // Session still active, put it back
                    task_data.session = s;
                    state.screen = TypedScreen::Task(task_data);
                }
                SessionAfterAction::Completed(s) => {
                    // Session completed, get feedback and mark completion time
                    let feedback = s.feedback().map_err(|_| UserError::OperationFailed)?;
                    state.ui.last_feedback = Some(feedback.clone());
                    state.ui.completion_time = Some(std::time::Instant::now());
                    session_completed = true;

                    // Transition to Results screen
                    state.screen = TypedScreen::Results(ResultsData::from_completed(s, feedback)?);
                }
            }
        } else {
            // Waiting for more keys, put session back
            task_data.session = session;
            state.screen = TypedScreen::Task(task_data);
        }
    }

    // Update quest progress for executed command (after releasing session borrow)
    if let Some(cmd) = executed_command {
        update(
            state,
            Message::UpdateQuestProgress {
                command: Some(cmd),
                scenario_completed: session_completed,
                duration: Duration::from_secs(0),
            },
        )?;
    }

    Ok(())
}
