//! Typestate-based input handling for compile-time safe state transitions
//!
//! This module implements the typestate pattern for input handling, providing
//! compile-time guarantees about which mode the editor is in and what kind of
//! input is expected.
//!
//! # Architecture
//!
//! The system uses zero-sized marker types to encode input state:
//!
//! ```text
//! BaseState        -> GotoPending (after 'g')
//!                  -> ViewPending (after 'z')
//!                  -> MatchPending (after 'm')
//!                  -> FindCharPending (after 'f'/'F'/'t'/'T')
//!                  -> ReplaceCharPending (after 'r')
//!                  -> CountPending (after digit 1-9)
//!
//! GotoPending      -> BaseState (after 'g'/'h'/'l'/'s'/'e' or cancel)
//! ViewPending      -> BaseState (after 'z'/'t'/'b'/'m'/'j'/'k' or cancel)
//! MatchPending     -> BaseState (after 'm' or cancel)
//! FindCharPending  -> BaseState (after any char or cancel)
//! ReplaceCharPending -> BaseState (after any char or cancel)
//! CountPending     -> BaseState (after command char or cancel)
//! ```
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::input::typestate::{InputState, InputStateMachine};
//!
//! let mut state_machine = InputStateMachine::new();
//! assert!(state_machine.state().is_base());
//!
//! // Press 'g' - transition to GotoPending
//! let result = state_machine.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
//! assert!(state_machine.state().is_goto_pending());
//!
//! // Press 'g' again - complete "gg" command
//! let result = state_machine.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
//! assert!(matches!(result, HandlerResult::Execute(_)));
//! assert!(state_machine.state().is_base());
//! ```
//!
//! # Integration Status
//!
//! This module provides the foundation for typestate-based input handling.
//! The types are currently used in tests and will be integrated into the
//! main event loop in a future update.

use std::borrow::Cow;
use std::marker::PhantomData;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::helix::commands::*;

// ============================================================================
// Handler state marker types (zero-sized)
// ============================================================================

/// Base state - no prefix, accepting normal input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BaseState;

/// Waiting for second key after 'g' (goto commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GotoPending;

/// Waiting for second key after 'z' (view commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewPending;

/// Waiting for second key after 'm' (match commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchPending;

/// Waiting for character after 'ms' (surround add)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundAddPending;

/// Waiting for character after 'md' (surround delete)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundDeletePending;

/// Waiting for first character after 'mr' (surround replace from char)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundReplaceFromPending;

/// Waiting for second character after 'mr{from}' (surround replace to char)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurroundReplaceToPending {
    /// The character to replace from
    pub from_char: char,
}

/// Waiting for text object after 'ma' (select around)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObjectAroundPending;

/// Waiting for text object after 'mi' (select inside)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextObjectInsidePending;

/// Waiting for character after 'f'/'F'/'t'/'T' (find/till commands)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FindCharPending {
    /// The direction and type of find operation
    pub find_type: FindType,
}

/// Type of find/till operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindType {
    /// Find forward ('f')
    FindForward,
    /// Find backward ('F')
    FindBackward,
    /// Till forward ('t')
    TillForward,
    /// Till backward ('T')
    TillBackward,
}

impl FindType {
    /// Get the command prefix for this find type
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::FindForward => "f",
            Self::FindBackward => "F",
            Self::TillForward => "t",
            Self::TillBackward => "T",
        }
    }
}

/// Waiting for character after 'r' (replace command)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplaceCharPending;

/// Building a count prefix (digits 1-9, then 0-9)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountPending {
    /// The accumulated count value
    pub count: usize,
}

// ============================================================================
// Sealed trait for handler states
// ============================================================================

mod private {
    pub trait Sealed {}
}

impl private::Sealed for BaseState {}
impl private::Sealed for GotoPending {}
impl private::Sealed for ViewPending {}
impl private::Sealed for MatchPending {}
impl private::Sealed for SurroundAddPending {}
impl private::Sealed for SurroundDeletePending {}
impl private::Sealed for SurroundReplaceFromPending {}
impl private::Sealed for SurroundReplaceToPending {}
impl private::Sealed for TextObjectAroundPending {}
impl private::Sealed for TextObjectInsidePending {}
impl private::Sealed for FindCharPending {}
impl private::Sealed for ReplaceCharPending {}
impl private::Sealed for CountPending {}

/// Marker trait for handler state types
///
/// This trait is sealed to ensure only valid states can be used.
pub trait HandlerState: private::Sealed {
    /// Human-readable name of this state
    fn state_name() -> &'static str;
}

impl HandlerState for BaseState {
    fn state_name() -> &'static str {
        "BASE"
    }
}

impl HandlerState for GotoPending {
    fn state_name() -> &'static str {
        "GOTO_PENDING"
    }
}

impl HandlerState for ViewPending {
    fn state_name() -> &'static str {
        "VIEW_PENDING"
    }
}

impl HandlerState for MatchPending {
    fn state_name() -> &'static str {
        "MATCH_PENDING"
    }
}

impl HandlerState for SurroundAddPending {
    fn state_name() -> &'static str {
        "SURROUND_ADD_PENDING"
    }
}

impl HandlerState for SurroundDeletePending {
    fn state_name() -> &'static str {
        "SURROUND_DELETE_PENDING"
    }
}

impl HandlerState for SurroundReplaceFromPending {
    fn state_name() -> &'static str {
        "SURROUND_REPLACE_FROM_PENDING"
    }
}

impl HandlerState for SurroundReplaceToPending {
    fn state_name() -> &'static str {
        "SURROUND_REPLACE_TO_PENDING"
    }
}

impl HandlerState for TextObjectAroundPending {
    fn state_name() -> &'static str {
        "TEXT_OBJECT_AROUND_PENDING"
    }
}

impl HandlerState for TextObjectInsidePending {
    fn state_name() -> &'static str {
        "TEXT_OBJECT_INSIDE_PENDING"
    }
}

impl HandlerState for FindCharPending {
    fn state_name() -> &'static str {
        "FIND_CHAR_PENDING"
    }
}

