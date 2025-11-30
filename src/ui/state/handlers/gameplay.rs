//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use crate::helix::commands::*;
use crate::security::UserError;
use crate::ui::state::{AppState, Message, TypedScreen, update};
use std::time::Duration;

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings
fn format_key_for_display(command: &str) -> String {
    match command {
        CMD_ARROW_LEFT => "←".to_string(),
        CMD_ARROW_RIGHT => "→".to_string(),
        CMD_ARROW_UP => "↑".to_string(),
        CMD_ARROW_DOWN => "↓".to_string(),
        CMD_BACKSPACE => "⌫".to_string(),
        CMD_ESCAPE => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
    }
}

/// Parse command buffer to determine if a complete command is available
///
/// Returns Some(command) if buffer contains a complete command, None if waiting for more keys
fn parse_command_buffer(buffer: &str) -> Option<&str> {
    match buffer {
        // Multi-key commands
        CMD_DELETE_LINE => Some(CMD_DELETE_LINE),
        CMD_GOTO_FILE_START => Some(CMD_GOTO_FILE_START),

        // Replace character command: r + any char
        cmd if cmd.starts_with('r') && cmd.len() == 2 => Some(buffer),

        // Partial commands - wait for more input
        "d" | "g" | CMD_REPLACE => None,

        // Single-key commands
        _ if buffer.len() == 1 => Some(buffer),

        // Invalid sequence - return empty to signal clear
        _ => Some(""),
    }
}

/// Process session result and update state accordingly
///
/// Returns whether session was completed
fn process_session_result(
    session_result: crate::game::SessionAfterAction,
    task_data: &mut crate::ui::state::TaskData,
    state: &mut AppState,
) -> Result<bool, UserError> {
    use crate::game::SessionAfterAction;

    match session_result {
        SessionAfterAction::StillActive(s) => {
            task_data.session = s;
            Ok(false)
        }
        SessionAfterAction::Completed(s) => {
            let feedback = s.feedback().map_err(|_| UserError::OperationFailed)?;
            state.ui.last_feedback = Some(feedback.clone());
            // Start success animation - keep Task screen, just mark completion time
            // The event loop will transition to Results after 1.5s delay
            state.ui.completion_time = Some(std::time::Instant::now());
            // Store completed session for later transition by CompleteScenario handler
            state.game.pending_completed_session = Some(s);
            Ok(true)
        }
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

    // In Insert mode, execute commands directly
    if task_data.session.is_insert_mode() {
        // Store last command for display (skip special commands and single chars)
        if command.as_ref() == CMD_ESCAPE {
            task_data.last_command = Some(command.to_string());
        }

        // Clone scenario before taking session (to avoid borrow conflict)
        let scenario = task_data.session.scenario().clone();

        // Take session for state transition
        let session = std::mem::replace(
            &mut task_data.session,
            crate::game::GameSession::new(scenario)?,
        );

        // Execute command and process result
        let result = session.record_action(command.to_string())?;
        session_completed = process_session_result(result, &mut task_data, state)?;

        // Always restore Task screen - even when completed, we show success popup
        state.screen = TypedScreen::Task(task_data);
    } else {
        // Normal mode: handle command buffer for multi-key commands
        task_data.command_buffer.push_str(&command);

        // Try to match a complete command
        let final_command = parse_command_buffer(&task_data.command_buffer);

        match final_command {
            Some("") => {
                // Invalid sequence - clear buffer and restore state
                task_data.command_buffer.clear();
                state.screen = TypedScreen::Task(task_data);
                return Ok(());
            }
            Some(cmd) => {
                // Complete command - execute it
                let cmd_string = cmd.to_string();
                task_data.command_buffer.clear();
                task_data.last_command = Some(cmd_string.clone());
                executed_command = Some(cmd_string.clone());

                // Clone scenario before taking session (to avoid borrow conflict)
                let scenario = task_data.session.scenario().clone();

                // Take session for state transition
                let session = std::mem::replace(
                    &mut task_data.session,
                    crate::game::GameSession::new(scenario)?,
                );

                let result = session.record_action(cmd_string)?;
                session_completed = process_session_result(result, &mut task_data, state)?;

                // Always restore Task screen - even when completed, we show success popup
                state.screen = TypedScreen::Task(task_data);
            }
            None => {
                // Waiting for more keys
                state.screen = TypedScreen::Task(task_data);
            }
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
