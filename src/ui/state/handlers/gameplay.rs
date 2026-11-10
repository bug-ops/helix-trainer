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
            let feedback = match s.feedback() {
                Ok(feedback) => feedback,
                Err(e) => {
                    // Keep the completed session rather than dropping the
                    // player's progress with it.
                    state.game.pending_completed_session = Some(s);
                    return Err(UserError::from(e));
                }
            };
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
    let placeholder = TypedScreen::Menu(crate::ui::state::MenuData::default());
    let old_screen = std::mem::replace(&mut state.screen, placeholder);

    let TypedScreen::Task(mut task_data) = old_screen else {
        unreachable!("Already checked above")
    };

    // The Task screen is restored whatever the outcome, so a recoverable
    // error (e.g. the simulator rejecting a command) never costs the player
    // their session.
    let outcome = execute_on_task(state, &mut task_data, keys, typed);
    state.screen = TypedScreen::Task(task_data);

    // Update quest progress for executed command (after releasing session borrow)
    if let Some(executed) = outcome? {
        update(
            state,
            Message::UpdateQuestProgress {
                command: Some(executed.command),
                scenario_completed: executed.completed,
                scenario_id: None,
                duration: Duration::from_secs(0),
            },
        )?;
    }

    Ok(HandlerOutcome::Stay)
}

/// A normal-mode command that was executed, for quest tracking.
struct ExecutedCommand {
    command: String,
    completed: bool,
}

/// Run one resolved key against the Task screen's session.
///
/// Returns the executed normal-mode command (if any) for quest tracking. On
/// `Err` the session is left as it was before the failing action.
fn execute_on_task(
    state: &mut AppState,
    task_data: &mut crate::ui::state::TaskData,
    keys: crate::input::keymap::CanonicalKeys,
    typed: Cow<'static, str>,
) -> Result<Option<ExecutedCommand>, UserError> {
    // Add key to history for display (format for readability). Uses the
    // physically-typed key, not the translated command - and, matching
    // pre-keymap-overlay behavior, happens for every keystroke
    // unconditionally, including ones that only extend a pending prefix.
    let display_key = format_key_for_display(typed.as_ref());
    task_data.add_key_to_history(display_key);

    // Show key history popup after first keypress
    state.ui.show_key_history = true;

    let is_insert_mode = task_data.session.is_insert_mode();

    if is_insert_mode {
        // R1: never translated in insert mode - `keys` is a single token
        // identical to `typed`. Execute directly (bypass input state machine).
        let cmd = keys.into_cow().into_owned();

        let is_escape = cmd == CMD_ESCAPE;
        apply_to_session(state, task_data, cmd)?;

        // Store last command for display (only for Escape), once it was accepted
        if is_escape {
            task_data.last_command = Some(CMD_ESCAPE.to_string());
        }
        Ok(None)
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
                // Waiting for more keys (state machine already updated), or
                // cancelled / unknown key
                HandlerResult::Transition(_) | HandlerResult::Cancel | HandlerResult::Stay => {
                    return Ok(None);
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
            return Ok(None);
        };

        // The simulator applies any count prefix; records as ONE action
        let completed = apply_to_session(state, task_data, cmd_str.clone())?;

        // Store last command for display only once the simulator accepted it
        task_data.last_command = Some(cmd_str.clone());

        Ok(Some(ExecutedCommand {
            command: cmd_str,
            completed,
        }))
    }
}

/// Apply `cmd` to the Task screen's session and route the result.
///
/// The failing step (`apply_action`) borrows the session, so an error leaves
/// it in place. Returns whether the scenario completed.
fn apply_to_session(
    state: &mut AppState,
    task_data: &mut crate::ui::state::TaskData,
    cmd: String,
) -> Result<bool, UserError> {
    task_data.session.apply_action(cmd)?;

    let scenario = task_data.session.scenario().clone();
    let session = std::mem::replace(
        &mut task_data.session,
        crate::game::GameSession::new(scenario)?,
    );
    process_session_result(session.finish_action(), task_data, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::GameSession;
    use crate::testing::{default_test_scenario, empty_test_app_state};
    use crate::ui::state::TaskData;

    fn task_state() -> AppState {
        let mut state = empty_test_app_state();
        let session = GameSession::new(default_test_scenario()).unwrap();
        state.screen = TypedScreen::Task(TaskData::new(session));
        state
    }

    /// Regression test for #466: a simulator error must leave the session in
    /// place with its action count unchanged.
    #[test]
    fn test_rejected_command_keeps_session_and_action_count() {
        let mut state = task_state();
        let TypedScreen::Task(mut task_data) = std::mem::replace(
            &mut state.screen,
            TypedScreen::Menu(crate::ui::state::MenuData::default()),
        ) else {
            unreachable!()
        };

        let result = apply_to_session(&mut state, &mut task_data, "not_a_command".to_string());

        assert!(result.is_err());
        assert_eq!(task_data.session.action_count(), 0);
    }

    #[test]
    fn test_execute_command_error_restores_task_screen() {
        let mut state = task_state();
        // `a(` is an invalid search regex: Enter is rejected by the simulator.
        for c in "/a(".chars() {
            let key = crate::input::keymap::CanonicalKeys::from_owned(c.to_string());
            handle_execute_command(&mut state, key, Cow::Owned(c.to_string())).unwrap();
        }
        let enter = crate::input::keymap::CanonicalKeys::from_static("Enter");
        let result = handle_execute_command(&mut state, enter, Cow::Borrowed("Enter"));

        assert!(result.is_err());
        let TypedScreen::Task(task_data) = &state.screen else {
            panic!("player was thrown off the Task screen");
        };
        assert_eq!(task_data.session.action_count(), 0);
        assert!(
            task_data.last_command.is_none(),
            "a rejected command must not be shown as executed"
        );
    }
}
