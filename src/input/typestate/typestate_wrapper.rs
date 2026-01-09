//! Typestate wrapper for compile-time safety
//!
//! Provides a type-level encoding of the input state for compile-time guarantees
//! about state transitions.

use std::marker::PhantomData;

use crossterm::event::KeyEvent;

use super::handler_result::HandlerResult;
use super::handlers::{InputHandler, KeyHandler};
use super::input_state::InputState;
use super::state_types::*;

/// Typestate wrapper that encodes the current state at the type level
///
/// This provides compile-time guarantees that state transitions are valid.
/// Use `InputStateMachine` for runtime state management.
#[derive(Debug, Clone)]
pub struct TypestateHandler<S: HandlerState> {
    _marker: PhantomData<S>,
}

impl<S: HandlerState> TypestateHandler<S> {
    /// Create a new handler in the given state
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<S: HandlerState> Default for TypestateHandler<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl TypestateHandler<BaseState> {
    /// Create a new handler in base state
    pub fn base() -> Self {
        Self::new()
    }

    /// Process a key event and return the result with potential state transition
    pub fn process_key(self, key: KeyEvent) -> (HandlerResult, TypestateHandlerState) {
        let result = KeyHandler::handle_key(&BaseState, key);
        let next_state = match &result {
            HandlerResult::Transition(InputState::GotoPending) => {
                TypestateHandlerState::GotoPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::ViewPending) => {
                TypestateHandlerState::ViewPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::MatchPending) => {
                TypestateHandlerState::MatchPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::SurroundAddPending) => {
                TypestateHandlerState::SurroundAddPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::SurroundDeletePending) => {
                TypestateHandlerState::SurroundDeletePending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::SurroundReplaceFromPending) => {
                TypestateHandlerState::SurroundReplaceFromPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::SurroundReplaceToPending { from_char }) => {
                TypestateHandlerState::SurroundReplaceToPending(TypestateHandler::new(), *from_char)
            }
            HandlerResult::Transition(InputState::FindCharPending { find_type }) => {
                TypestateHandlerState::FindCharPending(TypestateHandler::new(), *find_type)
            }
            HandlerResult::Transition(InputState::ReplaceCharPending) => {
                TypestateHandlerState::ReplaceCharPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::CountPending { count }) => {
                TypestateHandlerState::CountPending(TypestateHandler::new(), *count)
            }
            HandlerResult::Transition(InputState::UnmatchedPrevPending) => {
                TypestateHandlerState::UnmatchedPrevPending(TypestateHandler::new())
            }
            HandlerResult::Transition(InputState::UnmatchedNextPending) => {
                TypestateHandlerState::UnmatchedNextPending(TypestateHandler::new())
            }
            _ => TypestateHandlerState::Base(TypestateHandler::new()),
        };
        (result, next_state)
    }
}

/// Enum representing the typestate handler in any state
///
/// This allows runtime dispatch while maintaining type safety.
#[derive(Debug, Clone)]
pub enum TypestateHandlerState {
    Base(TypestateHandler<BaseState>),
    GotoPending(TypestateHandler<GotoPending>),
    ViewPending(TypestateHandler<ViewPending>),
    MatchPending(TypestateHandler<MatchPending>),
    SurroundAddPending(TypestateHandler<SurroundAddPending>),
    SurroundDeletePending(TypestateHandler<SurroundDeletePending>),
    SurroundReplaceFromPending(TypestateHandler<SurroundReplaceFromPending>),
    SurroundReplaceToPending(TypestateHandler<SurroundReplaceToPending>, char),
    TextObjectAroundPending(TypestateHandler<TextObjectAroundPending>),
    TextObjectInsidePending(TypestateHandler<TextObjectInsidePending>),
    FindCharPending(TypestateHandler<FindCharPending>, FindType),
    ReplaceCharPending(TypestateHandler<ReplaceCharPending>),
    CountPending(TypestateHandler<CountPending>, usize),
    UnmatchedPrevPending(TypestateHandler<UnmatchedPrevPending>),
    UnmatchedNextPending(TypestateHandler<UnmatchedNextPending>),
}

impl Default for TypestateHandlerState {
    fn default() -> Self {
        Self::Base(TypestateHandler::base())
    }
}

impl TypestateHandlerState {
    /// Create a new handler in base state
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a key and return the result and new state
    pub fn process_key(self, key: KeyEvent) -> (HandlerResult, Self) {
        match self {
            Self::Base(handler) => handler.process_key(key),
            Self::GotoPending(_) => {
                let result = KeyHandler::handle_key(&GotoPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::GotoPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::ViewPending(_) => {
                let result = KeyHandler::handle_key(&ViewPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::ViewPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::MatchPending(_) => {
                let result = KeyHandler::handle_key(&MatchPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::MatchPending(TypestateHandler::new()),
                    HandlerResult::Transition(InputState::SurroundAddPending) => {
                        Self::SurroundAddPending(TypestateHandler::new())
                    }
                    HandlerResult::Transition(InputState::SurroundDeletePending) => {
                        Self::SurroundDeletePending(TypestateHandler::new())
                    }
                    HandlerResult::Transition(InputState::SurroundReplaceFromPending) => {
                        Self::SurroundReplaceFromPending(TypestateHandler::new())
                    }
                    HandlerResult::Transition(InputState::TextObjectAroundPending) => {
                        Self::TextObjectAroundPending(TypestateHandler::new())
                    }
                    HandlerResult::Transition(InputState::TextObjectInsidePending) => {
                        Self::TextObjectInsidePending(TypestateHandler::new())
                    }
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::SurroundAddPending(_) => {
                let result = KeyHandler::handle_key(&SurroundAddPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::SurroundAddPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::SurroundDeletePending(_) => {
                let result = KeyHandler::handle_key(&SurroundDeletePending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::SurroundDeletePending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::SurroundReplaceFromPending(_) => {
                let result = KeyHandler::handle_key(&SurroundReplaceFromPending, key);
                let next = match &result {
                    HandlerResult::Stay => {
                        Self::SurroundReplaceFromPending(TypestateHandler::new())
                    }
                    HandlerResult::Transition(InputState::SurroundReplaceToPending {
                        from_char,
                    }) => Self::SurroundReplaceToPending(TypestateHandler::new(), *from_char),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::SurroundReplaceToPending(_, from_char) => {
                let result = KeyHandler::handle_key(&SurroundReplaceToPending { from_char }, key);
                let next = match &result {
                    HandlerResult::Stay => {
                        Self::SurroundReplaceToPending(TypestateHandler::new(), from_char)
                    }
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::TextObjectAroundPending(_) => {
                let result = KeyHandler::handle_key(&TextObjectAroundPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::TextObjectAroundPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::TextObjectInsidePending(_) => {
                let result = KeyHandler::handle_key(&TextObjectInsidePending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::TextObjectInsidePending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::FindCharPending(_, find_type) => {
                let result = KeyHandler::handle_key(&FindCharPending { find_type }, key);
                let next = match &result {
                    HandlerResult::Stay => {
                        Self::FindCharPending(TypestateHandler::new(), find_type)
                    }
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::ReplaceCharPending(_) => {
                let result = KeyHandler::handle_key(&ReplaceCharPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::ReplaceCharPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::CountPending(_, count) => {
                let result = KeyHandler::handle_key(&CountPending { count }, key);
                let next = match &result {
                    HandlerResult::Transition(InputState::CountPending { count: new_count }) => {
                        Self::CountPending(TypestateHandler::new(), *new_count)
                    }
                    HandlerResult::Stay => Self::CountPending(TypestateHandler::new(), count),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::UnmatchedPrevPending(_) => {
                let result = KeyHandler::handle_key(&UnmatchedPrevPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::UnmatchedPrevPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
            Self::UnmatchedNextPending(_) => {
                let result = KeyHandler::handle_key(&UnmatchedNextPending, key);
                let next = match &result {
                    HandlerResult::Stay => Self::UnmatchedNextPending(TypestateHandler::new()),
                    _ => Self::Base(TypestateHandler::new()),
                };
                (result, next)
            }
        }
    }

    /// Check if in base state
    pub fn is_base(&self) -> bool {
        matches!(self, Self::Base(_))
    }

    /// Get the current state name
    pub fn state_name(&self) -> &'static str {
        match self {
            Self::Base(_) => BaseState::state_name(),
            Self::GotoPending(_) => GotoPending::state_name(),
            Self::ViewPending(_) => ViewPending::state_name(),
            Self::MatchPending(_) => MatchPending::state_name(),
            Self::SurroundAddPending(_) => SurroundAddPending::state_name(),
            Self::SurroundDeletePending(_) => SurroundDeletePending::state_name(),
            Self::SurroundReplaceFromPending(_) => SurroundReplaceFromPending::state_name(),
            Self::SurroundReplaceToPending(_, _) => SurroundReplaceToPending::state_name(),
            Self::TextObjectAroundPending(_) => TextObjectAroundPending::state_name(),
            Self::TextObjectInsidePending(_) => TextObjectInsidePending::state_name(),
            Self::FindCharPending(_, _) => FindCharPending::state_name(),
            Self::ReplaceCharPending(_) => ReplaceCharPending::state_name(),
            Self::CountPending(_, _) => CountPending::state_name(),
            Self::UnmatchedPrevPending(_) => UnmatchedPrevPending::state_name(),
            Self::UnmatchedNextPending(_) => UnmatchedNextPending::state_name(),
        }
    }
}
