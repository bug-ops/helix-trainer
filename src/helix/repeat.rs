//! Repeat command (`.`) implementation
//!
//! Allows users to repeat the last editing action. This module provides
//! infrastructure for recording and replaying commands and insert mode sequences.
//!
//! # Architecture
//!
//! - `RepeatBuffer`: Stores the last repeatable action with compound action support
//! - `RepeatableAction`: Enum representing different types of repeatable actions
//! - `InsertModeRecorder`: Records insert mode sequences for replay
//! - `is_selection_command()`: Identifies selection commands that start compound actions
//! - `is_operator_command()`: Identifies operator commands that complete compound actions
//!
//! # Compound Actions
//!
//! In Helix, selection + operator combinations (like `xd` for select line then delete)
//! should be recorded as a single compound action. When `.` is pressed, both the
//! selection and the operator are replayed together.
//!
//! # Security
//!
//! - Insert mode text is limited to 1000 characters
//! - Movements are limited to 100 steps
//! - Pending key sequences limited to 10 keys

use crossterm::event::{KeyCode, KeyEvent};

// Re-export Mode from simulator to avoid duplication
pub use super::simulator::Mode;

/// Maximum length of insert mode text recording (security limit)
const MAX_INSERT_TEXT_LENGTH: usize = 1000;

/// Maximum number of movements in insert mode (security limit)
const MAX_INSERT_MOVEMENTS: usize = 100;

/// Maximum number of keys in a pending compound action (security limit)
const MAX_PENDING_KEYS: usize = 10;

/// Arrow key movement directions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    /// Move cursor left
    Left,
    /// Move cursor right
    Right,
    /// Move cursor up
    Up,
    /// Move cursor down
    Down,
}

/// Represents a repeatable action that can be replayed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatableAction {
    /// Single command or multi-key sequence (e.g., `x`, `dd`, `J`)
    ///
    /// # Fields
    ///
    /// - `keys`: The key sequence that was pressed
    /// - `expected_mode`: The mode the editor should be in before replay
    Command {
        keys: Vec<KeyEvent>,
        expected_mode: Mode,
    },

    /// Insert mode sequence (text + optional movements)
    ///
    /// # Fields
    ///
    /// - `entry_command`: The command used to enter insert mode (`i`, `a`, `A`, `I`, `o`, `O`)
    /// - `text`: The text that was typed
    /// - `movements`: Arrow key movements made during insert
    InsertSequence {
        entry_command: Option<String>,
        text: String,
        movements: Vec<Movement>,
    },
}

/// Stores the last repeatable action
///
/// This buffer maintains a history of one action that can be replayed
/// with the `.` command. It also supports compound actions where a selection
/// command (like `x`) is followed by an operator (like `d`).
#[derive(Debug)]
pub struct RepeatBuffer {
    last_action: Option<RepeatableAction>,
    insert_recorder: InsertModeRecorder,
    /// Pending keys for compound action (selection + operator)
    /// When a selection command is executed, its keys are stored here.
    /// When an operator follows, the pending keys are combined with the operator.
    pending_keys: Vec<KeyEvent>,
}

impl RepeatBuffer {
    /// Create a new empty repeat buffer
    pub fn new() -> Self {
        Self {
            last_action: None,
            insert_recorder: InsertModeRecorder::new(),
            pending_keys: Vec::new(),
        }
    }

    /// Check if the buffer is empty (no action to repeat)
    pub fn is_empty(&self) -> bool {
        self.last_action.is_none()
    }

    /// Get a reference to the last action (if any)
    pub fn last_action(&self) -> Option<&RepeatableAction> {
        self.last_action.as_ref()
    }

    /// Get a mutable reference to the insert mode recorder
    ///
    /// Used internally for insert mode tracking.
    pub fn insert_recorder_mut(&mut self) -> &mut InsertModeRecorder {
        &mut self.insert_recorder
    }

    /// Get a reference to the insert mode recorder
    ///
    /// Used internally for insert mode state checking.
    pub fn insert_recorder(&self) -> &InsertModeRecorder {
        &self.insert_recorder
    }

