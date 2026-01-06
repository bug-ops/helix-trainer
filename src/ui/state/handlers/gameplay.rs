//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use crate::game::format_key_for_display;
use crate::helix::commands::CMD_ESCAPE;
use crate::security::UserError;
use crate::ui::state::{AppState, HandlerOutcome, Message, TypedScreen, update};
use std::time::Duration;

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
///
/// Note: This handler operates on TaskData screen, so it uses AppState to access the screen
pub fn handle_show_hint(state: &mut AppState) -> Result<HandlerOutcome, UserError> {
    // Only handle if we're on Task screen
    let TypedScreen::Task(task_data) = &mut state.screen else {
        return Ok(HandlerOutcome::Stay); // Not on task screen
    };

    // If hint panel is already visible, close it (toggle behavior)
    if task_data.show_hint_panel {
        task_data.show_hint_panel = false;
        task_data.current_hint = None;
        return Ok(HandlerOutcome::Stay);
    }

    // Otherwise, try to show next hint
    if let Some(hint) = task_data.session.get_hint() {
        task_data.current_hint = Some(hint.clone());
        task_data.show_hint_panel = true;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle ExecuteCommand message
///
/// Processes user commands in normal or insert mode, tracks for quests
///
/// Note: This handler operates on TaskData screen and calls update() for quest progress,
/// so it requires full AppState access
pub fn handle_execute_command(
    state: &mut AppState,
    command: std::borrow::Cow<'static, str>,
) -> Result<HandlerOutcome, UserError> {
    // Only handle if we're on Task screen
    if !matches!(state.screen, TypedScreen::Task(_)) {
        return Ok(HandlerOutcome::Stay); // Not on task screen
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

    // Use unified command input processing
    use crate::game::{CommandInputResult, process_command_input};

    let is_insert_mode = task_data.session.is_insert_mode();
    let input_result = process_command_input(&mut task_data, &command, is_insert_mode);

    // Track command for quest progress (only execute once per complete command)
    let mut executed_command: Option<String> = None;
    let mut session_completed = false;

    match input_result {
        CommandInputResult::Execute(cmd) => {
            // Store last command for display
            if is_insert_mode {
                if command.as_ref() == CMD_ESCAPE {
                    task_data.last_command = Some(command.to_string());
                }
            } else {
                task_data.last_command = Some(cmd.clone());
                executed_command = Some(cmd.clone());
            }

            // Extract count and base command (e.g., "3h" -> count=3, base_cmd="h")
            let (count, base_cmd) = crate::game::extract_count_and_command(&cmd);
            let base_cmd = base_cmd.to_string();

            // Clone scenario before taking session (to avoid borrow conflict)
            let scenario = task_data.session.scenario().clone();

            // Take session for state transition
            let session = std::mem::replace(
                &mut task_data.session,
                crate::game::GameSession::new(scenario)?,
            );

            // Execute command with count - records as ONE action
            let result = session.record_action_with_count(cmd, &base_cmd, count)?;
            session_completed = process_session_result(result, &mut task_data, state)?;

            // Always restore Task screen - even when completed, we show success popup
            state.screen = TypedScreen::Task(task_data);
        }
        CommandInputResult::Invalid => {
            // Invalid sequence - buffer already cleared by process_command_input
            state.screen = TypedScreen::Task(task_data);
            return Ok(HandlerOutcome::Stay);
        }
        CommandInputResult::Partial => {
            // Waiting for more keys
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
                scenario_id: None,
                duration: Duration::from_secs(0),
            },
        )?;
    }

    Ok(HandlerOutcome::Stay)
}
