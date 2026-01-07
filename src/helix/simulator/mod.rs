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

pub mod commands;
pub mod find_state;
mod insert_mode;
mod mode;
pub mod search_state;
mod undo;
pub mod view_state;

#[cfg(test)]
mod tests;

use crate::game::{CursorPosition, EditorState};
use crate::helix::repeat::RepeatBuffer;
use crate::security::UserError;
use helix_core::{Rope, Selection, Transaction};
use std::marker::PhantomData;

// Re-export mode typestate markers
pub use mode::{EditorMode, InsertMode, NormalMode};

// Re-export state types
pub use find_state::{FindDirection, FindState, FindType};
pub use search_state::{SearchDirection, SearchState};
pub use view_state::ViewState;

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

    /// Search state for /, ?, n, N, *, Alt-* commands
    pub(super) search_state: SearchState,

    /// View state for z, zt, zb, zm, zj, zk commands
    pub(super) view_state: ViewState,

    /// Find state for f/F/t/T and Alt-. commands
    pub(super) find_state: FindState,

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
        let range = self.selection.primary();
        let head = range.head;
        let anchor = range.anchor;

        // Clamp to valid bounds
        let max_pos = self.doc.len_chars();
        let head_clamped = head.min(max_pos);

        // Convert head position to (line, col)
        // Note: We use `head` (not cursor()) for backward compatibility with scenarios.
        // The `head` represents the actual selection head position, while `cursor()`
        // returns the visual block cursor position which is head-1 for forward selections.
        let line = self.doc.char_to_line(head_clamped);
        let line_start = self.doc.line_to_char(line);
        let col = head_clamped - line_start;

        // Extract selection if anchor != head (non-empty selection)
        let selection = if anchor != head {
            let anchor_clamped = anchor.min(max_pos);
            let anchor_line = self.doc.char_to_line(anchor_clamped);
            let anchor_line_start = self.doc.line_to_char(anchor_line);
            let anchor_col = anchor_clamped - anchor_line_start;

            // Create selection with start being the smaller position
            let (start_line, start_col, end_line, end_col) = if anchor_clamped < head {
                (anchor_line, anchor_col, line, col)
            } else {
                (line, col, anchor_line, anchor_col)
            };

            Some(crate::game::Selection::new(
                CursorPosition::new(start_line, start_col).map_err(UserError::from)?,
                CursorPosition::new(end_line, end_col).map_err(UserError::from)?,
            ))
        } else {
            None
        };

        EditorState::new(
            self.doc.to_string(),
            CursorPosition::new(line, col).map_err(UserError::from)?,
            selection,
        )
        .map_err(UserError::from)
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
            search_state: SearchState::new(),
            view_state: ViewState::new(),
            find_state: FindState::new(),
            _mode: PhantomData,
        }
    }

    /// Create a new simulator from an EditorState (starts in Normal mode)
    ///
    /// Initializes the simulator with the content, cursor position, and optional selection
    /// from the EditorState. This is useful when starting from a scenario setup.
    pub fn from_editor_state(state: &EditorState) -> Self {
        let rope = Rope::from(state.content());
        let lines: Vec<&str> = state.content().lines().collect();

        // Helper to convert (row, col) to absolute char position
        let pos_to_char = |row: usize, col: usize| -> usize {
            if row == 0 {
                col
            } else {
                let mut pos = 0;
                for line_idx in 0..row {
                    if line_idx < lines.len() {
                        pos += lines[line_idx].chars().count() + 1; // +1 for newline
                    }
                }
                pos + col
            }
        };

        // Convert cursor position
        let cursor = state.cursor_position();
        let char_pos = pos_to_char(cursor.row, cursor.col);

        // Ensure position is within bounds
        let max_pos = rope.len_chars().saturating_sub(1);
        let safe_pos = char_pos.min(max_pos);

        // Handle selection if present
        let selection = if let Some(sel) = state.selection() {
            let anchor = pos_to_char(sel.start.row, sel.start.col);
            let head = pos_to_char(sel.end.row, sel.end.col);
            let safe_anchor = anchor.min(rope.len_chars());
            let safe_head = head.min(rope.len_chars());
            Selection::single(safe_anchor, safe_head)
        } else {
            Selection::point(safe_pos)
        };

        Self {
            doc: rope,
            selection,
            history: Vec::new(),
            clipboard: None,
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
            repeat_depth: 0,
            search_state: SearchState::new(),
            view_state: ViewState::new(),
            find_state: FindState::new(),
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
            search_state: self.search_state,
            view_state: self.view_state,
            find_state: self.find_state,
            _mode: PhantomData,
        }
    }

    /// Get a reference to the search state
    pub fn search_state(&self) -> &SearchState {
        &self.search_state
    }

    /// Get a mutable reference to the search state
    pub fn search_state_mut(&mut self) -> &mut SearchState {
        &mut self.search_state
    }

    /// Get a reference to the view state
    pub fn view_state(&self) -> &ViewState {
        &self.view_state
    }

    /// Get a mutable reference to the view state
    pub fn view_state_mut(&mut self) -> &mut ViewState {
        &mut self.view_state
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
            search_state: self.search_state,
            view_state: self.view_state,
            find_state: self.find_state,
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
                    // Execute each command in sequence
                    // This handles compound actions like x+d (select line + delete)
                    execute_key_sequence(self, keys)
                }
            }
            crate::helix::repeat::RepeatableAction::InsertSequence {
                entry_command,
                text,
                movements,
            } => {
                // Execute insert sequence through wrapper
                use crate::helix::commands::*;

                // Enter insert mode with the same command that was used originally
                let insert_cmd = entry_command.as_deref().unwrap_or(CMD_INSERT);
                commands::execute_command_any_mode(self, insert_cmd)?;

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

/// Execute a sequence of KeyEvents as commands
///
/// This handles both simple commands and compound actions (e.g., x+d for select line + delete).
/// Commands are parsed and executed in order, with multi-key sequences (gg, rx) handled properly.
fn execute_key_sequence(
    sim: &mut AnyModeSimulator,
    keys: &[crossterm::event::KeyEvent],
) -> Result<(), UserError> {
    use crate::helix::commands::*;
    use crossterm::event::KeyCode;

    if keys.is_empty() {
        return Ok(());
    }

    let mut i = 0;
    while i < keys.len() {
        let key = &keys[i];

        // Check for multi-key command patterns
        let cmd = if i + 1 < keys.len() {
            if let (KeyCode::Char(ch1), KeyCode::Char(ch2)) = (key.code, keys[i + 1].code) {
                match (ch1, ch2) {
                    ('g', 'g') => {
                        i += 2;
                        Some(CMD_GOTO_FILE_START.to_string())
                    }
                    ('r', _) => {
                        i += 2;
                        Some(format!("r{}", ch2))
                    }
                    ('f', _) | ('F', _) | ('t', _) | ('T', _) => {
                        i += 2;
                        Some(format!("{}{}", ch1, ch2))
                    }
                    ('g', 'h') | ('g', 'l') | ('g', 's') | ('g', 'e') => {
                        i += 2;
                        Some(format!("{}{}", ch1, ch2))
                    }
                    _ => None, // Not a multi-key sequence, try single
                }
            } else {
                None
            }
        } else {
            None
        };

        // If no multi-key match, process as single key
        let cmd = cmd.unwrap_or_else(|| {
            i += 1;
            match key.code {
                KeyCode::Char(ch) => ch.to_string(),
                KeyCode::Esc => CMD_ESCAPE.to_string(),
                KeyCode::Backspace => CMD_BACKSPACE.to_string(),
                KeyCode::Left => CMD_ARROW_LEFT.to_string(),
                KeyCode::Right => CMD_ARROW_RIGHT.to_string(),
                KeyCode::Up => CMD_ARROW_UP.to_string(),
                KeyCode::Down => CMD_ARROW_DOWN.to_string(),
                _ => String::new(), // Unknown - skip
            }
        });

        if !cmd.is_empty() {
            commands::execute_command_any_mode(sim, &cmd)?;
        }
    }

    Ok(())
}

/// Convert a sequence of KeyEvents back to a command string (legacy, kept for tests)
///
/// This reconstructs the original command from the recorded KeyEvent sequence.
/// Handles both single-key commands (`x`, `i`, etc.) and multi-key sequences (`gg`, `rx`).
///
/// # Errors
///
/// Returns an error if:
/// - The key sequence is empty
/// - The key sequence is unrecognized (unsupported multi-key command)
/// - The key code is not a known command
#[allow(dead_code)]
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