    /// Record a command action
    ///
    /// For selection commands: stores keys in pending buffer for compound action.
    /// For operator commands: combines with pending selection (if any) and records.
    /// For other commands: records directly and clears pending.
    pub fn record_command(&mut self, keys: Vec<KeyEvent>, mode: Mode) {
        // Check if this is a selection command that starts a compound action
        let is_selection = keys.iter().all(is_selection_command);
        // Check if this is an operator that completes a compound action
        let is_operator = keys.iter().all(is_operator_command);

        if is_selection {
            // Start or extend compound action - add to pending
            if self.pending_keys.len() < MAX_PENDING_KEYS {
                self.pending_keys.extend(keys);
            }
            // Don't record yet - wait for operator
        } else if is_operator && !self.pending_keys.is_empty() {
            // Complete compound action: pending selection + operator
            let mut compound_keys = std::mem::take(&mut self.pending_keys);
            compound_keys.extend(keys);
            self.last_action = Some(RepeatableAction::Command {
                keys: compound_keys,
                expected_mode: mode,
            });
        } else {
            // Regular command - record directly and clear pending
            self.pending_keys.clear();
            self.last_action = Some(RepeatableAction::Command {
                keys,
                expected_mode: mode,
            });
        }
    }

    /// Record a selection command that may start a compound action
    ///
    /// Selection commands (`x`, `X`, `%`) are stored in pending and wait
    /// for an operator to complete the compound action.
    pub fn record_selection(&mut self, keys: Vec<KeyEvent>) {
        if self.pending_keys.len() < MAX_PENDING_KEYS {
            self.pending_keys.extend(keys);
        }
    }

    /// Record an operator command, possibly completing a compound action
    ///
    /// If there are pending selection keys, combines them with the operator.
    /// Otherwise records just the operator.
    pub fn record_operator(&mut self, keys: Vec<KeyEvent>, mode: Mode) {
        if !self.pending_keys.is_empty() {
            // Complete compound action
            let mut compound_keys = std::mem::take(&mut self.pending_keys);
            compound_keys.extend(keys);
            self.last_action = Some(RepeatableAction::Command {
                keys: compound_keys,
                expected_mode: mode,
            });
        } else {
            // Just the operator
            self.last_action = Some(RepeatableAction::Command {
                keys,
                expected_mode: mode,
            });
        }
    }

    /// Clear pending keys without recording
    ///
    /// Called when a non-repeatable command breaks the compound action chain.
    pub fn clear_pending(&mut self) {
        self.pending_keys.clear();
    }

    /// Check if there are pending selection keys
    pub fn has_pending(&self) -> bool {
        !self.pending_keys.is_empty()
    }

    /// Set the last action directly
    ///
    /// Used primarily for storing insert mode sequences after recording.
    /// Also clears pending keys.
    pub fn set_last_action(&mut self, action: RepeatableAction) {
        self.pending_keys.clear();
        self.last_action = Some(action);
    }
}

impl Default for RepeatBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Records insert mode actions for replay
///
/// This recorder tracks text input and cursor movements made during
/// insert mode, so they can be replayed with the `.` command.
///
/// # Security
///
/// - Text recording is limited to 1000 characters
/// - Movement recording is limited to 100 steps
#[derive(Debug)]
pub struct InsertModeRecorder {
    is_recording: bool,
    entry_command: Option<String>,
    text: String,
    movements: Vec<Movement>,
}

impl InsertModeRecorder {
    /// Create a new empty recorder
    pub fn new() -> Self {
        Self {
            is_recording: false,
            entry_command: None,
            text: String::new(),
            movements: Vec::new(),
        }
    }

    /// Check if currently recording
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    /// Begin recording insert mode actions
    ///
    /// Clears any previous recording and starts fresh.
    /// Optionally captures the command used to enter insert mode.
    ///
    /// # Arguments
    /// * `entry_command` - The command that entered insert mode (`i`, `a`, `A`, `I`, `o`, `O`)
    pub fn start(&mut self, entry_command: Option<String>) {
        self.is_recording = true;
        self.entry_command = entry_command;
        self.text.clear();
        self.movements.clear();
    }

    /// Record a character typed in insert mode
    ///
    /// Characters are appended to the text buffer up to the security limit.
    pub fn record_char(&mut self, ch: char) {
        if !self.is_recording {
            return;
        }

        // Security: limit text length
        if self.text.len() < MAX_INSERT_TEXT_LENGTH {
            self.text.push(ch);
        }
    }

    /// Record an arrow key movement in insert mode
    ///
    /// Movements are tracked separately from text so they can be replayed.
    pub fn record_movement(&mut self, movement: Movement) {
        if !self.is_recording {
            return;
        }

        // Security: limit movement count
        if self.movements.len() < MAX_INSERT_MOVEMENTS {
            self.movements.push(movement);
        }
    }