impl HandlerState for ReplaceCharPending {
    fn state_name() -> &'static str {
        "REPLACE_CHAR_PENDING"
    }
}

impl HandlerState for CountPending {
    fn state_name() -> &'static str {
        "COUNT_PENDING"
    }
}

// ============================================================================
// Handler result type
// ============================================================================

/// Result of handling a key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResult {
    /// Stay in current state, no command to execute
    Stay,
    /// Transition to a new state
    Transition(InputState),
    /// Execute a command and return to base state
    ///
    /// Uses `Cow<'static, str>` to avoid allocations for static command strings
    /// while still supporting dynamic commands (with count prefix or character arguments).
    Execute(Cow<'static, str>),
    /// Cancel current state and return to base
    Cancel,
}

impl HandlerResult {
    /// Check if this result indicates staying in the same state
    pub fn is_stay(&self) -> bool {
        matches!(self, Self::Stay)
    }

    /// Check if this result indicates a state transition
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::Transition(_))
    }

    /// Check if this result indicates command execution
    pub fn is_execute(&self) -> bool {
        matches!(self, Self::Execute(_))
    }

    /// Check if this result indicates cancellation
    pub fn is_cancel(&self) -> bool {
        matches!(self, Self::Cancel)
    }

    /// Get the command to execute, if any
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Execute(cmd) => Some(cmd),
            _ => None,
        }
    }
}

// ============================================================================
// Runtime input state enum (wraps typestate)
// ============================================================================

/// Runtime representation of input state
///
/// This enum wraps the typestate pattern for runtime use, allowing dynamic
/// dispatch while still benefiting from the type-safe design.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputState {
    /// No prefix, normal input mode
    #[default]
    Base,
    /// After 'g' - waiting for goto command second key
    GotoPending,
    /// After 'z' - waiting for view command second key
    ViewPending,
    /// After 'm' - waiting for match command second key
    MatchPending,
    /// After 'ms' - waiting for surround add character
    SurroundAddPending,
    /// After 'md' - waiting for surround delete character
    SurroundDeletePending,
    /// After 'mr' - waiting for surround replace from character
    SurroundReplaceFromPending,
    /// After 'mr{char}' - waiting for surround replace to character
    SurroundReplaceToPending { from_char: char },
    /// After 'ma' - waiting for text object (around)
    TextObjectAroundPending,
    /// After 'mi' - waiting for text object (inside)
    TextObjectInsidePending,
    /// After 'f'/'F'/'t'/'T' - waiting for character
    FindCharPending { find_type: FindType },
    /// After 'r' - waiting for replacement character
    ReplaceCharPending,
    /// After digit 1-9 - building count prefix
    CountPending { count: usize },
}

impl InputState {
    /// Check if this is the base state
    pub fn is_base(&self) -> bool {
        matches!(self, Self::Base)
    }

    /// Check if this is goto pending state
    pub fn is_goto_pending(&self) -> bool {
        matches!(self, Self::GotoPending)
    }

    /// Check if this is view pending state
    pub fn is_view_pending(&self) -> bool {
        matches!(self, Self::ViewPending)
    }

    /// Check if this is match pending state
    pub fn is_match_pending(&self) -> bool {
        matches!(self, Self::MatchPending)
    }

    /// Check if this is find char pending state
    pub fn is_find_char_pending(&self) -> bool {
        matches!(self, Self::FindCharPending { .. })
    }

    /// Check if this is replace char pending state
    pub fn is_replace_char_pending(&self) -> bool {
        matches!(self, Self::ReplaceCharPending)
    }

    /// Check if this is count pending state
    pub fn is_count_pending(&self) -> bool {
        matches!(self, Self::CountPending { .. })
    }

    /// Check if this state is waiting for a character argument
    pub fn is_waiting_for_char(&self) -> bool {
        matches!(
            self,
            Self::FindCharPending { .. }
                | Self::ReplaceCharPending
                | Self::SurroundAddPending
                | Self::SurroundDeletePending
                | Self::SurroundReplaceFromPending
                | Self::SurroundReplaceToPending { .. }
        )
    }

    /// Check if this state is a prefix state (waiting for more input)
    pub fn is_prefix_state(&self) -> bool {
        !matches!(self, Self::Base)
    }

    /// Check if this is a surround pending state
    pub fn is_surround_pending(&self) -> bool {
        matches!(
            self,
            Self::SurroundAddPending
                | Self::SurroundDeletePending
                | Self::SurroundReplaceFromPending
                | Self::SurroundReplaceToPending { .. }
        )
    }

    /// Check if this is a text object pending state (waiting for text object type)
    pub fn is_text_object_pending(&self) -> bool {
        matches!(
            self,
            Self::TextObjectAroundPending | Self::TextObjectInsidePending
        )
    }

    /// Get the state name for display
    pub fn name(&self) -> &'static str {
        match self {
            Self::Base => "BASE",
            Self::GotoPending => "GOTO_PENDING",
            Self::ViewPending => "VIEW_PENDING",
            Self::MatchPending => "MATCH_PENDING",
            Self::SurroundAddPending => "SURROUND_ADD_PENDING",
            Self::SurroundDeletePending => "SURROUND_DELETE_PENDING",
            Self::SurroundReplaceFromPending => "SURROUND_REPLACE_FROM_PENDING",
            Self::SurroundReplaceToPending { .. } => "SURROUND_REPLACE_TO_PENDING",
            Self::TextObjectAroundPending => "TEXT_OBJECT_AROUND_PENDING",
            Self::TextObjectInsidePending => "TEXT_OBJECT_INSIDE_PENDING",
            Self::FindCharPending { .. } => "FIND_CHAR_PENDING",
            Self::ReplaceCharPending => "REPLACE_CHAR_PENDING",
            Self::CountPending { .. } => "COUNT_PENDING",
        }
    }
}

// ============================================================================
// Input handler trait
// ============================================================================

