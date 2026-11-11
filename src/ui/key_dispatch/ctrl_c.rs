//! `Ctrl-c` routing
//!
//! `Ctrl-c` quits the application on every non-gameplay screen but is a
//! Helix command (`toggle_comments`) during gameplay, so it can never be a
//! global shortcut. [`message`] decides per screen and, on gameplay screens,
//! per input state, mirroring Helix 25.07.1.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::handlers::{gameplay_input_state, handle_gameplay_key, is_gameplay_insert_mode};
use crate::input::typestate::InputState;
use crate::ui::state::TypedScreen;
use crate::ui::{AppState, Message};

/// What a `Ctrl-c` press does on the current screen and input state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CtrlC {
    /// Quit the application.
    Quit,
    /// Nothing happens.
    Ignore,
    /// The input state machine receives `Ctrl-c` itself (`toggle_comments`).
    Forward,
    /// `c` is typed: a pending command takes it as its argument, and Insert mode inserts it.
    /// Helix ignores the modifier in both.
    AsPlainChar,
    /// The pending state or prompt is cancelled and the key is dropped.
    CancelPending,
}

/// Whether `key` is a `Ctrl-c` press.
pub(crate) fn is_ctrl_c(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// The message a `Ctrl-c` press produces in `state`.
pub(crate) fn message(key: KeyEvent, state: &AppState) -> Option<Message> {
    let plain = |code| KeyEvent::new(code, KeyModifiers::NONE);
    match action(state) {
        CtrlC::Quit => Some(Message::QuitApp),
        CtrlC::Ignore => None,
        CtrlC::Forward => handle_gameplay_key(key, state),
        CtrlC::AsPlainChar => handle_gameplay_key(plain(KeyCode::Char('c')), state),
        CtrlC::CancelPending => handle_gameplay_key(plain(KeyCode::Esc), state),
    }
}

fn action(state: &AppState) -> CtrlC {
    match &state.screen {
        TypedScreen::Task(_) => gameplay_action(state),
        TypedScreen::MiniGame(_) => {
            // ASSUMPTION: countdown, pause and game-over take no editor input; Ctrl-c is dropped there.
            if state
                .minigame_session()
                .is_some_and(|session| session.state().is_playing())
            {
                gameplay_action(state)
            } else {
                CtrlC::Ignore
            }
        }
        TypedScreen::ModeSelection(_)
        | TypedScreen::Menu(_)
        | TypedScreen::Results(_)
        | TypedScreen::Profile(_)
        | TypedScreen::Statistics(_)
        | TypedScreen::Achievements(_)
        | TypedScreen::Review(_)
        | TypedScreen::CategoryFilters(_)
        | TypedScreen::EndGameSummary(_) => CtrlC::Quit,
    }
}

fn gameplay_action(state: &AppState) -> CtrlC {
    // Helix has no Insert-mode `C-c` binding; its fallback types the character.
    if is_gameplay_insert_mode(state) {
        return CtrlC::AsPlainChar;
    }
    gameplay_input_state(state).map_or(CtrlC::Ignore, |machine| pending_action(machine.state()))
}

fn pending_action(input: &InputState) -> CtrlC {
    match input {
        // A register waiting for its command drops out when `toggle_comments` runs.
        InputState::Base
        | InputState::CountPending { .. }
        | InputState::RegisterOpPending { .. } => CtrlC::Forward,
        InputState::FindCharPending { .. }
        | InputState::CountedFindCharPending { .. }
        | InputState::ReplaceCharPending
        | InputState::SurroundAddPending
        | InputState::SurroundDeletePending
        | InputState::SurroundReplaceFromPending
        | InputState::SurroundReplaceToPending { .. }
        | InputState::TextObjectAroundPending
        | InputState::TextObjectInsidePending
        | InputState::RegisterPending => CtrlC::AsPlainChar,
        InputState::GotoPending
        | InputState::CountedGotoPending { .. }
        | InputState::ViewPending
        | InputState::MatchPending
        | InputState::UnmatchedPrevPending
        | InputState::UnmatchedNextPending
        | InputState::CommandLinePending { .. }
        | InputState::RegexPromptPending { .. } => CtrlC::CancelPending,
    }
}
