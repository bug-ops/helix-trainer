//! Helix text editor simulator using helix-core primitives
//!
//! This module provides a HelixSimulator that uses the helix-core library
//! for text editing operations. It ensures unicode-correct handling of
//! graphemes, supports multi-cursor operations, and maintains undo history.
//!
//! # Type-Safe Modes
//!
//! The simulator uses the typestate pattern to enforce mode-specific operations
//! at compile time. See the `mode` module for details.

mod commands;
mod insert_mode;
mod mode;
mod undo;

#[cfg(test)]
mod tests;

use crate::game::{CursorPosition, EditorState};
use crate::helix::repeat::RepeatBuffer;
use crate::security::UserError;
use helix_core::{Rope, Selection, Transaction};
use std::marker::PhantomData;

// Re-export mode typestate markers
pub use mode::{EditorMode, InsertMode, NormalMode};

// Re-export old Mode enum for backward compatibility during migration
pub use Mode::*;

/// Maximum recursion depth for repeat command to prevent infinite loops
/// This allows for reasonable chaining (e.g., recording a repeat within a macro)
/// while preventing stack overflow from accidental infinite recursion
const MAX_REPEAT_DEPTH: usize = 100;

/// Editor mode (Normal or Insert)
///
/// Controls which operations are available and how input is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Normal mode: execute commands
    Normal,
    /// Insert mode: insert characters
    Insert,
}

/// Helix editor simulator using helix-core text primitives
///
/// Provides a faithful simulation of Helix editor operations with proper
/// unicode handling, undo/redo support, and multi-cursor awareness.
///
/// # Generic Parameter
///
/// The `M` type parameter represents the editor mode using the typestate pattern:
/// - `HelixSimulator<NormalMode>` - Normal mode (default)
/// - `HelixSimulator<InsertMode>` - Insert mode
///
/// Mode-specific methods are only available in the appropriate mode,
/// enforced at compile time.
pub struct HelixSimulator<M: EditorMode = NormalMode> {
    /// Text buffer (using Rope for efficient edits)
    pub(super) doc: Rope,

    /// Current selection(s) with head and anchor positions
    pub(super) selection: Selection,

    /// Undo history stack storing both transactions and previous document states
    pub(super) history: Vec<(Transaction, Rope)>,

    /// Clipboard for yank and paste operations
    pub(super) clipboard: Option<String>,

    /// Repeat buffer for recording and replaying actions
    pub(super) repeat_buffer: RepeatBuffer,

    /// Flag to prevent recording during repeat execution
    pub(super) is_repeating: bool,

    /// Current recursion depth for repeat command (protects against infinite loops)
    pub(super) repeat_depth: usize,

    /// Phantom data for typestate mode marker (zero-cost)
    _mode: PhantomData<M>,
}

// Methods available in ALL modes
impl<M: EditorMode> HelixSimulator<M> {
    /// Get current editor mode name
    pub fn mode_name(&self) -> &'static str {
        M::name()
    }

    /// Get current editor state
    pub fn get_state(&self) -> Result<EditorState, UserError> {
        let mut head = self.selection.primary().head;

        // Clamp cursor to valid bounds (sometimes helix-core can put it past end)
        let max_pos = self.doc.len_chars();
        if head > max_pos {
            head = max_pos;
        }

        // Convert head position to (line, col)
        let line = self.doc.char_to_line(head);
        let line_start = self.doc.line_to_char(line);
        let col = head - line_start;

        EditorState::new(
            self.doc.to_string(),
            CursorPosition::new(line, col).map_err(|_| UserError::OperationFailed)?,
            None,
        )
        .map_err(|_| UserError::OperationFailed)
    }

    /// Convert simulator state to EditorState (alias for get_state)
    pub fn to_editor_state(&self) -> Result<EditorState, UserError> {
        self.get_state()
    }

    /// Get a reference to the repeat buffer
    ///
    /// Allows inspection of the last recorded action for debugging or testing.
    pub fn repeat_buffer(&self) -> &RepeatBuffer {
        &self.repeat_buffer
    }

    /// Apply transaction and save history
    pub(super) fn apply_transaction(&mut self, transaction: Transaction) {
        // Save previous state before applying transaction
        let prev_doc = self.doc.clone();
        self.history.push((transaction.clone(), prev_doc));
        transaction.apply(&mut self.doc);
    }
}

// Methods ONLY available in NormalMode
impl HelixSimulator<NormalMode> {
    /// Create a new simulator with initial content (starts in Normal mode)
    pub fn new(content: String) -> Self {
        Self {
            doc: Rope::from(content.as_str()),
            selection: Selection::point(0),
            history: Vec::new(),
            clipboard: None,
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
            repeat_depth: 0,
            _mode: PhantomData,
        }
    }

