//! Input state machine for runtime state management
//!
//! The `InputStateMachine` provides a runtime wrapper around the typestate pattern,
//! allowing dynamic dispatch of key events to the appropriate handler.

use crossterm::event::KeyEvent;

use super::handler_result::HandlerResult;
use super::handlers::{InputHandler, KeyHandler};
use super::input_state::InputState;
use super::key_mapping::parse_helix_key_string;
use super::state_types::*;

/// State machine for input handling
///
/// Manages the current input state and dispatches key events to the appropriate
/// handler based on the current state.
#[derive(Debug, Clone, Default)]
pub struct InputStateMachine {
    state: InputState,
}

impl InputStateMachine {
    /// Create a new state machine in base state
    pub fn new() -> Self {
        Self {
            state: InputState::Base,
        }
    }

    /// Get the current state
    pub fn state(&self) -> &InputState {
        &self.state
    }

    /// Reset to base state
    pub fn reset(&mut self) {
        self.state = InputState::Base;
    }

    /// Process a key event and return the result
    ///
    /// Updates internal state based on the result.
    pub fn process_key(&mut self, key: KeyEvent) -> HandlerResult {
        let result = match &self.state {
            InputState::Base => KeyHandler::handle_key(&BaseState, key),
            InputState::GotoPending => KeyHandler::handle_key(&GotoPending, key),
            InputState::ViewPending => KeyHandler::handle_key(&ViewPending, key),
            InputState::MatchPending => KeyHandler::handle_key(&MatchPending, key),
            InputState::SurroundAddPending => KeyHandler::handle_key(&SurroundAddPending, key),
            InputState::SurroundDeletePending => {
                KeyHandler::handle_key(&SurroundDeletePending, key)
            }
            InputState::SurroundReplaceFromPending => {
                KeyHandler::handle_key(&SurroundReplaceFromPending, key)
            }
            InputState::SurroundReplaceToPending { from_char } => KeyHandler::handle_key(
                &SurroundReplaceToPending {
                    from_char: *from_char,
                },
                key,
            ),
            InputState::TextObjectAroundPending => {
                KeyHandler::handle_key(&TextObjectAroundPending, key)
            }
            InputState::TextObjectInsidePending => {
                KeyHandler::handle_key(&TextObjectInsidePending, key)
            }
            InputState::FindCharPending { find_type } => KeyHandler::handle_key(
                &FindCharPending {
                    find_type: *find_type,
                },
                key,
            ),
            InputState::ReplaceCharPending => KeyHandler::handle_key(&ReplaceCharPending, key),
            InputState::RegisterPending => KeyHandler::handle_key(&RegisterPending, key),
            InputState::RegisterOpPending { register } => KeyHandler::handle_key(
                &RegisterOpPending {
                    register: *register,
                },
                key,
            ),
            InputState::CommandLinePending { buffer } => KeyHandler::handle_key(
                &CommandLinePending {
                    buffer: buffer.clone(),
                },
                key,
            ),
            InputState::RegexPromptPending { kind, buffer } => KeyHandler::handle_key(
                &RegexPromptPending {
                    kind: *kind,
                    buffer: buffer.clone(),
                },
                key,
            ),
            InputState::CountPending { count } => {
                KeyHandler::handle_key(&CountPending { count: *count }, key)
            }
            InputState::UnmatchedPrevPending => KeyHandler::handle_key(&UnmatchedPrevPending, key),
            InputState::UnmatchedNextPending => KeyHandler::handle_key(&UnmatchedNextPending, key),
        };

        // Update state based on result - move values instead of cloning
        match result {
            HandlerResult::Stay => HandlerResult::Stay,
            HandlerResult::Transition(new_state) => {
                // Return the state variant for the caller, store a fresh copy in self
                let result_state = new_state.clone();
                self.state = new_state;
                HandlerResult::Transition(result_state)
            }
            HandlerResult::Execute(cmd) => {
                self.state = InputState::Base;
                HandlerResult::Execute(cmd)
            }
            HandlerResult::Cancel => {
                self.state = InputState::Base;
                HandlerResult::Cancel
            }
        }
    }

    /// Check if waiting for character input
    pub fn is_waiting_for_char(&self) -> bool {
        self.state.is_waiting_for_char()
    }

    /// Check if in a prefix state (waiting for more input)
    pub fn is_prefix_state(&self) -> bool {
        self.state.is_prefix_state()
    }

    /// Get current count if in count pending state
    pub fn pending_count(&self) -> Option<usize> {
        match &self.state {
            InputState::CountPending { count } => Some(*count),
            _ => None,
        }
    }

