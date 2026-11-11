//! `BaseState` handler - handles input when no prefix is active
//!
//! This is the default state handler that processes single-key commands and
//! transitions to pending states for multi-key sequences.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::{CMD_ESCAPE, CMD_REPLAY_MACRO, CMD_TOGGLE_MACRO_RECORDING};

use super::count::count_from_digit;
use super::{InputHandler, KeyHandler};
use crate::input::typestate::{
    handler_result::HandlerResult,
    input_state::{InputState, RegexPromptKind},
    key_mapping::{Chord, ctrl_command, map_single_key_command, normalize_key_event},
    state_types::{BaseState, FindType},
};

fn execute(cmd: &'static str) -> HandlerResult {
    HandlerResult::Execute(Cow::Borrowed(cmd))
}

fn regex_prompt(kind: RegexPromptKind) -> HandlerResult {
    HandlerResult::Transition(InputState::RegexPromptPending {
        kind,
        buffer: String::new(),
    })
}

fn ctrl_key(code: KeyCode) -> HandlerResult {
    match code {
        KeyCode::Char(c) => ctrl_command(c).map_or(HandlerResult::Stay, execute),
        _ => HandlerResult::Stay,
    }
}

/// Alt chords never open a plain-key pending state: they run the Alt-bound
/// command or are ignored (#487). `Alt-K` is the one chord that opens a prompt.
fn alt_key(code: KeyCode, modifiers: KeyModifiers) -> HandlerResult {
    match code {
        KeyCode::Char('K') => regex_prompt(RegexPromptKind::RemoveSelections),
        KeyCode::Char(c) => {
            map_single_key_command(c, modifiers).map_or(HandlerResult::Stay, execute)
        }
        KeyCode::Esc => execute(CMD_ESCAPE),
        _ => HandlerResult::Stay,
    }
}

fn plain_key(code: KeyCode, modifiers: KeyModifiers) -> HandlerResult {
    match code {
        // Prefix commands - transition to pending states
        KeyCode::Char('g') => HandlerResult::Transition(InputState::GotoPending),
        KeyCode::Char('z') => HandlerResult::Transition(InputState::ViewPending),
        KeyCode::Char('m') => HandlerResult::Transition(InputState::MatchPending),
        KeyCode::Char('[') => HandlerResult::Transition(InputState::UnmatchedPrevPending),
        KeyCode::Char(']') => HandlerResult::Transition(InputState::UnmatchedNextPending),

        // Find/till commands - transition to find char pending
        KeyCode::Char(c) if let Some(find_type) = FindType::from_key(c) => {
            HandlerResult::Transition(InputState::FindCharPending { find_type })
        }

        KeyCode::Char('r') => HandlerResult::Transition(InputState::ReplaceCharPending),
        KeyCode::Char('"') => HandlerResult::Transition(InputState::RegisterPending),

        KeyCode::Char(':') => HandlerResult::Transition(InputState::CommandLinePending {
            buffer: String::new(),
        }),

        // Regex prompts: `s`/`S` select/split, `/` search
        KeyCode::Char('s') => regex_prompt(RegexPromptKind::SelectRegex),
        KeyCode::Char('S') => regex_prompt(RegexPromptKind::SplitSelection),
        KeyCode::Char('/') => regex_prompt(RegexPromptKind::Search),
        KeyCode::Char('?') => regex_prompt(RegexPromptKind::SearchBackward),
        KeyCode::Char('K') => regex_prompt(RegexPromptKind::KeepSelections),

        // Macro recording ('q') and replay ('Q'). This handler only runs
        // in Normal mode (Insert mode input bypasses the state machine
        // entirely - see `handle_insert_mode_input`), so these always
        // mean "toggle recording" / "replay macro" here; dispatched as
        // plain commands so `execute_command_any_mode` does the work.
        KeyCode::Char('q') => execute(CMD_TOGGLE_MACRO_RECORDING),
        KeyCode::Char('Q') => execute(CMD_REPLAY_MACRO),

        // Count prefix - digits 1-9 start a count
        KeyCode::Char(c) if let Some(count) = count_from_digit(c) => {
            HandlerResult::Transition(InputState::CountPending { count })
        }

        // Single-key commands - execute immediately; unknown keys stay in base
        KeyCode::Char(c) => {
            map_single_key_command(c, modifiers).map_or(HandlerResult::Stay, execute)
        }

        KeyCode::Esc => execute(CMD_ESCAPE),

        _ => HandlerResult::Stay,
    }
}

impl InputHandler<BaseState> for KeyHandler {
    fn handle_key(_state: &BaseState, key: KeyEvent) -> HandlerResult {
        // Normalize key event for consistent handling across terminals
        // (e.g., 'c' + SHIFT + ALT -> 'C' + ALT)
        let key = normalize_key_event(key);

        match Chord::of(key.modifiers) {
            Chord::Ctrl => ctrl_key(key.code),
            Chord::Alt => alt_key(key.code, key.modifiers),
            Chord::Plain => plain_key(key.code, key.modifiers),
        }
    }
}
