//! Gameplay interaction message handlers
//!
//! Handles command execution and hint display

use std::time::Duration;

use crate::game::format_key_for_display;
use crate::grammar::commands::CMD_ESCAPE;
use crate::input::resolve::{Resolution, resolve_keys};
use crate::security::{UserError, error_chain};
use crate::ui::state::{
    AppState, HandlerOutcome, HintPanel, InputStateAccess, Message, TypedScreen, dispatch,
};

/// Handle `ShowHint` message
///
/// Toggles hint panel visibility and fetches next hint
///
/// Note: This handler operates on `TaskData` screen, so it uses `AppState` to access the screen
pub fn handle_show_hint(state: &mut AppState) -> HandlerOutcome {
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
pub fn handle_execute_command(
    state: &mut AppState,
    keys: &crate::input::keymap::CanonicalKeys,
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

    // `ensure_scorable` ran while the session was still active, so finishing cannot fail
    // for a scenario that reached here; should it, the placeholder screen stays and the
    // error surfaces.
    let task_data = match task_data.finish_action(state.progress.monotonic_now()) {
        Ok(task_data) => task_data,
        Err(e) => {
            tracing::error!(error = %error_chain(&e), "Failed to build the scenario feedback");
            return Err(UserError::from(e));
        }
    };
    let completed = task_data.session.completed().is_some();
    state.screen = TypedScreen::Task(task_data);

    // Update quest progress for executed command (after releasing session borrow)
    if let Some(executed) = executed {
        dispatch(
            state,
            Message::UpdateQuestProgress {
                command: Some(executed.command),
                scenario_completed: completed,
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
}

/// Run one resolved key against the Task screen's session.
///
/// Returns the executed normal-mode command (if any) for quest tracking. On
/// `Err` the session is left as it was before the failing action.
fn execute_on_task(
    state: &mut AppState,
    task_data: &mut crate::ui::state::TaskData,
    keys: &crate::input::keymap::CanonicalKeys,
    typed: &str,
) -> Result<Option<ExecutedCommand>, UserError> {
    // A finished task shows its success animation; it takes no more input
    if task_data.session.active().is_none() {
        return Ok(None);
    }

    // Add key to history for display (format for readability). Uses the
    // physically-typed key, not the translated command - and, matching
    // pre-keymap-overlay behavior, happens for every keystroke
    // unconditionally, including ones that only extend a pending prefix.
    let display_key = format_key_for_display(typed);
    task_data.add_key_to_history(display_key);

    // Show key history popup after first keypress
    state.ui.show_key_history = true;

    let mode = task_data.session.playable().mode_kind();
    match resolve_keys(task_data.input_state_mut(), mode, keys) {
        Resolution::Insert(cmd) => {
            let is_escape = cmd == CMD_ESCAPE;
            apply_to_session(task_data, cmd.into_owned())?;

            // Store last command for display (only for Escape), once it was accepted
            if is_escape {
                task_data.last_command = Some(CMD_ESCAPE.to_string());
            }
            Ok(None)
        }
        Resolution::Execute { command, .. } => {
            // The simulator applies any count prefix; records as ONE action
            apply_typed_to_session(task_data, &command)?;

            // Store last command for display only once the simulator accepted it
            let command = command.to_string();
            task_data.last_command = Some(command.clone());

            Ok(Some(ExecutedCommand { command }))
        }
        Resolution::Pending | Resolution::Discarded => Ok(None),
    }
}

/// Check that a finished scenario with `action_count` actions can be scored.
fn ensure_scorable(
    scoring: &crate::config::ScoringConfig,
    action_count: usize,
) -> Result<(), UserError> {
    crate::game::Scorer::score_with_config(scoring, action_count)
        .map(drop)
        .map_err(UserError::from)
}

/// Apply the parsed `command` to the Task screen's active session.
fn apply_typed_to_session(
    task_data: &mut crate::ui::state::TaskData,
    command: &crate::helix::normal_command::NormalCommand,
) -> Result<(), UserError> {
    apply_with(task_data, |session| session.apply_command(command))
}

/// Apply `cmd` to the Task screen's active session.
///
/// The failing step (`apply_action`) borrows the session, so an error leaves it in place.
fn apply_to_session(
    task_data: &mut crate::ui::state::TaskData,
    cmd: String,
) -> Result<(), UserError> {
    apply_with(task_data, |session| session.apply_action(cmd))
}

fn apply_with(
    task_data: &mut crate::ui::state::TaskData,
    apply: impl FnOnce(&mut crate::game::GameSession<crate::game::Active>) -> Result<(), UserError>,
) -> Result<(), UserError> {
    let Some(session) = task_data.session.active_mut() else {
        return Ok(());
    };
    apply(session)?;

    // Everything that can fail when finishing a scenario (the score, hence the feedback) is
    // checked while the session is still in place, so the transition to Completed cannot fail
    // halfway and strand the player.
    if session.check_completion() {
        ensure_scorable(&session.scenario().scoring, session.action_count())?;
    }
    Ok(())
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

        let result = apply_to_session(&mut task_data, "not_a_command".to_string());

        assert!(result.is_err());
        assert_eq!(task_data.session.playable().action_count(), 0);
    }

    /// Regression test for #525: a scenario that cannot be scored is rejected while the
    /// session is still active, instead of after it was consumed by the Completed transition.
    #[test]
    fn test_ensure_scorable_rejects_unscorable_completion() {
        use crate::config::ScoringConfig;
        use std::num::NonZeroUsize;

        let scoring = ScoringConfig {
            optimal_count: NonZeroUsize::new(1 << 40).unwrap(),
            max_points: u32::MAX,
            tolerance: 0,
        };

        assert!(ensure_scorable(&scoring, (1 << 40) + 1).is_err());
        assert!(ensure_scorable(&scoring, 1).is_ok());
    }

    #[test]
    fn test_execute_command_error_restores_task_screen() {
        let mut state = task_state();
        // `a(` is an invalid search regex: Enter is rejected by the simulator.
        for c in "/a(".chars() {
            let key = crate::input::keymap::CanonicalKeys::from_owned(c.to_string());
            handle_execute_command(&mut state, &key, &c.to_string()).unwrap();
        }
        let enter = crate::input::keymap::CanonicalKeys::from_static("Enter");
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