    /// Create a new simulator from an EditorState (starts in Normal mode)
    ///
    /// Initializes the simulator with the content and cursor position from the EditorState.
    /// This is useful when starting from a scenario setup.
    pub fn from_editor_state(state: &EditorState) -> Self {
        let rope = Rope::from(state.content());

        // Convert (row, col) to absolute char position
        let cursor = state.cursor_position();
        let char_pos = if cursor.row == 0 {
            cursor.col
        } else {
            // Find the character position by navigating through lines
            let mut pos = 0;
            let lines: Vec<&str> = state.content().lines().collect();

            // Add characters from all previous lines (including newlines)
            for line_idx in 0..cursor.row {
                if line_idx < lines.len() {
                    pos += lines[line_idx].chars().count() + 1; // +1 for newline
                }
            }
            // Add column offset in current line
            pos + cursor.col
        };

        // Ensure position is within bounds
        let max_pos = rope.len_chars().saturating_sub(1);
        let safe_pos = char_pos.min(max_pos);

        Self {
            doc: rope,
            selection: Selection::point(safe_pos),
            history: Vec::new(),
            clipboard: None,
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
            repeat_depth: 0,
            _mode: PhantomData,
        }
    }

    /// Transition to Insert mode
    pub fn enter_insert_mode(self) -> HelixSimulator<InsertMode> {
        HelixSimulator {
            doc: self.doc,
            selection: self.selection,
            history: self.history,
            clipboard: self.clipboard,
            repeat_buffer: self.repeat_buffer,
            is_repeating: self.is_repeating,
            repeat_depth: self.repeat_depth,
            _mode: PhantomData,
        }
    }
}

// Methods ONLY available in InsertMode
impl HelixSimulator<InsertMode> {
    /// Transition to Normal mode
    pub fn exit_insert_mode(self) -> HelixSimulator<NormalMode> {
        HelixSimulator {
            doc: self.doc,
            selection: self.selection,
            history: self.history,
            clipboard: self.clipboard,
            repeat_buffer: self.repeat_buffer,
            is_repeating: self.is_repeating,
            repeat_depth: self.repeat_depth,
            _mode: PhantomData,
        }
    }
}

/// Runtime mode-switching wrapper for HelixSimulator
///
/// This enum allows dynamic mode switching while maintaining the compile-time
/// safety of the typed simulators internally. Use this when you need to store
/// a simulator that can change modes at runtime (e.g., in GameSession).
pub enum AnyModeSimulator {
    /// Simulator in Normal mode
    Normal(HelixSimulator<NormalMode>),
    /// Simulator in Insert mode
    Insert(HelixSimulator<InsertMode>),
}

impl AnyModeSimulator {
    /// Create a new simulator in Normal mode
    pub fn new(content: String) -> Self {
        Self::Normal(HelixSimulator::new(content))
    }

    /// Create from an EditorState in Normal mode
    pub fn from_editor_state(state: &EditorState) -> Self {
        Self::Normal(HelixSimulator::from_editor_state(state))
    }

    /// Check if currently in Insert mode
    pub fn is_insert_mode(&self) -> bool {
        matches!(self, Self::Insert(_))
    }

    /// Get current mode as enum
    pub fn mode(&self) -> Mode {
        match self {
            Self::Normal(_) => Mode::Normal,
            Self::Insert(_) => Mode::Insert,
        }
    }

    /// Get current mode name
    pub fn mode_name(&self) -> &'static str {
        match self {
            Self::Normal(_) => NormalMode::name(),
            Self::Insert(_) => InsertMode::name(),
        }
    }

    /// Execute a command, handling mode transitions
    pub fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        commands::execute_command_any_mode(self, cmd)
    }

    /// Get current editor state
    pub fn to_editor_state(&self) -> Result<EditorState, UserError> {
        match self {
            Self::Normal(sim) => sim.to_editor_state(),
            Self::Insert(sim) => sim.to_editor_state(),
        }
    }

    /// Get current editor state (alias for to_editor_state)
    pub fn get_state(&self) -> Result<EditorState, UserError> {
        self.to_editor_state()
    }

    /// Get reference to repeat buffer
    pub fn repeat_buffer(&self) -> &RepeatBuffer {
        match self {
            Self::Normal(sim) => sim.repeat_buffer(),
            Self::Insert(sim) => sim.repeat_buffer(),
        }
    }
}

// Repeat command implementation for AnyModeSimulator
impl AnyModeSimulator {
    /// Execute the repeat (`.`) command
    ///
    /// Replays the last recorded action. If no action has been recorded,
    /// this is a no-op. The repeat command itself is never recorded.
    pub(super) fn execute_repeat(&mut self) -> Result<(), UserError> {
        self.execute_repeat_impl()
    }