/// Trait for handling input in a specific state
///
/// This trait uses the typestate pattern to encode the current state at the
/// type level, ensuring compile-time safety for state transitions.
pub trait InputHandler<S: HandlerState> {
    /// Handle a key event in this state
    ///
    /// Returns a `HandlerResult` indicating what action to take.
    fn handle_key(state: &S, key: KeyEvent) -> HandlerResult;
}

/// Marker struct for implementing InputHandler
#[derive(Debug, Clone, Copy)]
pub struct KeyHandler;

// ============================================================================
// Base state handler
// ============================================================================

impl InputHandler<BaseState> for KeyHandler {
    fn handle_key(_state: &BaseState, key: KeyEvent) -> HandlerResult {
        // Only handle keys without modifiers (except Shift for uppercase)
        let has_modifier = key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT);

        match (key.code, has_modifier) {
            // Special modifiers - let through for Ctrl-R, Ctrl-C, etc.
            (KeyCode::Char('r'), true) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                HandlerResult::Execute(Cow::Borrowed(CMD_CTRL_R))
            }
            (KeyCode::Char('c'), true) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                HandlerResult::Execute(Cow::Borrowed(CMD_TOGGLE_COMMENTS))
            }

            // Ignore other modifier combinations
            (_, true) => HandlerResult::Stay,

            // Prefix commands - transition to pending states
            (KeyCode::Char('g'), false) => HandlerResult::Transition(InputState::GotoPending),
            (KeyCode::Char('z'), false) => HandlerResult::Transition(InputState::ViewPending),
            (KeyCode::Char('m'), false) => HandlerResult::Transition(InputState::MatchPending),

            // Find/till commands - transition to find char pending
            (KeyCode::Char('f'), false) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::FindForward,
            }),
            (KeyCode::Char('F'), _) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::FindBackward,
            }),
            (KeyCode::Char('t'), false) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::TillForward,
            }),
            (KeyCode::Char('T'), _) => HandlerResult::Transition(InputState::FindCharPending {
                find_type: FindType::TillBackward,
            }),

            // Replace command - transition to replace char pending
            (KeyCode::Char('r'), false) => {
                HandlerResult::Transition(InputState::ReplaceCharPending)
            }

            // Count prefix - digits 1-9 start a count
            (KeyCode::Char(c @ '1'..='9'), false) => {
                let count = c
                    .to_digit(10)
                    .expect("pattern match guarantees ASCII digit")
                    as usize;
                HandlerResult::Transition(InputState::CountPending { count })
            }

            // Single-key commands - execute immediately
            (KeyCode::Char(c), _) => {
                if let Some(cmd) = map_single_key_command(c, key.modifiers) {
                    HandlerResult::Execute(Cow::Borrowed(cmd))
                } else {
                    // Unknown key - stay in base state
                    HandlerResult::Stay
                }
            }

            // Escape and other special keys
            (KeyCode::Esc, _) => HandlerResult::Execute(Cow::Borrowed(CMD_ESCAPE)),

            // Unknown keys - stay in base state
            _ => HandlerResult::Stay,
        }
    }
}

// ============================================================================
// Goto pending state handler (after 'g')
// ============================================================================

impl InputHandler<GotoPending> for KeyHandler {
    fn handle_key(_state: &GotoPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'gg' - goto file start
            KeyCode::Char('g') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_FILE_START)),
            // 'gh' - goto line start
            KeyCode::Char('h') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_LINE_START)),
            // 'gl' - goto line end
            KeyCode::Char('l') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_LINE_END)),
            // 'gs' - goto first non-whitespace
            KeyCode::Char('s') => {
                HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_FIRST_NONWHITESPACE))
            }
            // 'ge' - goto last line
            KeyCode::Char('e') => HandlerResult::Execute(Cow::Borrowed(CMD_GOTO_LAST_LINE)),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// View pending state handler (after 'z')
// ============================================================================

impl InputHandler<ViewPending> for KeyHandler {
    fn handle_key(_state: &ViewPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'zz' - center view
            KeyCode::Char('z') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_CENTER)),
            // 'zt' - view top
            KeyCode::Char('t') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_TOP)),
            // 'zb' - view bottom
            KeyCode::Char('b') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_BOTTOM)),
            // 'zm' - view center horizontal
            KeyCode::Char('m') => HandlerResult::Execute(Cow::Borrowed(CMD_VIEW_CENTER_HORIZONTAL)),
            // 'zj' - scroll down
            KeyCode::Char('j') => HandlerResult::Execute(Cow::Borrowed(CMD_SCROLL_DOWN)),
            // 'zk' - scroll up
            KeyCode::Char('k') => HandlerResult::Execute(Cow::Borrowed(CMD_SCROLL_UP)),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Match pending state handler (after 'm')
// ============================================================================

