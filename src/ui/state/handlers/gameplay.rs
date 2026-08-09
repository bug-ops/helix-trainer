//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use std::borrow::Cow;
use std::time::Duration;

use crate::game::format_key_for_display;
use crate::helix::commands::CMD_ESCAPE;
use crate::input::typestate::{HandlerResult, command_to_key_event};
use crate::security::UserError;
use crate::ui::state::{AppState, HandlerOutcome, InputStateAccess, Message, TypedScreen, update};

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
            let feedback = s.feedback().map_err(UserError::from)?;
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
    if let Some(hint) = task_data.session.hint() {
        task_data.current_hint = Some(hint.clone());
        task_data.show_hint_panel = true;
    }
    Ok(HandlerOutcome::Stay)
}

/// Handle ExecuteCommand message
///
/// Processes user commands in normal or insert mode, tracks for quests.
/// Uses the typestate-based `InputStateMachine` for multi-key command handling.
///
/// `keys` is the canonical command (post keymap-overlay translation) to
/// dispatch; `typed` is the physically-pressed key, recorded to
/// `KeyHistory` instead of the translated command so the popup shows what
/// the user actually pressed. A multi-token `keys` (a keymap remap whose
/// target spans more than one canonical key, e.g. `G` -> `"ge"`) is
/// applied atomically via [`InputStateMachine::apply_canonical_expansion`].
///
/// Note: This handler operates on TaskData screen and calls update() for quest progress,
/// so it requires full AppState access
pub fn handle_execute_command(
    state: &mut AppState,
    keys: crate::input::keymap::CanonicalKeys,
    typed: Cow<'static, str>,
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

    // Add key to history for display (format for readability). Uses the
    // physically-typed key, not the translated command - and, matching
    // pre-keymap-overlay behavior, happens for every keystroke
    // unconditionally, including ones that only extend a pending prefix.
    let display_key = format_key_for_display(typed.as_ref());
    task_data.add_key_to_history(display_key);

    // Show key history popup after first keypress
    state.ui.show_key_history = true;

    let is_insert_mode = task_data.session.is_insert_mode();

    // Track command for quest progress (only execute once per complete command)
    let mut executed_command: Option<String> = None;
    let session_completed;

    if is_insert_mode {
        // R1: never translated in insert mode - `keys` is a single token
        // identical to `typed`. Execute directly (bypass input state machine).
        let cmd = keys.into_cow().into_owned();

        // Store last command for display (only for Escape)
        if cmd == CMD_ESCAPE {
            task_data.last_command = Some(cmd.clone());
        }

        // Execute the command
        let scenario = task_data.session.scenario().clone();
        let session = std::mem::replace(
            &mut task_data.session,
            crate::game::GameSession::new(scenario)?,
        );
        let result = session.record_action(cmd)?;
        session_completed = process_session_result(result, &mut task_data, state)?;
        state.screen = TypedScreen::Task(task_data);
    } else {
        // Normal mode - use InputStateMachine for multi-key command handling.
        // Single-token keys (the overwhelmingly common case) are dispatched
        // directly to avoid the `Vec` allocation `tokens()` would incur.
        //
        // An empty `keys` (never produced today - the keymap parser rejects
        // empty bindings at config-load time) would take this branch with
        // an empty `tokens`; `apply_canonical_expansion` handles that by
        // returning `None`, unlike the old `tokens[0]` indexing this
        // replaced, which would have panicked.
        let resolved = if !keys.is_single_token() {
            let tokens = keys.tokens();
            task_data
                .input_state_mut()
                .apply_canonical_expansion(&tokens)
        } else {
            match task_data
                .input_state_mut()
                .process_key(command_to_key_event(keys.as_str()))
            {
                HandlerResult::Execute(cmd) => Some(cmd.to_string()),
                HandlerResult::Transition(_) => {
                    // Waiting for more keys - state machine already updated
                    state.screen = TypedScreen::Task(task_data);
                    return Ok(HandlerOutcome::Stay);
                }
                HandlerResult::Cancel | HandlerResult::Stay => {
                    // Cancelled or unknown key - restore screen and stay
                    state.screen = TypedScreen::Task(task_data);
                    return Ok(HandlerOutcome::Stay);
                }
            }
        };

        let Some(cmd_str) = resolved else {
            // Multi-token expansion didn't resolve cleanly (see
            // `apply_canonical_expansion`'s doc comment) - discard, leave
            // state untouched.
            tracing::warn!(
                keys = keys.as_str(),
                "keymap expansion did not resolve; discarding"
            );
            state.screen = TypedScreen::Task(task_data);
            return Ok(HandlerOutcome::Stay);
        };

        // Store last command for display
        task_data.last_command = Some(cmd_str.clone());
        executed_command = Some(cmd_str.clone());

        // Extract count and base command (e.g., "3h" -> count=3, base_cmd="h")
        let (count, base_cmd) = crate::game::extract_count_and_command(&cmd_str);
        let base_cmd = base_cmd.to_string();

        // Clone scenario before taking session (to avoid borrow conflict)
        let scenario = task_data.session.scenario().clone();

        // Take session for state transition
        let session = std::mem::replace(
            &mut task_data.session,
            crate::game::GameSession::new(scenario)?,
        );

        // Execute command with count - records as ONE action
        let result = session.record_action_with_count(cmd_str, &base_cmd, count)?;
        session_completed = process_session_result(result, &mut task_data, state)?;

        // Always restore Task screen - even when completed, we show success popup
        state.screen = TypedScreen::Task(task_data);
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
