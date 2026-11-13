//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use super::SessionInput;
use super::key_history;
use super::quests::quest_command;
use crate::error::UserError;
use crate::grammar::commands::CMD_ESCAPE;
use crate::helix::InsertInput;
use crate::ids::NormalizedCommand;
use crate::input::resolve::{Resolution, resolve_keys};
use crate::ui::state::{
    AppState, HandlerOutcome, HintPanel, InputStateAccess, Message, QuestEvent, TypedScreen,
    dispatch,
};

/// Handle `ShowHint` message
///
/// Toggles hint panel visibility and fetches next hint
///
/// Note: This handler operates on `TaskData` screen, so it uses `AppState` to access the screen
pub(crate) fn handle_show_hint(state: &mut AppState) -> HandlerOutcome {
    // Only handle if we're on Task screen
    let TypedScreen::Task(task_data) = &mut state.screen else {
        return HandlerOutcome::Stay; // Not on task screen
    };

    // If hint panel is already visible, close it (toggle behavior)
    if task_data.hint.is_shown() {
        task_data.hint = HintPanel::Hidden;
        return HandlerOutcome::Stay;
    }

    // Otherwise, try to show next hint (a finished task has none to give)
    if let Some(hint) = task_data
        .session
        .active_mut()
        .and_then(crate::game::GameSession::hint)
    {
        task_data.hint = HintPanel::Shown(hint);
    }
    HandlerOutcome::Stay
}

/// Handle `ExecuteCommand` message
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
/// Note: This handler operates on `TaskData` screen and calls `update()` for quest progress,
/// so it requires full `AppState` access
pub(crate) fn handle_execute_command(
    state: &mut AppState,
    keys: &crate::input::keymap::GameplayKey,
    typed: &str,
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
    let executed = match execute_on_task(state, &mut task_data, keys, typed) {
        Ok(executed) => executed,
        Err(e) => {
            state.screen = TypedScreen::Task(task_data);
            return Err(e);
        }
    };

    state.screen = TypedScreen::Task(task_data.finish_action(state.progress.monotonic_now()));

    // Update quest progress for executed command (after releasing session borrow)
    if let Some(executed) = executed {
        dispatch(
            state,
            Message::UpdateQuestProgress(QuestEvent::CommandExecuted(executed.command)),
        )?;
    }

    Ok(HandlerOutcome::Stay)
}

/// A normal-mode command that was executed, for quest tracking.
struct ExecutedCommand {
    command: NormalizedCommand,
}

/// Run one resolved key against the Task screen's session.
///
/// Returns the executed normal-mode command (if any) for quest tracking. On
/// `Err` the session is left as it was before the failing action.
fn execute_on_task(
    state: &mut AppState,
    task_data: &mut crate::ui::state::TaskData,
    keys: &crate::input::keymap::GameplayKey,
    typed: &str,
) -> Result<Option<ExecutedCommand>, UserError> {
    // A finished task shows its success animation; it takes no more input
    if task_data.session.active().is_none() {
        return Ok(None);
    }

    key_history::record_key(&mut state.ui, &mut task_data.key_history, typed);

    let mode = task_data.session.playable().mode_kind();
    match resolve_keys(task_data.input_state_mut(), mode, keys) {
        Resolution::Insert(input) => {
            apply_to_session(task_data, SessionInput::Insert(&input))?;

            // Store last command for display (only for Escape), once it was accepted
            if input == InsertInput::Escape {
                task_data.last_command = Some(CMD_ESCAPE.to_string());
            }
            Ok(None)
        }
        Resolution::Execute { command, .. } => {
            // The simulator applies any count prefix; records as ONE action
            apply_to_session(task_data, SessionInput::Command(&command))?;

            // Store last command for display only once the simulator accepted it
            task_data.last_command = Some(command.to_string());

            Ok(quest_command(&command).map(|command| ExecutedCommand { command }))
        }
        Resolution::Pending | Resolution::Discarded => Ok(None),
    }
}

/// Apply `input` to the Task screen's active session.
///
/// The failing step borrows the session, so an error leaves it in place.
fn apply_to_session(
    task_data: &mut crate::ui::state::TaskData,
    input: SessionInput<'_>,
) -> Result<(), UserError> {
    apply_with(task_data, |session| {
        match input {
            SessionInput::Insert(input) => session.apply_insert(input),
            SessionInput::Command(command) => session.apply_command(command),
        }
        .map_err(UserError::from)
    })
}

fn apply_with(
    task_data: &mut crate::ui::state::TaskData,
    apply: impl FnOnce(&mut crate::game::GameSession<crate::game::Active>) -> Result<(), UserError>,
) -> Result<(), UserError> {
    let Some(session) = task_data.session.active_mut() else {
        return Ok(());
    };
    apply(session)
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

        let result = apply_with(&mut task_data, |session| {
            session
                .apply_action("not_a_command")
                .map_err(UserError::from)
        });

        assert!(result.is_err());
        assert_eq!(task_data.session.playable().action_count(), 0);
    }

    #[test]
    fn test_execute_command_error_restores_task_screen() {
        let mut state = task_state();
        // `a(` is an invalid search regex: Enter is rejected by the simulator.
        for c in "/a(".chars() {
            let key = crate::input::keymap::GameplayKey::Keys(crate::testing::keys(&c.to_string()));
            handle_execute_command(&mut state, &key, &c.to_string()).unwrap();
        }
        let enter = crate::input::keymap::GameplayKey::Keys(crate::testing::keys("Enter"));
        let result = handle_execute_command(&mut state, &enter, "Enter");

        assert!(result.is_err());
        let TypedScreen::Task(task_data) = &state.screen else {
            panic!("player was thrown off the Task screen");
        };
        assert_eq!(task_data.session.playable().action_count(), 0);
        assert!(
            task_data.last_command.is_none(),
            "a rejected command must not be shown as executed"
        );
    }
}