impl InputHandler<MatchPending> for KeyHandler {
    fn handle_key(_state: &MatchPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // 'mm' - match brackets
            KeyCode::Char('m') => HandlerResult::Execute(Cow::Borrowed(CMD_MATCH_BRACKETS)),
            // 'ms' - surround add (transition to SurroundAddPending)
            KeyCode::Char('s') => HandlerResult::Transition(InputState::SurroundAddPending),
            // 'md' - surround delete (transition to SurroundDeletePending)
            KeyCode::Char('d') => HandlerResult::Transition(InputState::SurroundDeletePending),
            // 'mr' - surround replace (transition to SurroundReplaceFromPending)
            KeyCode::Char('r') => HandlerResult::Transition(InputState::SurroundReplaceFromPending),
            // 'ma' - text object around (transition to TextObjectAroundPending)
            KeyCode::Char('a') => HandlerResult::Transition(InputState::TextObjectAroundPending),
            // 'mi' - text object inside (transition to TextObjectInsidePending)
            KeyCode::Char('i') => HandlerResult::Transition(InputState::TextObjectInsidePending),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid second key - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Surround add pending state handler (after 'ms')
// ============================================================================

impl InputHandler<SurroundAddPending> for KeyHandler {
    fn handle_key(_state: &SurroundAddPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character for surrounding
            KeyCode::Char(c) => {
                let cmd = format!("ms{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Surround delete pending state handler (after 'md')
// ============================================================================

impl InputHandler<SurroundDeletePending> for KeyHandler {
    fn handle_key(_state: &SurroundDeletePending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character for deletion target
            KeyCode::Char(c) => {
                let cmd = format!("md{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Surround replace from pending state handler (after 'mr')
// ============================================================================

impl InputHandler<SurroundReplaceFromPending> for KeyHandler {
    fn handle_key(_state: &SurroundReplaceFromPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character as "from" character
            KeyCode::Char(c) => {
                HandlerResult::Transition(InputState::SurroundReplaceToPending { from_char: c })
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Surround replace to pending state handler (after 'mr{from}')
// ============================================================================

impl InputHandler<SurroundReplaceToPending> for KeyHandler {
    fn handle_key(state: &SurroundReplaceToPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character as "to" character
            KeyCode::Char(c) => {
                let cmd = format!("mr{}{}", state.from_char, c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Text object around pending state handler (after 'ma')
// ============================================================================

impl InputHandler<TextObjectAroundPending> for KeyHandler {
    fn handle_key(_state: &TextObjectAroundPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Valid text objects: w, W, (, ), [, ], {, }, <, >, ", ', `, p
            KeyCode::Char(
                c @ ('w' | 'W' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
                | 'p'),
            ) => {
                let cmd = format!("ma{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid text object - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Text object inside pending state handler (after 'mi')
// ============================================================================

impl InputHandler<TextObjectInsidePending> for KeyHandler {
    fn handle_key(_state: &TextObjectInsidePending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Valid text objects: w, W, (, ), [, ], {, }, <, >, ", ', `, p
            KeyCode::Char(
                c @ ('w' | 'W' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | '"' | '\'' | '`'
                | 'p'),
            ) => {
                let cmd = format!("mi{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Invalid text object - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Find char pending state handler (after 'f'/'F'/'t'/'T')
// ============================================================================

impl InputHandler<FindCharPending> for KeyHandler {
    fn handle_key(state: &FindCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character
            KeyCode::Char(c) => {
                // Dynamic command with character argument requires allocation
                let cmd = format!("{}{}", state.find_type.prefix(), c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Replace char pending state handler (after 'r')
// ============================================================================

impl InputHandler<ReplaceCharPending> for KeyHandler {
    fn handle_key(_state: &ReplaceCharPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // Accept any printable character (including space, newline, etc.)
            KeyCode::Char(c) => {
                // Dynamic command with character argument requires allocation
                let cmd = format!("r{}", c);
                HandlerResult::Execute(Cow::Owned(cmd))
            }
            // Enter - replace with newline
            KeyCode::Enter => HandlerResult::Execute(Cow::Borrowed("r\n")),
            // Escape - cancel
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Count pending state handler (building count prefix)
// ============================================================================

/// Maximum count value to prevent overflow attacks and unreasonable values.
/// 10,000 is more than enough for any practical editing operation.
const MAX_COUNT: usize = 10_000;

impl InputHandler<CountPending> for KeyHandler {
    fn handle_key(state: &CountPending, key: KeyEvent) -> HandlerResult {
        match key.code {
            // More digits - continue building count
            KeyCode::Char(c @ '0'..='9') => {
                let digit = c
                    .to_digit(10)
                    .expect("pattern match guarantees ASCII digit")
                    as usize;
                let new_count = state
                    .count
                    .saturating_mul(10)
                    .saturating_add(digit)
                    .min(MAX_COUNT);
                HandlerResult::Transition(InputState::CountPending { count: new_count })
            }
            // Command character - execute with count
            KeyCode::Char(c) => {
                // Only allow certain commands with count prefix
                if is_count_compatible_command(c, key.modifiers) {
                    if let Some(cmd) = map_single_key_command(c, key.modifiers) {
                        // Dynamic command with count prefix requires allocation
                        let full_cmd = format!("{}{}", state.count, cmd);
                        HandlerResult::Execute(Cow::Owned(full_cmd))
                    } else {
                        HandlerResult::Cancel
                    }
                } else {
                    // Invalid - count prefix not allowed with this command
                    HandlerResult::Cancel
                }
            }
            // Escape - cancel count
            KeyCode::Esc => HandlerResult::Cancel,
            // Other keys - cancel
            _ => HandlerResult::Cancel,
        }
    }
}

// ============================================================================
// Input state machine
// ============================================================================

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
            InputState::CountPending { count } => {
                KeyHandler::handle_key(&CountPending { count: *count }, key)
            }
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
}

// ============================================================================
// Helper functions
// ============================================================================

/// Map a single-key command character to its command string
///
/// Returns None for invalid commands or prefix commands.
fn map_single_key_command(c: char, modifiers: KeyModifiers) -> Option<&'static str> {
    let is_shift = modifiers.contains(KeyModifiers::SHIFT);

    match (c, is_shift) {
        // Movement
        ('h', false) => Some(CMD_MOVE_LEFT),
        ('j', false) => Some(CMD_MOVE_DOWN),
        ('k', false) => Some(CMD_MOVE_UP),
        ('l', false) => Some(CMD_MOVE_RIGHT),

        // Word movement
        ('w', false) => Some(CMD_MOVE_WORD_FORWARD),
        ('b', false) => Some(CMD_MOVE_WORD_BACKWARD),
        ('e', false) => Some(CMD_MOVE_WORD_END),

        // WORD movement (uppercase)
        ('W', _) => Some(CMD_MOVE_LONG_WORD_FORWARD),
        ('B', _) => Some(CMD_MOVE_LONG_WORD_BACKWARD),
        ('E', _) => Some(CMD_MOVE_LONG_WORD_END),

        // Selection
        ('x', false) => Some(CMD_SELECT_LINE),
        ('X', _) => Some(CMD_EXTEND_LINE),
        ('%', _) => Some(CMD_SELECT_ALL),
        (';', false) => Some(CMD_COLLAPSE_SELECTION),
        ('v', false) => Some(CMD_SELECT_MODE),

        // Editing
        ('d', false) => Some(CMD_DELETE_SELECTION),
        ('c', false) => Some(CMD_CHANGE),
        ('i', false) => Some(CMD_INSERT),
        ('a', false) => Some(CMD_APPEND),
        ('I', _) => Some(CMD_INSERT_LINE_START),
        ('A', _) => Some(CMD_APPEND_LINE_END),
        ('o', false) => Some(CMD_OPEN_BELOW),
        ('O', _) => Some(CMD_OPEN_ABOVE),
        ('J', _) => Some(CMD_JOIN_LINES),

        // Indentation
        ('>', _) => Some(CMD_INDENT),
        ('<', _) => Some(CMD_DEDENT),

        // Case
        ('~', _) => Some(CMD_SWITCH_CASE),
        ('`', _) => Some(CMD_SWITCH_CASE_ALT),

        // Clipboard
        ('y', false) => Some(CMD_YANK),
        ('p', false) => Some(CMD_PASTE_AFTER),
        ('P', _) => Some(CMD_PASTE_BEFORE),

        // Undo/Redo
        ('u', false) => Some(CMD_UNDO),
        ('U', _) => Some(CMD_REDO),

        // Repeat
        ('.', _) => Some(CMD_REPEAT),

        // Search
        ('/', _) => Some(CMD_SEARCH_FORWARD),
        ('?', _) => Some(CMD_SEARCH_BACKWARD),
        ('n', false) => Some(CMD_SEARCH_NEXT),
        ('N', _) => Some(CMD_SEARCH_PREV),
        ('*', _) => Some(CMD_SEARCH_WORD),

        // Selection manipulation
        ('s', false) => Some(CMD_SELECT_REGEX),
        ('S', _) => Some(CMD_SPLIT_SELECTION),
        ('&', _) => Some(CMD_ALIGN_SELECTIONS),
        ('_', _) => Some(CMD_TRIM_SELECTIONS),
        ('C', _) => Some(CMD_COPY_SELECTION_NEXT),
        ('K', _) => Some(CMD_KEEP_MATCHING),

        _ => None,
    }
}

/// Check if a command is compatible with count prefix
fn is_count_compatible_command(c: char, modifiers: KeyModifiers) -> bool {
    let is_shift = modifiers.contains(KeyModifiers::SHIFT);

    matches!(
        (c, is_shift),
        // Movement commands support count
        ('h', false)
            | ('j', false)
            | ('k', false)
            | ('l', false)
            | ('w', false)
            | ('b', false)
            | ('e', false)
            | ('W', _)
            | ('B', _)
            | ('E', _)
            // Selection
            | ('x', false)
            | ('X', _)
            // Some editing commands
            | ('d', false)
            | ('c', false)
            | ('J', _)
            | ('>', _)
            | ('<', _)
            // Undo/Redo
            | ('u', false)
            | ('U', _)
            // Search navigation
            | ('n', false)
            | ('N', _)
    )
}

// ============================================================================
// Public convenience functions for handlers
// ============================================================================

/// Map a key event to a Helix command string in normal mode
///
/// This is a convenience function for use in handlers that maps
/// simple key events to their corresponding Helix command strings.
/// It handles single-key commands but not multi-key sequences
/// (those require the InputStateMachine).
pub fn map_key_to_helix_command(key: KeyEvent) -> Option<&'static str> {
    match key.code {
        KeyCode::Char(c) => map_single_key_command(c, key.modifiers),
        KeyCode::Esc => Some(CMD_ESCAPE),
        KeyCode::Backspace => Some(CMD_BACKSPACE),
        KeyCode::Left => Some(CMD_ARROW_LEFT),
        KeyCode::Right => Some(CMD_ARROW_RIGHT),
        KeyCode::Up => Some(CMD_ARROW_UP),
        KeyCode::Down => Some(CMD_ARROW_DOWN),
        _ => None,
    }
}

/// Handle insert mode input and convert to command string
///
/// Returns the character/command to insert or execute in insert mode.
/// This handles printable characters, escape, backspace, and arrow keys.
pub fn handle_insert_mode_input(key: KeyEvent) -> Option<Cow<'static, str>> {
    match key.code {
        KeyCode::Char(c) => Some(Cow::Owned(c.to_string())),
        KeyCode::Esc => Some(Cow::Borrowed(CMD_ESCAPE)),
        KeyCode::Backspace => Some(Cow::Borrowed(CMD_BACKSPACE)),
        KeyCode::Enter => Some(Cow::Borrowed("\n")),
        KeyCode::Tab => Some(Cow::Borrowed("\t")),
        KeyCode::Left => Some(Cow::Borrowed(CMD_ARROW_LEFT)),
        KeyCode::Right => Some(Cow::Borrowed(CMD_ARROW_RIGHT)),
        KeyCode::Up => Some(Cow::Borrowed(CMD_ARROW_UP)),
        KeyCode::Down => Some(Cow::Borrowed(CMD_ARROW_DOWN)),
        _ => None,
    }
}

// ============================================================================
// Typestate wrapper for compile-time safety
// ============================================================================

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
        }
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Convert a command string to a KeyEvent for the input state machine
///
/// This is a helper function to bridge the gap between command strings
/// coming from the input layer and the KeyEvent-based state machine.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::input::typestate::command_to_key_event;
///
/// let key = command_to_key_event("h");
/// assert_eq!(key.code, KeyCode::Char('h'));
///
/// let key = command_to_key_event("Escape");
/// assert_eq!(key.code, KeyCode::Esc);
/// ```
pub fn command_to_key_event(command: &str) -> KeyEvent {
    // Handle single character commands
    if command.len() == 1 {
        let c = command.chars().next().unwrap();
        // Check if it's an uppercase letter (implies Shift)
        let modifiers = if c.is_ascii_uppercase() {
            KeyModifiers::SHIFT
        } else {
            KeyModifiers::NONE
        };
        return KeyEvent::new(KeyCode::Char(c), modifiers);
    }

    // Handle special command strings
    match command {
        "Escape" => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        "Left" => KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        "Right" => KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        "Up" => KeyEvent::new(KeyCode::Up, KeyModifiers::NONE),
        "Down" => KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        "Backspace" => KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE),
        _ => {
            // Default: treat first char as the key
            let c = command.chars().next().unwrap_or(' ');
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    mod input_state_tests {
        use super::*;

        #[test]
        fn test_input_state_default_is_base() {
            let state = InputState::default();
            assert!(state.is_base());
        }

        #[test]
        fn test_input_state_predicates() {
            assert!(InputState::Base.is_base());
            assert!(InputState::GotoPending.is_goto_pending());
            assert!(InputState::ViewPending.is_view_pending());
            assert!(InputState::MatchPending.is_match_pending());
            assert!(
                InputState::FindCharPending {
                    find_type: FindType::FindForward
                }
                .is_find_char_pending()
            );
            assert!(InputState::ReplaceCharPending.is_replace_char_pending());
            assert!(InputState::CountPending { count: 5 }.is_count_pending());
        }

        #[test]
        fn test_is_waiting_for_char() {
            assert!(!InputState::Base.is_waiting_for_char());
            assert!(!InputState::GotoPending.is_waiting_for_char());
            assert!(
                InputState::FindCharPending {
                    find_type: FindType::FindForward
                }
                .is_waiting_for_char()
            );
            assert!(InputState::ReplaceCharPending.is_waiting_for_char());
        }

        #[test]
        fn test_is_prefix_state() {
            assert!(!InputState::Base.is_prefix_state());
            assert!(InputState::GotoPending.is_prefix_state());
            assert!(InputState::ViewPending.is_prefix_state());
            assert!(InputState::CountPending { count: 3 }.is_prefix_state());
        }
    }

    mod handler_result_tests {
        use super::*;

        #[test]
        fn test_handler_result_predicates() {
            assert!(HandlerResult::Stay.is_stay());
            assert!(!HandlerResult::Stay.is_transition());

            assert!(HandlerResult::Transition(InputState::GotoPending).is_transition());
            assert!(!HandlerResult::Transition(InputState::GotoPending).is_execute());

            assert!(HandlerResult::Execute(Cow::Borrowed("gg")).is_execute());
            assert_eq!(
                HandlerResult::Execute(Cow::Borrowed("gg")).command(),
                Some("gg")
            );

            assert!(HandlerResult::Cancel.is_cancel());
        }
    }

    mod base_state_handler_tests {
        use super::*;

        #[test]
        fn test_base_state_goto_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::GotoPending)
            ));
        }

        #[test]
        fn test_base_state_view_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::ViewPending)
            ));
        }

        #[test]
        fn test_base_state_match_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::MatchPending)
            ));
        }

        #[test]
        fn test_base_state_find_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::FindCharPending {
                    find_type: FindType::FindForward
                })
            ));
        }

        #[test]
        fn test_base_state_replace_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::ReplaceCharPending)
            ));
        }

        #[test]
        fn test_base_state_count_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::CountPending { count: 3 })
            ));
        }

        #[test]
        fn test_base_state_single_key_command() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_MOVE_LEFT));
        }

        #[test]
        fn test_base_state_escape() {
            let result =
                KeyHandler::handle_key(&BaseState, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_ESCAPE));
        }