    /// Finish recording and return the recorded action
    ///
    /// Stops recording and returns a `RepeatableAction::InsertSequence`
    /// containing all recorded text, movements, and the entry command. Resets the recorder
    /// to empty state.
    pub fn finish(&mut self) -> RepeatableAction {
        self.is_recording = false;

        let action = RepeatableAction::InsertSequence {
            entry_command: self.entry_command.clone(),
            text: self.text.clone(),
            movements: self.movements.clone(),
        };

        // Reset state
        self.entry_command = None;
        self.text.clear();
        self.movements.clear();

        action
    }
}

impl Default for InsertModeRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a key is a selection command that starts a compound action
///
/// Selection commands modify the selection without changing the document.
/// They can be followed by an operator to form a compound action.
///
/// # Selection Commands
/// - `x` - Select line (extend)
/// - `X` - Extend to line bounds
/// - `%` - Select all
///
/// Note: `;` (collapse selection) is NOT included as it doesn't expand selection
pub fn is_selection_command(key: &KeyEvent) -> bool {
    if let KeyCode::Char(ch) = key.code {
        matches!(ch, 'x' | 'X' | '%')
    } else {
        false
    }
}

/// Check if a key is an operator command that completes a compound action
///
/// Operators act on the current selection. When preceded by a selection
/// command, they form a compound action that can be repeated together.
///
/// # Operator Commands
/// - `d` - Delete selection
/// - `c` - Change selection (delete and enter insert mode)
/// - `y` - Yank (copy) selection
/// - `>` - Indent selection
/// - `<` - Dedent selection
pub fn is_operator_command(key: &KeyEvent) -> bool {
    if let KeyCode::Char(ch) = key.code {
        matches!(ch, 'd' | 'c' | 'y' | '>' | '<')
    } else {
        false
    }
}