    /// Get the pending surround replace "from" char if in that state
    ///
    /// Returns Some(char) when waiting for the "to" character in surround replace,
    /// used to preview which brackets will be replaced.
    pub fn pending_surround_replace_char(&self) -> Option<char> {
        match &self.state {
            InputState::SurroundReplaceToPending { from_char } => Some(*from_char),
            _ => None,
        }
    }

    /// Get info about a pending register-select sequence, for UI feedback.
    ///
    /// Returns `None` outside of `"`/`"<reg>` input.
    pub fn pending_register(&self) -> Option<RegisterPreview> {
        match &self.state {
            InputState::RegisterPending => Some(RegisterPreview::SelectingRegister),
            InputState::RegisterOpPending { register } => {
                Some(RegisterPreview::AwaitingOperator(*register))
            }
            _ => None,
        }
    }

    /// Get the in-progress `:`-prefixed command-line buffer, if pending.
    ///
    /// Returns the text typed after the colon (not including the colon
    /// itself), for rendering a live prompt line.
    pub fn pending_command_line(&self) -> Option<&str> {
        match &self.state {
            InputState::CommandLinePending { buffer } => Some(buffer.as_str()),
            _ => None,
        }
    }

    /// Attempt a multi-token keymap-overlay expansion via
    /// clone-and-commit-or-discard.
    ///
    /// `tokens` must be the tokens of a canonical key sequence resolved by
    /// the keymap overlay (see `crate::input::keymap::CanonicalKeys::tokens`).
    /// Every token is replayed against a private clone of `self`; the
    /// clone is committed back into `self` only if every intermediate
    /// token transitions and the final token executes. On any other
    /// outcome — an unparsable token, a `Cancel`/`Stay`, or a `Transition`
    /// on the final token — `self` is left completely untouched and
    /// `None` is returned.
    ///
    /// Only meant for multi-token expansions: a single-token target that
    /// itself lands on a further-pending state (e.g. a remap to
    /// `find_next_char`) is dispatched directly by the caller instead,
    /// bypassing this method entirely (see the design decision record,
    /// S6 rule 3) — the registry never produces a multi-token canonical
    /// sequence whose final token is itself a prefix, so this method's
    /// stricter "final token must execute" contract holds for every real
    /// input it's actually called with.
    ///
    /// Debug-asserts that `self` is in `Base` before dispatching: the
    /// keymap overlay only ever produces a multi-token expansion for a
    /// `Base`-context lookup (see the design decision record), so this
    /// would catch a caller wiring bug, not user input.
    pub fn apply_canonical_expansion(&mut self, tokens: &[&str]) -> Option<String> {
        debug_assert!(
            self.state.is_base(),
            "multi-token keymap expansion applied outside Base context"
        );
        if tokens.is_empty() {
            return None;
        }
        let mut clone = self.clone();
        let last = tokens.len() - 1;
        for (i, token) in tokens.iter().enumerate() {
            let key_event = parse_helix_key_string(token)?;
            match clone.process_key(key_event) {
                HandlerResult::Execute(cmd) if i == last => {
                    let resolved = cmd.to_string();
                    *self = clone;
                    return Some(resolved);
                }
                HandlerResult::Transition(_) if i != last => continue,
                _ => return None,
            }
        }
        None
    }

    /// Get the in-progress `s`/`S` regex-selection prompt buffer, if pending.
    ///
    /// Returns the command kind and the pattern typed so far (not including
    /// the leading 's'/'S'), for rendering a live prompt line.
    pub fn pending_regex_prompt(&self) -> Option<(super::input_state::RegexPromptKind, &str)> {
        match &self.state {
            InputState::RegexPromptPending { kind, buffer } => Some((*kind, buffer.as_str())),
            _ => None,
        }
    }

    /// Get a preview character for surround operations
    ///
    /// Returns Some(char) when in a state that should show bracket preview:
    /// - `SurroundReplaceToPending` - show brackets that will be replaced
    /// - `SurroundDeletePending` waiting for char - handled at input time
    ///
    /// Used for visual feedback in the editor.
    pub fn pending_surround_preview(&self) -> Option<SurroundPreview> {
        match &self.state {
            InputState::SurroundReplaceToPending { from_char } => {
                Some(SurroundPreview::Replace(*from_char))
            }
            _ => None,
        }
    }
}

/// Type of surround preview to display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurroundPreview {
    /// Preview for surround replace - shows which brackets will be replaced
    Replace(char),
    /// Preview for surround delete - shows which brackets will be deleted
    Delete(char),
}

/// State of a pending named-register selection sequence, for UI feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterPreview {
    /// Waiting for the register character after `"`.
    SelectingRegister,
    /// Register selected; waiting for the operator (y/p/P/R).
    AwaitingOperator(char),
}