        #[test]
        fn test_base_state_find_backward_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::FindCharPending {
                    find_type: FindType::FindBackward
                })
            ));
        }

        #[test]
        fn test_base_state_till_backward_prefix() {
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::FindCharPending {
                    find_type: FindType::TillBackward
                })
            ));
        }

        #[test]
        fn test_base_state_unknown_key_stays() {
            // Test that unknown keys stay in base state
            let result = KeyHandler::handle_key(
                &BaseState,
                KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT),
            );
            assert!(matches!(result, HandlerResult::Stay));
        }
    }

    mod goto_pending_handler_tests {
        use super::*;

        #[test]
        fn test_goto_pending_gg() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_FILE_START));
        }

        #[test]
        fn test_goto_pending_gh() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_LINE_START));
        }

        #[test]
        fn test_goto_pending_gl() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_LINE_END));
        }

        #[test]
        fn test_goto_pending_invalid_key_cancels() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_goto_pending_escape_cancels() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_goto_pending_gs() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            );
            assert!(
                matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_FIRST_NONWHITESPACE)
            );
        }

        #[test]
        fn test_goto_pending_ge() {
            let result = KeyHandler::handle_key(
                &GotoPending,
                KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_GOTO_LAST_LINE));
        }
    }

    mod view_pending_handler_tests {
        use super::*;

        #[test]
        fn test_view_pending_zz() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_CENTER));
        }

        #[test]
        fn test_view_pending_zt() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_TOP));
        }

        #[test]
        fn test_view_pending_zb() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_BOTTOM));
        }

        #[test]
        fn test_view_pending_zm() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            );
            assert!(
                matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_VIEW_CENTER_HORIZONTAL)
            );
        }

        #[test]
        fn test_view_pending_zj() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_SCROLL_DOWN));
        }

        #[test]
        fn test_view_pending_zk() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_SCROLL_UP));
        }

        #[test]
        fn test_view_pending_escape_cancels() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_view_pending_invalid_cancels() {
            let result = KeyHandler::handle_key(
                &ViewPending,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod match_pending_handler_tests {
        use super::*;

        #[test]
        fn test_match_pending_mm() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == CMD_MATCH_BRACKETS));
        }

        #[test]
        fn test_match_pending_ms_transitions_to_surround_add() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::SurroundAddPending)
            ));
        }

        #[test]
        fn test_match_pending_md_transitions_to_surround_delete() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::SurroundDeletePending)
            ));
        }

        #[test]
        fn test_match_pending_mr_transitions_to_surround_replace() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::SurroundReplaceFromPending)
            ));
        }

        #[test]
        fn test_match_pending_ma_transitions_to_text_object_around() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::TextObjectAroundPending)
            ));
        }

        #[test]
        fn test_match_pending_mi_transitions_to_text_object_inside() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::TextObjectInsidePending)
            ));
        }

        #[test]
        fn test_match_pending_invalid_cancels() {
            let result = KeyHandler::handle_key(
                &MatchPending,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod surround_add_pending_handler_tests {
        use super::*;

        #[test]
        fn test_surround_add_pending_accept_char() {
            let result = KeyHandler::handle_key(
                &SurroundAddPending,
                KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "ms("));
        }

        #[test]
        fn test_surround_add_pending_accept_bracket() {
            let result = KeyHandler::handle_key(
                &SurroundAddPending,
                KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "ms["));
        }

        #[test]
        fn test_surround_add_pending_accept_quote() {
            let result = KeyHandler::handle_key(
                &SurroundAddPending,
                KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "ms\""));
        }

        #[test]
        fn test_surround_add_pending_escape_cancels() {
            let result = KeyHandler::handle_key(
                &SurroundAddPending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod surround_delete_pending_handler_tests {
        use super::*;

        #[test]
        fn test_surround_delete_pending_accept_char() {
            let result = KeyHandler::handle_key(
                &SurroundDeletePending,
                KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "md("));
        }

        #[test]
        fn test_surround_delete_pending_accept_bracket() {
            let result = KeyHandler::handle_key(
                &SurroundDeletePending,
                KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "md{"));
        }

        #[test]
        fn test_surround_delete_pending_escape_cancels() {
            let result = KeyHandler::handle_key(
                &SurroundDeletePending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod surround_replace_pending_handler_tests {
        use super::*;

        #[test]
        fn test_surround_replace_from_transitions_to_to() {
            let result = KeyHandler::handle_key(
                &SurroundReplaceFromPending,
                KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::SurroundReplaceToPending { from_char: '(' })
            ));
        }

        #[test]
        fn test_surround_replace_from_escape_cancels() {
            let result = KeyHandler::handle_key(
                &SurroundReplaceFromPending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_surround_replace_to_completes() {
            let state = SurroundReplaceToPending { from_char: '(' };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mr(["));
        }

        #[test]
        fn test_surround_replace_to_quotes() {
            let state = SurroundReplaceToPending { from_char: '"' };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mr\"'"));
        }

        #[test]
        fn test_surround_replace_to_escape_cancels() {
            let state = SurroundReplaceToPending { from_char: '(' };
            let result =
                KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod text_object_around_pending_handler_tests {
        use super::*;

        #[test]
        fn test_text_object_around_word() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "maw"));
        }

        #[test]
        fn test_text_object_around_word_big() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "maW"));
        }

        #[test]
        fn test_text_object_around_parens() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "ma("));
        }

        #[test]
        fn test_text_object_around_quotes() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "ma\""));
        }

        #[test]
        fn test_text_object_around_paragraph() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "map"));
        }

        #[test]
        fn test_text_object_around_escape_cancels() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_text_object_around_invalid_cancels() {
            let result = KeyHandler::handle_key(
                &TextObjectAroundPending,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod text_object_inside_pending_handler_tests {
        use super::*;

        #[test]
        fn test_text_object_inside_word() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "miw"));
        }

        #[test]
        fn test_text_object_inside_brackets() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mi["));
        }

        #[test]
        fn test_text_object_inside_braces() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mi{"));
        }

        #[test]
        fn test_text_object_inside_angle_brackets() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Char('<'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mi<"));
        }

        #[test]
        fn test_text_object_inside_single_quote() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mi'"));
        }

        #[test]
        fn test_text_object_inside_backtick() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Char('`'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "mi`"));
        }

        #[test]
        fn test_text_object_inside_escape_cancels() {
            let result = KeyHandler::handle_key(
                &TextObjectInsidePending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod find_char_pending_handler_tests {
        use super::*;

        #[test]
        fn test_find_char_pending_accept_char() {
            let state = FindCharPending {
                find_type: FindType::FindForward,
            };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "fa"));
        }

        #[test]
        fn test_find_char_pending_backward() {
            let state = FindCharPending {
                find_type: FindType::FindBackward,
            };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "Fx"));
        }

        #[test]
        fn test_till_char_pending() {
            let state = FindCharPending {
                find_type: FindType::TillForward,
            };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "te"));
        }

        #[test]
        fn test_find_char_escape_cancels() {
            let state = FindCharPending {
                find_type: FindType::FindForward,
            };
            let result =
                KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod replace_char_pending_handler_tests {
        use super::*;

        #[test]
        fn test_replace_char_pending_accept_char() {
            let result = KeyHandler::handle_key(
                &ReplaceCharPending,
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "ra"));
        }

        #[test]
        fn test_replace_char_pending_enter_newline() {
            let result = KeyHandler::handle_key(
                &ReplaceCharPending,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "r\n"));
        }

        #[test]
        fn test_replace_char_escape_cancels() {
            let result = KeyHandler::handle_key(
                &ReplaceCharPending,
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }
    }

    mod count_pending_handler_tests {
        use super::*;

        #[test]
        fn test_count_pending_more_digits() {
            let state = CountPending { count: 3 };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE),
            );
            assert!(matches!(
                result,
                HandlerResult::Transition(InputState::CountPending { count: 35 })
            ));
        }

        #[test]
        fn test_count_pending_command() {
            let state = CountPending { count: 3 };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Execute(cmd) if cmd == "3j"));
        }

        #[test]
        fn test_count_pending_invalid_command_cancels() {
            let state = CountPending { count: 3 };
            // 'g' is not count-compatible (it's a prefix)
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            );
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_count_pending_escape_cancels() {
            let state = CountPending { count: 5 };
            let result =
                KeyHandler::handle_key(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(matches!(result, HandlerResult::Cancel));
        }

        #[test]
        fn test_count_pending_overflow_protection() {
            // Start with a large count close to overflow
            let state = CountPending {
                count: usize::MAX / 10,
            };
            // Adding another digit should not panic and should clamp to MAX_COUNT
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE),
            );
            // Result should be a transition with count clamped to MAX_COUNT
            match result {
                HandlerResult::Transition(InputState::CountPending { count }) => {
                    assert!(
                        count <= MAX_COUNT,
                        "Count {} should be <= MAX_COUNT {}",
                        count,
                        MAX_COUNT
                    );
                }
                _ => panic!("Expected Transition to CountPending"),
            }
        }

        #[test]
        fn test_count_pending_clamps_at_max() {
            // Test that count is clamped to MAX_COUNT
            let state = CountPending { count: 9999 };
            let result = KeyHandler::handle_key(
                &state,
                KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE),
            );
            // 9999 * 10 + 9 = 99999 which exceeds MAX_COUNT (10000)
            // So it should be clamped to MAX_COUNT
            match result {
                HandlerResult::Transition(InputState::CountPending { count }) => {
                    assert_eq!(count, MAX_COUNT);
                }
                _ => panic!("Expected Transition to CountPending"),
            }
        }
    }

    mod state_machine_tests {
        use super::*;

        #[test]
        fn test_state_machine_initial_state() {
            let sm = InputStateMachine::new();
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_goto_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'g' - transition to GotoPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(sm.state().is_goto_pending());

            // Press 'g' again - execute "gg"
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some(CMD_GOTO_FILE_START));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_find_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'f' - transition to FindCharPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(sm.state().is_find_char_pending());

            // Press 'a' - execute "fa"
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("fa"));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_count_sequence() {
            let mut sm = InputStateMachine::new();

            // Press '1' - transition to CountPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(sm.state().is_count_pending());
            assert_eq!(sm.pending_count(), Some(1));

            // Press '2' - continue building count
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert_eq!(sm.pending_count(), Some(12));

            // Press 'j' - execute "12j"
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("12j"));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_cancel_returns_to_base() {
            let mut sm = InputStateMachine::new();

            // Press 'g' - transition to GotoPending
            sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
            assert!(sm.state().is_goto_pending());

            // Press Escape - cancel
            let result = sm.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(result.is_cancel());
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_reset() {
            let mut sm = InputStateMachine::new();

            sm.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
            assert!(sm.state().is_goto_pending());

            sm.reset();
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_surround_add_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'm' - transition to MatchPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(sm.state().is_match_pending());

            // Press 's' - transition to SurroundAddPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(matches!(sm.state(), InputState::SurroundAddPending));

            // Press '(' - execute "ms("
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("ms("));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_surround_delete_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'm' - transition to MatchPending
            sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            assert!(sm.state().is_match_pending());

            // Press 'd' - transition to SurroundDeletePending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(matches!(sm.state(), InputState::SurroundDeletePending));

            // Press '{' - execute "md{"
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('{'), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("md{"));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_surround_replace_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'm' - transition to MatchPending
            sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            assert!(sm.state().is_match_pending());

            // Press 'r' - transition to SurroundReplaceFromPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(matches!(sm.state(), InputState::SurroundReplaceFromPending));

            // Press '(' - transition to SurroundReplaceToPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(matches!(
                sm.state(),
                InputState::SurroundReplaceToPending { from_char: '(' }
            ));

            // Press '[' - execute "mr(["
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("mr(["));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_surround_escape_cancels() {
            let mut sm = InputStateMachine::new();

            // Go to SurroundAddPending
            sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            sm.process_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
            assert!(matches!(sm.state(), InputState::SurroundAddPending));

            // Press Escape - should cancel
            let result = sm.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(result.is_cancel());
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_surround_pending_predicates() {
            assert!(InputState::SurroundAddPending.is_surround_pending());
            assert!(InputState::SurroundDeletePending.is_surround_pending());
            assert!(InputState::SurroundReplaceFromPending.is_surround_pending());
            assert!(InputState::SurroundReplaceToPending { from_char: '(' }.is_surround_pending());
            assert!(!InputState::Base.is_surround_pending());
            assert!(!InputState::MatchPending.is_surround_pending());
        }

        #[test]
        fn test_state_machine_text_object_around_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'm' - transition to MatchPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(sm.state().is_match_pending());

            // Press 'a' - transition to TextObjectAroundPending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(matches!(sm.state(), InputState::TextObjectAroundPending));

            // Press 'w' - execute "maw"
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("maw"));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_text_object_inside_sequence() {
            let mut sm = InputStateMachine::new();

            // Press 'm' - transition to MatchPending
            sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            assert!(sm.state().is_match_pending());

            // Press 'i' - transition to TextObjectInsidePending
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
            assert!(result.is_transition());
            assert!(matches!(sm.state(), InputState::TextObjectInsidePending));

            // Press '(' - execute "mi("
            let result = sm.process_key(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some("mi("));
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_state_machine_text_object_escape_cancels() {
            let mut sm = InputStateMachine::new();

            // Go to TextObjectAroundPending
            sm.process_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
            sm.process_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
            assert!(matches!(sm.state(), InputState::TextObjectAroundPending));

            // Press Escape - should cancel
            let result = sm.process_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert!(result.is_cancel());
            assert!(sm.state().is_base());
        }

        #[test]
        fn test_text_object_pending_predicates() {
            assert!(InputState::TextObjectAroundPending.is_text_object_pending());
            assert!(InputState::TextObjectInsidePending.is_text_object_pending());
            assert!(!InputState::Base.is_text_object_pending());
            assert!(!InputState::MatchPending.is_text_object_pending());
            assert!(!InputState::SurroundAddPending.is_text_object_pending());
        }
    }

    mod typestate_handler_tests {
        use super::*;

        #[test]
        fn test_typestate_handler_base_to_goto() {
            let handler = TypestateHandler::<BaseState>::base();
            let (result, next) =
                handler.process_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));

            assert!(result.is_transition());
            assert!(matches!(next, TypestateHandlerState::GotoPending(_)));
        }

        #[test]
        fn test_typestate_handler_state_process() {
            let mut state = TypestateHandlerState::new();
            assert!(state.is_base());

            // Press 'z' - transition to ViewPending
            let (result, next) =
                state.process_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
            assert!(result.is_transition());
            state = next;
            assert!(matches!(state, TypestateHandlerState::ViewPending(_)));

            // Press 'z' - execute "zz"
            let (result, next) =
                state.process_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
            assert!(result.is_execute());
            assert_eq!(result.command(), Some(CMD_VIEW_CENTER));
            state = next;
            assert!(state.is_base());
        }
    }
}