    /// Execute repeat (internal implementation with depth/state tracking)
    fn execute_repeat_impl(&mut self) -> Result<(), UserError> {
        // Get the last action (if any)
        let action = match self {
            Self::Normal(sim) => {
                // Check recursion depth
                if sim.repeat_depth >= MAX_REPEAT_DEPTH {
                    return Ok(());
                }
                sim.repeat_buffer.last_action().cloned()
            }
            Self::Insert(_) => return Ok(()), // Can't repeat in insert mode
        };

        let action = match action {
            Some(a) => a,
            None => return Ok(()),
        };

        // Set repeating flag and increment depth
        if let Self::Normal(sim) = self {
            sim.is_repeating = true;
            sim.repeat_depth += 1;
        }

        // Execute action
        let result = match &action {
            crate::helix::repeat::RepeatableAction::Command {
                keys,
                expected_mode,
            } => {
                // Validate mode
                if expected_mode != &Mode::Normal {
                    Ok(()) // No-op: requires Normal mode
                } else {
                    // Convert keys to command string
                    let cmd = key_events_to_cmd(keys)?;
                    // Execute through wrapper to handle mode transitions
                    commands::execute_command_any_mode(self, &cmd)
                }
            }
            crate::helix::repeat::RepeatableAction::InsertSequence { text, movements } => {
                // Execute insert sequence through wrapper
                use crate::helix::commands::*;

                // Enter insert mode
                commands::execute_command_any_mode(self, CMD_INSERT)?;

                // Insert text character by character
                for ch in text.chars() {
                    commands::execute_command_any_mode(self, &ch.to_string())?;
                }

                // Apply movements
                for movement in movements {
                    let cmd = match movement {
                        crate::helix::repeat::Movement::Left => CMD_ARROW_LEFT,
                        crate::helix::repeat::Movement::Right => CMD_ARROW_RIGHT,
                        crate::helix::repeat::Movement::Up => CMD_ARROW_UP,
                        crate::helix::repeat::Movement::Down => CMD_ARROW_DOWN,
                    };
                    commands::execute_command_any_mode(self, cmd)?;
                }

                // Exit insert mode
                commands::execute_command_any_mode(self, CMD_ESCAPE)?;
                Ok(())
            }
        };

        // Reset repeating flag and depth
        if let Self::Normal(sim) = self {
            sim.is_repeating = false;
            sim.repeat_depth -= 1;
        }

        result
    }
}

/// Convert a sequence of KeyEvents back to a command string
///
/// This reconstructs the original command from the recorded KeyEvent sequence.
/// Handles both single-key commands (`x`, `i`, etc.) and multi-key sequences (`dd`, `gg`, `rx`).
///
/// # Errors
///
/// Returns an error if:
/// - The key sequence is empty
/// - The key sequence is unrecognized (unsupported multi-key command)
/// - The key code is not a known command
fn key_events_to_cmd(keys: &[crossterm::event::KeyEvent]) -> Result<String, UserError> {
    use crate::helix::commands::*;
    use crossterm::event::KeyCode;

    if keys.is_empty() {
        return Err(UserError::OperationFailed);
    }

    // Handle multi-key sequences
    if keys.len() == 2
        && let (KeyCode::Char(ch1), KeyCode::Char(ch2)) = (keys[0].code, keys[1].code)
    {
        // Check for known multi-key commands
        return match (ch1, ch2) {
            ('d', 'd') => Ok(CMD_DELETE_LINE.to_string()),
            ('g', 'g') => Ok(CMD_GOTO_FILE_START.to_string()),
            ('r', _) => Ok(format!("r{}", ch2)), // Replace command
            _ => Err(UserError::OperationFailed), // Unknown multi-key sequence
        };
    }

    // Single key command
    if keys.len() == 1 {
        return match keys[0].code {
            KeyCode::Char(ch) => Ok(ch.to_string()),
            KeyCode::Esc => Ok(CMD_ESCAPE.to_string()),
            KeyCode::Backspace => Ok(CMD_BACKSPACE.to_string()),
            KeyCode::Left => Ok(CMD_ARROW_LEFT.to_string()),
            KeyCode::Right => Ok(CMD_ARROW_RIGHT.to_string()),
            KeyCode::Up => Ok(CMD_ARROW_UP.to_string()),
            KeyCode::Down => Ok(CMD_ARROW_DOWN.to_string()),
            _ => Err(UserError::OperationFailed), // Unknown key code
        };
    }

    // Unsupported key sequence length (3+ keys)
    Err(UserError::OperationFailed)
}

// Implement CommandExecutor trait for AnyModeSimulator
impl super::executor::CommandExecutor for AnyModeSimulator {
    fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        self.execute_command(cmd)
    }

    fn to_editor_state(&self) -> Result<EditorState, UserError> {
        self.to_editor_state()
    }

    fn mode(&self) -> Mode {
        self.mode()
    }
}