/// Check if a command should be recorded for repeat
///
/// Returns `false` for:
/// - `.` (repeat itself)
/// - `u`, `U` (undo/redo)
/// - `F1` (hint)
/// - `Esc` (cancel)
/// - Movement commands: `h`, `j`, `k`, `l`, `w`, `b`, `e`, `0`, `$`, `g`, `G`
///
/// Returns `true` for all editing commands:
/// - Character operations: `x`, `r`
/// - Line operations: `d` (dd), `J`
/// - Insert commands: `i`, `a`, `I`, `A`, `o`, `O`
/// - Change: `c`
/// - Clipboard: `y`, `p`, `P`
/// - Indent: `>`, `<`
pub fn is_repeatable_command(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Char(ch) => match ch {
            // Repeat itself - prevent infinite recursion
            '.' => false,

            // Undo/redo - stateful, should not repeat
            'u' | 'U' => false,

            // Movement commands - not editing actions
            'h' | 'j' | 'k' | 'l' => false,
            'w' | 'b' | 'e' => false,
            '0' | '$' => false,
            'g' | 'G' => false,

            // Selection commands - start compound actions
            'x' => true, // select line
            // Operator commands - complete compound actions
            'd' => true,       // delete line
            'i' | 'a' => true, // insert/append
            'I' | 'A' => true, // insert/append at bounds
            'o' | 'O' => true, // open line
            'r' => true,       // replace char
            'c' => true,       // change
            'J' => true,       // join lines
            'y' => true,       // yank
            'p' | 'P' => true, // paste
            '>' | '<' => true, // indent/dedent

            // Hint key alternative
            '?' => false,

            // All other printable characters are repeatable
            // (they can be used as arguments to 'r' command, or in insert mode)
            _ if ch.is_ascii_graphic() || ch == ' ' => true,

            // Everything else is not repeatable
            _ => false,
        },

        // Function keys
        KeyCode::F(1) => false, // Hint key

        // Special keys
        KeyCode::Esc => false,

        // All other keys are not repeatable
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn make_key(ch: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
    }

    // RepeatBuffer tests
    #[test]
    fn test_repeat_buffer_starts_empty() {
        let buffer = RepeatBuffer::new();
        assert!(buffer.is_empty());
        assert!(buffer.last_action().is_none());
    }

    #[test]
    fn test_repeat_buffer_default() {
        let buffer = RepeatBuffer::default();
        assert!(buffer.is_empty());
    }

    // InsertModeRecorder tests
    #[test]
    fn test_insert_mode_recorder_new() {
        let recorder = InsertModeRecorder::new();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_insert_mode_recorder_default() {
        let recorder = InsertModeRecorder::default();
        assert!(!recorder.is_recording());
    }

    #[test]
    fn test_insert_mode_recorder_start() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);
        assert!(recorder.is_recording());
    }

    #[test]
    fn test_insert_mode_recorder_record_char() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(Some("i".to_string()));

        recorder.record_char('h');
        recorder.record_char('e');
        recorder.record_char('l');
        recorder.record_char('l');
        recorder.record_char('o');

        let action = recorder.finish();
        assert!(!recorder.is_recording());

        match action {
            RepeatableAction::InsertSequence {
                entry_command,
                text,
                movements,
            } => {
                assert_eq!(entry_command, Some("i".to_string()));
                assert_eq!(text, "hello");
                assert!(movements.is_empty());
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    #[test]
    fn test_insert_mode_recorder_record_movement() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(Some("a".to_string()));

        recorder.record_char('h');
        recorder.record_char('i');
        recorder.record_movement(Movement::Left);
        recorder.record_movement(Movement::Left);
        recorder.record_char('!');

        let action = recorder.finish();

        match action {
            RepeatableAction::InsertSequence {
                entry_command,
                text,
                movements,
            } => {
                assert_eq!(entry_command, Some("a".to_string()));
                assert_eq!(text, "hi!");
                assert_eq!(movements.len(), 2);
                assert_eq!(movements[0], Movement::Left);
                assert_eq!(movements[1], Movement::Left);
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    #[test]
    fn test_insert_mode_recorder_empty_recording() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);

        let action = recorder.finish();

        match action {
            RepeatableAction::InsertSequence {
                entry_command,
                text,
                movements,
            } => {
                assert_eq!(entry_command, None);
                assert!(text.is_empty());
                assert!(movements.is_empty());
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    #[test]
    fn test_insert_mode_recorder_no_recording_without_start() {
        let mut recorder = InsertModeRecorder::new();

        // Try to record without starting
        recorder.record_char('x');
        recorder.record_movement(Movement::Left);

        let action = recorder.finish();

        match action {
            RepeatableAction::InsertSequence {
                entry_command,
                text,
                movements,
            } => {
                assert_eq!(entry_command, None);
                assert!(text.is_empty());
                assert!(movements.is_empty());
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    #[test]
    fn test_insert_mode_recorder_max_text_length() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);

        // Record MAX_INSERT_TEXT_LENGTH + 100 characters
        for _ in 0..MAX_INSERT_TEXT_LENGTH + 100 {
            recorder.record_char('x');
        }

        let action = recorder.finish();

        match action {
            RepeatableAction::InsertSequence { text, .. } => {
                // Should be limited to MAX_INSERT_TEXT_LENGTH
                assert_eq!(text.len(), MAX_INSERT_TEXT_LENGTH);
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    #[test]
    fn test_insert_mode_recorder_max_movements() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);

        // Record MAX_INSERT_MOVEMENTS + 10 movements
        for _ in 0..MAX_INSERT_MOVEMENTS + 10 {
            recorder.record_movement(Movement::Left);
        }

        let action = recorder.finish();

        match action {
            RepeatableAction::InsertSequence { movements, .. } => {
                // Should be limited to MAX_INSERT_MOVEMENTS
                assert_eq!(movements.len(), MAX_INSERT_MOVEMENTS);
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    #[test]
    fn test_insert_mode_recorder_resets_after_finish() {
        let mut recorder = InsertModeRecorder::new();

        // First recording
        recorder.start(Some("a".to_string()));
        recorder.record_char('a');
        let _ = recorder.finish();

        // Second recording should start fresh
        recorder.start(Some("i".to_string()));
        recorder.record_char('b');
        let action = recorder.finish();

        match action {
            RepeatableAction::InsertSequence {
                entry_command,
                text,
                ..
            } => {
                assert_eq!(entry_command, Some("i".to_string()));
                assert_eq!(text, "b"); // Should only have 'b', not 'ab'
            }
            _ => panic!("Expected InsertSequence"),
        }
    }

    // is_repeatable_command tests
    #[test]
    fn test_is_repeatable_movement_commands() {
        // All movement commands should NOT be repeatable
        assert!(!is_repeatable_command(&make_key('h')));
        assert!(!is_repeatable_command(&make_key('j')));
        assert!(!is_repeatable_command(&make_key('k')));
        assert!(!is_repeatable_command(&make_key('l')));
        assert!(!is_repeatable_command(&make_key('w')));
        assert!(!is_repeatable_command(&make_key('b')));
        assert!(!is_repeatable_command(&make_key('e')));
        assert!(!is_repeatable_command(&make_key('0')));
        assert!(!is_repeatable_command(&make_key('$')));
        assert!(!is_repeatable_command(&make_key('g')));
        assert!(!is_repeatable_command(&make_key('G')));
    }

    #[test]
    fn test_is_repeatable_editing_commands() {
        // All editing commands SHOULD be repeatable
        assert!(is_repeatable_command(&make_key('x'))); // delete char
        assert!(is_repeatable_command(&make_key('d'))); // delete line (dd)
        assert!(is_repeatable_command(&make_key('i'))); // insert
        assert!(is_repeatable_command(&make_key('a'))); // append
        assert!(is_repeatable_command(&make_key('I'))); // insert at start
        assert!(is_repeatable_command(&make_key('A'))); // append at end
        assert!(is_repeatable_command(&make_key('o'))); // open below
        assert!(is_repeatable_command(&make_key('O'))); // open above
        assert!(is_repeatable_command(&make_key('r'))); // replace
        assert!(is_repeatable_command(&make_key('c'))); // change
        assert!(is_repeatable_command(&make_key('J'))); // join
        assert!(is_repeatable_command(&make_key('y'))); // yank
        assert!(is_repeatable_command(&make_key('p'))); // paste after
        assert!(is_repeatable_command(&make_key('P'))); // paste before
        assert!(is_repeatable_command(&make_key('>'))); // indent
        assert!(is_repeatable_command(&make_key('<'))); // dedent
    }

    #[test]
    fn test_is_repeatable_special_commands() {
        // These should NOT be repeatable
        assert!(!is_repeatable_command(&make_key('.'))); // repeat itself
        assert!(!is_repeatable_command(&make_key('u'))); // undo
        assert!(!is_repeatable_command(&make_key('U'))); // redo

        // Hint keys
        let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert!(!is_repeatable_command(&f1)); // hint (F1)
        let question = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!is_repeatable_command(&question)); // hint (?)

        // Esc key
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!is_repeatable_command(&esc));
    }

    #[test]
    fn test_is_repeatable_printable_characters() {
        // Printable ASCII characters ARE repeatable (used with 'r' command or in insert mode)
        assert!(is_repeatable_command(&make_key('q')));
        assert!(is_repeatable_command(&make_key('z')));
        assert!(is_repeatable_command(&make_key('1')));
        assert!(is_repeatable_command(&make_key('!')));
        assert!(is_repeatable_command(&make_key('_')));
        assert!(is_repeatable_command(&make_key(' ')));

        // But special non-printable keys are not
        assert!(!is_repeatable_command(&KeyEvent::new(
            KeyCode::Tab,
            KeyModifiers::NONE
        )));
    }

    #[test]
    fn test_movement_enum_equality() {
        assert_eq!(Movement::Left, Movement::Left);
        assert_eq!(Movement::Right, Movement::Right);
        assert_eq!(Movement::Up, Movement::Up);
        assert_eq!(Movement::Down, Movement::Down);

        assert_ne!(Movement::Left, Movement::Right);
        assert_ne!(Movement::Up, Movement::Down);
    }

    #[test]
    fn test_repeatable_action_equality() {
        let key = make_key('x');

        let action1 = RepeatableAction::Command {
            keys: vec![key],
            expected_mode: Mode::Normal,
        };

        let action2 = RepeatableAction::Command {
            keys: vec![key],
            expected_mode: Mode::Normal,
        };

        assert_eq!(action1, action2);

        let insert1 = RepeatableAction::InsertSequence {
            entry_command: Some("i".to_string()),
            text: "hello".to_string(),
            movements: vec![Movement::Left],
        };

        let insert2 = RepeatableAction::InsertSequence {
            entry_command: Some("i".to_string()),
            text: "hello".to_string(),
            movements: vec![Movement::Left],
        };

        assert_eq!(insert1, insert2);
    }

    #[test]
    fn test_mode_equality() {
        assert_eq!(Mode::Normal, Mode::Normal);
        assert_eq!(Mode::Insert, Mode::Insert);
        assert_ne!(Mode::Normal, Mode::Insert);
    }
}
