//! Helix text editor simulator using helix-core primitives
//!
//! This module provides a HelixSimulator that uses the helix-core library
//! for text editing operations. It ensures unicode-correct handling of
//! graphemes, supports multi-cursor operations, and maintains undo history.

mod commands;
mod insert_mode;
mod undo;

#[cfg(test)]
mod tests;

use crate::game::{CursorPosition, EditorState};
use crate::helix::repeat::RepeatBuffer;
use crate::security::UserError;
use helix_core::{Rope, Selection, Transaction};

// Re-export Mode for convenience
pub use Mode::*;

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
pub struct HelixSimulator {
    /// Text buffer (using Rope for efficient edits)
    pub(super) doc: Rope,

    /// Current selection(s) with head and anchor positions
    pub(super) selection: Selection,

    /// Editor mode (Normal or Insert)
    pub(super) mode: Mode,

    /// Undo history stack storing both transactions and previous document states
    pub(super) history: Vec<(Transaction, Rope)>,

    /// Clipboard for yank and paste operations
    pub(super) clipboard: Option<String>,

    /// Repeat buffer for recording and replaying actions
    pub(super) repeat_buffer: RepeatBuffer,

    /// Flag to prevent recording during repeat execution
    pub(super) is_repeating: bool,
}

impl HelixSimulator {
    /// Create a new simulator with initial content
    pub fn new(content: String) -> Self {
        Self {
            doc: Rope::from(content.as_str()),
            selection: Selection::point(0),
            mode: Mode::Normal,
            history: Vec::new(),
            clipboard: None,
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
        }
    }

    /// Create a new simulator from an EditorState
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
            mode: Mode::Normal,
            history: Vec::new(),
            clipboard: None,
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
        }
    }

    /// Execute a Helix command
    ///
    /// Routes command to appropriate handler based on current mode.
    pub fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        commands::execute_command(self, cmd)
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

    /// Get current mode
    pub fn mode(&self) -> Mode {
        self.mode
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

    /// Execute the repeat (`.`) command
    ///
    /// Replays the last recorded action. If no action has been recorded,
    /// this is a no-op. The repeat command itself is never recorded.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The replayed command execution fails
    /// - Mode validation fails (though we skip silently for mode mismatches)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut sim = HelixSimulator::new("hello".to_string());
    ///
    /// // Delete a character
    /// sim.execute_command("x").unwrap();
    /// assert_eq!(sim.text(), "ello");
    ///
    /// // Repeat the delete
    /// sim.execute_command(".").unwrap();
    /// assert_eq!(sim.text(), "llo");
    /// ```
    pub(super) fn execute_repeat(&mut self) -> Result<(), UserError> {
        // Get the last action (if any)
        let action = match self.repeat_buffer.last_action() {
            Some(action) => action.clone(), // Clone to avoid borrow issues
            None => return Ok(()),          // No action to repeat - no-op
        };

        // Set flag to prevent recording during repeat
        // Use RAII pattern to ensure flag is always reset
        self.is_repeating = true;
        let result = self.execute_repeat_inner(&action);
        self.is_repeating = false;
        result
    }

    /// Internal repeat execution - allows proper RAII cleanup of is_repeating flag
    fn execute_repeat_inner(
        &mut self,
        action: &crate::helix::repeat::RepeatableAction,
    ) -> Result<(), UserError> {
        match action {
            crate::helix::repeat::RepeatableAction::Command {
                keys,
                expected_mode,
            } => {
                // Validate mode
                let current_mode = match self.mode {
                    Mode::Normal => crate::helix::repeat::Mode::Normal,
                    Mode::Insert => crate::helix::repeat::Mode::Insert,
                };

                // If mode doesn't match, this is a no-op (Vim/Helix semantics)
                // Example: Last action was in normal mode, but we're now in insert mode
                // User would need to Esc first before repeating
                if &current_mode != expected_mode {
                    return Ok(()); // No-op: repeat requires correct mode
                }

                // Convert all keys to a command string
                let cmd = key_events_to_cmd(keys)?;
                self.execute_command(&cmd)?;
                Ok(())
            }

            crate::helix::repeat::RepeatableAction::InsertSequence { text, movements } => {
                use crate::helix::commands::*;

                // Enter insert mode
                self.execute_command(CMD_INSERT)?;

                // Insert text character by character
                for ch in text.chars() {
                    self.execute_command(&ch.to_string())?;
                }

                // Apply movements
                for movement in movements {
                    match movement {
                        crate::helix::repeat::Movement::Left => {
                            self.execute_command(CMD_ARROW_LEFT)?
                        }
                        crate::helix::repeat::Movement::Right => {
                            self.execute_command(CMD_ARROW_RIGHT)?
                        }
                        crate::helix::repeat::Movement::Up => self.execute_command(CMD_ARROW_UP)?,
                        crate::helix::repeat::Movement::Down => {
                            self.execute_command(CMD_ARROW_DOWN)?
                        }
                    }
                }

                // Exit insert mode
                self.execute_command(CMD_ESCAPE)?;
                Ok(())
            }
        }
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

// Implement CommandExecutor trait for HelixSimulator
impl super::executor::CommandExecutor for HelixSimulator {
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
