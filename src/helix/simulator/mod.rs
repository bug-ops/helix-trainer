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

pub mod command_line;
pub mod commands;
pub mod find_state;
mod insert_mode;
mod mode;
pub mod register_file;
pub mod search_state;
pub mod snapshot;
mod undo;
pub mod view_state;

#[cfg(test)]
mod tests;

use crate::game::EditorState;
use crate::game::editor_state::{CursorPosition, Selection as GameSelection};
use crate::helix::macro_recorder::MacroRecorder;
use crate::helix::repeat::RepeatBuffer;
use crate::security::UserError;
use helix_core::{Rope, Selection, Transaction};
use std::marker::PhantomData;

// Re-export mode typestate markers
pub use mode::{EditorMode, InsertMode, NormalMode};

// Re-export state types
pub use command_line::CommandLine;
pub use find_state::{FindDirection, FindState, FindType};
pub use register_file::RegisterFile;
pub use search_state::{SearchDirection, SearchState};
pub use snapshot::{EditorDisplay, EditorSnapshot, SelectionBounds, SerializableRange};
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

    /// Undo history stack storing the document state from immediately
    /// before each applied transaction.
    pub(super) history: Vec<Rope>,

    /// Redo stack storing the document state each `undo` moves away from,
    /// so `redo` can restore it. Cleared whenever a new transaction is
    /// applied, matching standard editor undo/redo semantics.
    pub(super) redo_stack: Vec<Rope>,

    /// Named registers for yank, paste, and replace operations
    ///
    /// Plain `y`/`p`/`P`/`R` (with no `"<reg>` prefix) read and write the
    /// unnamed register; see [`RegisterFile`].
    pub(super) registers: RegisterFile,

    /// Repeat buffer for recording and replaying actions
    pub(super) repeat_buffer: RepeatBuffer,

    /// Macro recorder for `q`/`Q` recording and replay
    ///
    /// Lives on the generic `HelixSimulator<M>`, not `NormalMode`-only, and
    /// is threaded across every mode-transition struct literal so that
    /// recording survives an Insert-mode excursion inside a macro.
    pub(super) macro_recorder: MacroRecorder,

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

    /// Create an EditorSnapshot from current simulator state.
    ///
    /// This is the preferred way to capture simulator state for:
    /// - Comparison with target state
    /// - Serialization/persistence
    /// - Test assertions
    pub fn to_snapshot(&self) -> EditorSnapshot {
        EditorSnapshot::from_helix(&self.doc, &self.selection)
    }

    /// Get display facade for UI rendering.
    ///
    /// The display facade provides row/col conversion from char offsets
    /// without copying data. Use this at the UI boundary for rendering.
    pub fn display(&self) -> EditorDisplay<'_> {
        EditorDisplay::new(&self.doc, &self.selection)
    }

    /// Check if simulator state matches a target snapshot.
    ///
    /// Used for completion checking. Performs order-independent
    /// selection comparison for multi-cursor scenarios.
    pub fn matches_snapshot(&self, target: &EditorSnapshot) -> bool {
        self.to_snapshot().matches(target)
    }

    /// Check if simulator state matches another simulator.
    ///
    /// Convenience method for direct simulator-to-simulator comparison.
    pub fn matches<N: EditorMode>(&self, other: &HelixSimulator<N>) -> bool {
        let self_snap = self.to_snapshot();
        let other_snap = other.to_snapshot();
        self_snap.matches(&other_snap)
    }

    /// Get current editor state
    ///
    /// Exports ALL selections from helix_core::Selection to EditorState.
    /// Point selections (anchor == head) are skipped as they represent cursors without selection.
    pub fn state(&self) -> Result<EditorState, UserError> {
        let max_pos = self.doc.len_chars();

        // Helper: convert char index to (row, col)
        let pos_to_row_col = |char_idx: usize| -> (usize, usize) {
            let clamped = char_idx.min(max_pos);
            let line = self.doc.char_to_line(clamped);
            let line_start = self.doc.line_to_char(line);
            let col = clamped - line_start;
            (line, col)
        };

        // Export ALL selections from helix_core::Selection
        let selections: Vec<GameSelection> = self
            .selection
            .ranges()
            .iter()
            .filter_map(|range| {
                let anchor = range.anchor;
                let head = range.head;

                // Skip point selections (anchor == head) - they're just cursors
                if anchor == head {
                    return None;
                }

                let (anchor_row, anchor_col) = pos_to_row_col(anchor);
                let (head_row, head_col) = pos_to_row_col(head);

                // Normalize to ensure start <= end for Selection
                let ((start_row, start_col), (end_row, end_col)) = if anchor < head {
                    ((anchor_row, anchor_col), (head_row, head_col))
                } else {
                    ((head_row, head_col), (anchor_row, anchor_col))
                };

                Some(GameSelection::new(
                    CursorPosition {
                        row: start_row,
                        col: start_col,
                    },
                    CursorPosition {
                        row: end_row,
                        col: end_col,
                    },
                ))
            })
            .collect();

        // Build cursor position from primary range's head
        let primary = self.selection.primary();
        let (cursor_row, cursor_col) = pos_to_row_col(primary.head);
        let cursor = CursorPosition::new(cursor_row, cursor_col).map_err(UserError::from)?;

        // Determine primary selection index
        let primary_idx = self.selection.primary_index();

        if selections.is_empty() {
            // No non-point selections - use simple constructor
            EditorState::new(self.doc.to_string(), cursor, None).map_err(UserError::from)
        } else {
            // Use with_selections for multi-selection support
            // Clamp primary_idx to valid range (may differ if some ranges were skipped)
            let clamped_primary_idx = primary_idx.min(selections.len().saturating_sub(1));
            EditorState::with_selections(
                self.doc.to_string(),
                cursor,
                selections,
                clamped_primary_idx,
            )
            .map_err(UserError::from)
        }
    }

    /// Get a reference to the repeat buffer
    ///
    /// Allows inspection of the last recorded action for debugging or testing.
    pub fn repeat_buffer(&self) -> &RepeatBuffer {
        &self.repeat_buffer
    }

    /// Whether a `q`/`Q` macro is currently being recorded
    pub fn is_recording_macro(&self) -> bool {
        self.macro_recorder.is_recording()
    }

    /// Toggle `q`/`Q` macro recording on/off
    pub(super) fn toggle_macro_recording(&mut self) {
        self.macro_recorder.toggle();
    }

    /// Apply transaction and save history
    ///
    /// No-op transactions (empty change set) are skipped entirely: they
    /// leave the document unchanged, so recording them would create a
    /// spurious undo step and needlessly discard valid redo history.
    pub(super) fn apply_transaction(&mut self, transaction: Transaction) {
        if transaction.changes().is_empty() {
            return;
        }

        // Save previous state before applying transaction
        let prev_doc = self.doc.clone();
        self.history.push(prev_doc);
        transaction.apply(&mut self.doc);
        // A new edit invalidates any previously undone changes
        self.redo_stack.clear();
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
            redo_stack: Vec::new(),
            registers: RegisterFile::new(),
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
            repeat_depth: 0,
            search_state: SearchState::new(),
            view_state: ViewState::new(),
            find_state: FindState::new(),
            macro_recorder: MacroRecorder::new(),
            _mode: PhantomData,
        }
    }

    /// Create a new simulator from an EditorSnapshot (starts in Normal mode)
    ///
    /// This is the preferred way to initialize from serialized state.
    /// Unlike `from_editor_state()`, this uses char offsets directly
    /// without row/col conversion overhead.
    pub fn from_snapshot(snapshot: &EditorSnapshot) -> Self {
        let rope = Rope::from(snapshot.content.as_str());
        let max_pos = rope.len_chars();

        // Convert snapshot selections to helix_core::Selection
        let selection = if snapshot.selections.is_empty() {
            Selection::point(0)
        } else {
            let ranges: Vec<helix_core::Range> = snapshot
                .selections
                .iter()
                .map(|r| {
                    let anchor = r.anchor.min(max_pos);
                    let head = r.head.min(max_pos);
                    helix_core::Range::new(anchor, head)
                })
                .collect();
            let primary_idx = snapshot.primary_idx.min(ranges.len().saturating_sub(1));
            Selection::new(ranges.into(), primary_idx)
        };

        Self {
            doc: rope,
            selection,
            history: Vec::new(),
            redo_stack: Vec::new(),
            registers: RegisterFile::new(),
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
            repeat_depth: 0,
            search_state: SearchState::new(),
            view_state: ViewState::new(),
            find_state: FindState::new(),
            macro_recorder: MacroRecorder::new(),
            _mode: PhantomData,
        }
    }

    /// Create a new simulator from an EditorState (starts in Normal mode)
    ///
    /// Initializes the simulator with the content, cursor position, and ALL selections
    /// from the EditorState. This enables multi-cursor scenarios.
    pub fn from_editor_state(state: &EditorState) -> Self {
        let rope = Rope::from(state.content());

        // Helper to convert (row, col) to absolute char position using Rope API
        let row_col_to_pos = |row: usize, col: usize| -> usize {
            let line_count = rope.len_lines();
            if row >= line_count {
                return rope.len_chars();
            }
            let line_start = rope.line_to_char(row);
            let line_len = rope.line(row).len_chars();
            // Clamp column to line length (excluding newline for position calculation)
            let safe_col = col.min(line_len.saturating_sub(1).max(col.min(line_len)));
            line_start.saturating_add(safe_col).min(rope.len_chars())
        };

        // Convert EditorState selections to helix_core::Selection
        let selections = state.selections();
        let selection = if selections.is_empty() {
            // No selections: create single point range from cursor_position
            let (row, col) = state.cursor_position();
            let char_idx = row_col_to_pos(row, col);
            let max_pos = rope.len_chars();
            let safe_pos = char_idx.min(max_pos);
            Selection::point(safe_pos)
        } else {
            // Convert all selections to helix_core::Range
            let ranges: Vec<helix_core::Range> = selections
                .iter()
                .map(|sel| {
                    let start_idx = row_col_to_pos(sel.start.row, sel.start.col);
                    let end_idx = row_col_to_pos(sel.end.row, sel.end.col);
                    let safe_start = start_idx.min(rope.len_chars());
                    let safe_end = end_idx.min(rope.len_chars());
                    // helix_core::Range: anchor is start, head is end
                    helix_core::Range::new(safe_start, safe_end)
                })
                .collect();

            // Get primary index, clamped to valid range
            let primary_idx = state
                .primary_selection_idx()
                .min(ranges.len().saturating_sub(1));

            // Create Selection with all ranges
            // Note: Selection::new() panics if ranges is empty, but we checked above
            Selection::new(ranges.into(), primary_idx)
        };

        Self {
            doc: rope,
            selection,
            history: Vec::new(),
            redo_stack: Vec::new(),
            registers: RegisterFile::new(),
            repeat_buffer: RepeatBuffer::new(),
            is_repeating: false,
            repeat_depth: 0,
            search_state: SearchState::new(),
            view_state: ViewState::new(),
            find_state: FindState::new(),
            macro_recorder: MacroRecorder::new(),
            _mode: PhantomData,
        }
    }

    /// Transition to Insert mode
    ///
    /// Collapses the selection to a single cursor at the primary range's
    /// head, discarding every other range.
    ///
    /// Known limitation: real Helix keeps every cursor active through
    /// Insert mode (typing/backspacing at each one), but this simulator's
    /// Insert-mode commands (`insert_text`, `backspace`, the arrow-key
    /// handlers) all read/write only `selection.primary()` and overwrite the
    /// whole selection with a single point, not just at entry. Preserving
    /// the full selection here would still leave every secondary cursor
    /// frozen (or panicking on stale offsets) as soon as the user typed, so
    /// the collapse happens up front instead. Fixing this for real means
    /// reworking Insert mode's commands to operate over every range, not
    /// just entry - out of scope for a single-method change.
    pub fn enter_insert_mode(self) -> HelixSimulator<InsertMode> {
        // Collapse selection to cursor position (head of primary range)
        let cursor_pos = self.selection.primary().head;
        let collapsed_selection = Selection::single(cursor_pos, cursor_pos);

        HelixSimulator {
            doc: self.doc,
            selection: collapsed_selection,
            history: self.history,
            redo_stack: self.redo_stack,
            registers: self.registers,
            repeat_buffer: self.repeat_buffer,
            is_repeating: self.is_repeating,
            repeat_depth: self.repeat_depth,
            search_state: self.search_state,
            view_state: self.view_state,
            find_state: self.find_state,
            macro_recorder: self.macro_recorder,
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
            redo_stack: self.redo_stack,
            registers: self.registers,
            repeat_buffer: self.repeat_buffer,
            is_repeating: self.is_repeating,
            repeat_depth: self.repeat_depth,
            search_state: self.search_state,
            view_state: self.view_state,
            find_state: self.find_state,
            macro_recorder: self.macro_recorder,
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

    /// Whether a `q`/`Q` macro is currently being recorded
    pub fn is_recording_macro(&self) -> bool {
        match self {
            Self::Normal(sim) => sim.is_recording_macro(),
            Self::Insert(sim) => sim.is_recording_macro(),
        }
    }

    /// Record a successfully-executed command into the active macro, if
    /// currently recording. No-op if not recording or mid-replay.
    fn record_macro_command(&mut self, cmd: &str) {
        match self {
            Self::Normal(sim) => sim.macro_recorder.record(cmd),
            Self::Insert(sim) => sim.macro_recorder.record(cmd),
        }
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
    pub fn state(&self) -> Result<EditorState, UserError> {
        match self {
            Self::Normal(sim) => sim.state(),
            Self::Insert(sim) => sim.state(),
        }
    }

    /// Get reference to repeat buffer
    pub fn repeat_buffer(&self) -> &RepeatBuffer {
        match self {
            Self::Normal(sim) => sim.repeat_buffer(),
            Self::Insert(sim) => sim.repeat_buffer(),
        }
    }

    /// Create an EditorSnapshot from current simulator state.
    ///
    /// This is the preferred way to capture simulator state for:
    /// - Comparison with target state
    /// - Serialization/persistence
    /// - Test assertions
    pub fn to_snapshot(&self) -> EditorSnapshot {
        match self {
            Self::Normal(sim) => sim.to_snapshot(),
            Self::Insert(sim) => sim.to_snapshot(),
        }
    }

    /// Check if simulator state matches a target snapshot.
    ///
    /// Used for completion checking. Performs order-independent
    /// selection comparison for multi-cursor scenarios. In addition to
    /// content/cursor/selection equality, the editor must be back in
    /// Normal mode: a scenario is only "complete" once the documented
    /// solution has fully run, and every scenario solution that enters
    /// Insert mode returns to Normal mode (via `Escape`) before the
    /// target state is reached. Without this check, a scenario whose
    /// target content is reachable mid-Insert (e.g. `o` immediately
    /// after opening a blank line) would register as complete before
    /// the trailing `Escape` is pressed.
    pub fn matches_snapshot(&self, target: &EditorSnapshot) -> bool {
        self.mode() == Mode::Normal
            && match self {
                Self::Normal(sim) => sim.matches_snapshot(target),
                // Unreachable: the `mode() == Mode::Normal` check above
                // already short-circuits before this arm can run.
                Self::Insert(_) => false,
            }
    }

    /// Create from an EditorSnapshot in Normal mode.
    pub fn from_snapshot(snapshot: &EditorSnapshot) -> Self {
        Self::Normal(HelixSimulator::from_snapshot(snapshot))
    }

    /// Get display facade for UI rendering.
    ///
    /// The display facade provides row/col conversion from char offsets
    /// without copying data. Use this at the UI boundary for rendering.
    pub fn display(&self) -> EditorDisplay<'_> {
        match self {
            Self::Normal(sim) => sim.display(),
            Self::Insert(sim) => sim.display(),
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

        // Set repeating flag and increment depth, capturing the prior value
        // of `is_repeating` so it can be restored (not just cleared) below.
        // Only reachable here from the Normal arm - Insert already returned
        // above.
        let prior_is_repeating = match self {
            Self::Normal(sim) => {
                let prior = sim.is_repeating;
                sim.is_repeating = true;
                sim.repeat_depth += 1;
                prior
            }
            Self::Insert(_) => unreachable!("Insert mode returns Ok(()) above"),
        };

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

        // Restore the repeating flag and depth unconditionally, on whichever
        // mode replay ended in - unlike `.`, a macro replay containing an
        // insert excursion with no trailing Escape can end in Insert mode.
        // Restoring (not clearing) `is_repeating` matters for nested replay:
        // clearing it would drop the outer replay's flag. `saturating_sub`
        // also avoids a debug-build underflow panic if depth is ever 0 here.
        match self {
            Self::Normal(sim) => {
                sim.is_repeating = prior_is_repeating;
                sim.repeat_depth = sim.repeat_depth.saturating_sub(1);
            }
            Self::Insert(sim) => {
                sim.is_repeating = prior_is_repeating;
                sim.repeat_depth = sim.repeat_depth.saturating_sub(1);
            }
        }

        result
    }

    /// Execute the macro replay (`Q`) command
    ///
    /// Replays the stored macro by feeding each recorded command string back
    /// through `execute_command_any_mode` - the same dispatch path as live
    /// input, so replay never duplicates dispatch logic. No-op if nothing is
    /// stored, if already inside a replay (prevents runaway recursion), or
    /// if the replay-depth budget is exhausted. Only callable from Normal
    /// mode - callers must check this first.
    pub(super) fn execute_macro_replay(&mut self) -> Result<(), UserError> {
        let commands = match self {
            Self::Normal(sim) => {
                if !sim.macro_recorder.begin_replay() {
                    return Ok(());
                }
                sim.macro_recorder.stored().to_vec()
            }
            Self::Insert(_) => return Ok(()),
        };

        let mut result = Ok(());
        for cmd in &commands {
            result = commands::execute_command_any_mode(self, cmd);
            if result.is_err() {
                break;
            }
        }

        // Replay may have ended in either mode (e.g. a stored macro that
        // enters Insert with no trailing Escape), so end_replay must run
        // regardless of self's current variant.
        match self {
            Self::Normal(sim) => sim.macro_recorder.end_replay(),
            Self::Insert(sim) => sim.macro_recorder.end_replay(),
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

        // Named register prefix ("<reg><op>, e.g. `"_d`, `"ay`) is a 3-key
        // sequence, so it must be checked before the 2-key patterns below -
        // otherwise the leading '"' would be consumed alone as a bare
        // (invalid) single-key command.
        let register_cmd = if i + 2 < keys.len()
            && key.code == KeyCode::Char('"')
            && let (KeyCode::Char(register), KeyCode::Char(op)) =
                (keys[i + 1].code, keys[i + 2].code)
        {
            i += 3;
            Some(format!("{CMD_SELECT_REGISTER}{register}{op}"))
        } else {
            None
        };

        // Check for multi-key command patterns
        let cmd = if register_cmd.is_some() {
            register_cmd
        } else if i + 1 < keys.len() {
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

// Implement CommandExecutor trait for AnyModeSimulator
impl super::executor::CommandExecutor for AnyModeSimulator {
    fn execute_command(&mut self, cmd: &str) -> Result<(), UserError> {
        self.execute_command(cmd)
    }

    fn state(&self) -> Result<EditorState, UserError> {
        self.state()
    }

    fn mode(&self) -> Mode {
        self.mode()
    }
}
