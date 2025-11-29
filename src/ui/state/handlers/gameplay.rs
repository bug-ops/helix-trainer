//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use crate::security::UserError;
use crate::ui::state::{AppState, Message, update};
use std::time::Duration;

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings
fn format_key_for_display(command: &str) -> String {
    match command {
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
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
    // If hint panel is already visible, close it (toggle behavior)
    if state.ui.show_hint_panel {
        state.ui.show_hint_panel = false;
        state.ui.current_hint = None;
        return Ok(());
    }

    // Otherwise, try to show next hint
    if let Some(session) = &mut state.game.session
        && let Some(hint) = session.get_hint()
    {
        state.ui.current_hint = Some(hint.clone());
        state.ui.show_hint_panel = true;
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
    // Add key to history for display (format for readability)
    let display_key = format_key_for_display(command.as_ref());
    state.add_key_to_history(display_key);

    // Show key history popup after first keypress
    state.ui.show_key_history = true;

    // Track command for quest progress (only execute once per complete command)
    let mut executed_command: Option<String> = None;

    if let Some(session) = &mut state.game.session {
        // In Insert mode, execute commands directly
        if session.is_insert_mode() {
            // Store last command for display (skip special commands and single chars)
            if command.as_ref() == "Escape" {
                state.ui.last_command = Some(command.to_string());
            }

            // Execute command through session
            session.record_action(command.to_string())?;
        } else {
            // Normal mode: handle command buffer for multi-key commands
            state.ui.command_buffer.push_str(&command);

            // Try to match a complete command
            let final_command = match state.ui.command_buffer.as_str() {
                // Multi-key commands
                "dd" => Some("dd"),
                "gg" => Some("gg"),

                // Replace character command: r + any char
                cmd if cmd.starts_with('r') && cmd.len() == 2 => {
                    Some(state.ui.command_buffer.as_str())
                }

                // Partial commands - wait for more input
                "d" | "g" | "r" => None,

                // Single-key commands (clear buffer and execute)
                _ if state.ui.command_buffer.len() == 1 => Some(state.ui.command_buffer.as_str()),

                // Invalid sequence - clear buffer
                _ => {
                    state.ui.command_buffer.clear();
                    return Ok(());
                }
            };

            if let Some(cmd) = final_command {
                // We have a complete command
                let cmd_string = cmd.to_string();
                state.ui.command_buffer.clear();

                // Store for display
                state.ui.last_command = Some(cmd_string.clone());

                // Track for quest progress
                executed_command = Some(cmd_string.clone());

                // Execute command through session
                session.record_action(cmd_string)?;
            }
            // If None, we're waiting for more keys (buffer not cleared)
        }
    }

    // Update quest progress for executed command (after releasing session borrow)
    if let Some(cmd) = executed_command {
        update(
            state,
            Message::UpdateQuestProgress {
                command: Some(cmd),
                scenario_completed: false,
                duration: Duration::from_secs(0),
            },
        )?;
    }

    // Check if scenario is complete
    if let Some(session) = &state.game.session
        && session.is_completed()
    {
        // Mark completion time instead of immediately going to results
        // This allows showing the success state before transition
        state.ui.completion_time = Some(std::time::Instant::now());
    }

    Ok(())
}
