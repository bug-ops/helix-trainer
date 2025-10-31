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

    /// Apply transaction and save history
    pub(super) fn apply_transaction(&mut self, transaction: Transaction) {
        // Save previous state before applying transaction
        let prev_doc = self.doc.clone();
        self.history.push((transaction.clone(), prev_doc));
        transaction.apply(&mut self.doc);
    }
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
