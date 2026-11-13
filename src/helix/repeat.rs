//! Repeat command (`.`) implementation
//!
//! Models Helix 25.07.1 `repeat_last_insert`: `.` replays only the last
//! Insert session (entry command, typed keys, closing `Escape`). Selections,
//! operators, `>`, `J`, `p` and every other edit are never recorded, and `.`
//! with no recorded Insert session does nothing.
//!
//! # Architecture
//!
//! - `RepeatBuffer`: Stores the last Insert session
//! - `RecordedInsert`: The entry command, typed text and non-text keys of one session
//! - `InsertModeRecorder`: Records insert mode sequences for replay
//!
//! # Security
//!
//! - Insert sessions are recorded in full: a cap would silently truncate the
//!   replayed text and leave later keys with a stale `after_chars`

use super::simulator::InsertCommand;
use crate::grammar::commands::{CMD_ARROW_DOWN, CMD_ARROW_LEFT, CMD_ARROW_RIGHT, CMD_ARROW_UP};

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

/// A non-text Insert-mode key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertAction {
    /// An arrow key
    Move(Movement),
    /// An Insert-mode editing key (`Ctrl-w`, `Ctrl-u`, `Tab`, ...)
    Edit(InsertCommand),
}

impl InsertAction {
    /// The key a command string names (arrows and the Insert-mode editing keys), if any.
    #[must_use]
    pub fn parse(cmd: &str) -> Option<Self> {
        match cmd {
            CMD_ARROW_LEFT => Some(Self::Move(Movement::Left)),
            CMD_ARROW_RIGHT => Some(Self::Move(Movement::Right)),
            CMD_ARROW_UP => Some(Self::Move(Movement::Up)),
            CMD_ARROW_DOWN => Some(Self::Move(Movement::Down)),
            _ => InsertCommand::parse(cmd).map(Self::Edit),
        }
    }

    /// The canonical command string that replays this key
    pub fn command(self) -> &'static str {
        match self {
            Self::Move(Movement::Left) => CMD_ARROW_LEFT,
            Self::Move(Movement::Right) => CMD_ARROW_RIGHT,
            Self::Move(Movement::Up) => CMD_ARROW_UP,
            Self::Move(Movement::Down) => CMD_ARROW_DOWN,
            Self::Edit(command) => command.as_str(),
        }
    }
}

/// A non-text Insert-mode key recorded between typed characters
///
/// `after_chars` is the number of typed characters that preceded the key, so
/// replay can interleave it with the recorded text in its original order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertKey {
    /// Count of typed characters recorded before this key
    pub after_chars: usize,
    /// The key that was pressed
    pub action: InsertAction,
}

/// One recorded Insert session: what `.` replays
///
/// Mirrors Helix `repeat_last_insert`: the entry command, the keys typed in
/// Insert mode and the closing `Escape`. Nothing else is ever repeatable.
///
/// # Fields
///
/// - `entry_command`: The command used to enter insert mode (`i`, `a`, `A`, `I`, `o`, `O`, `c`, `Alt-c`, `"<reg>c`)
/// - `text`: The text that was typed
/// - `keys`: Arrow movements and editing keys (`Ctrl-w`, `Ctrl-u`, `Tab`, ...) in press order, interleaved with `text`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedInsert {
    /// Command that entered Insert mode, if it was not a bare mode switch.
    pub entry_command: Option<String>,
    /// Literal text typed during the session.
    pub text: String,
    /// Non-text keys pressed during the session, in press order.
    pub keys: Vec<InsertKey>,
}

/// Stores the last Insert session for `.`
///
/// Only Insert sessions are recorded; selections, operators and every other
/// edit leave the buffer untouched (FR-066).
#[derive(Debug)]
pub struct RepeatBuffer {
    last_insert: Option<RecordedInsert>,
    insert_recorder: InsertModeRecorder,
}

impl RepeatBuffer {
    /// Create a new empty repeat buffer
    pub fn new() -> Self {
        Self {
            last_insert: None,
            insert_recorder: InsertModeRecorder::new(),
        }
    }

    /// Check if the buffer is empty (no Insert session to repeat)
    pub fn is_empty(&self) -> bool {
        self.last_insert.is_none()
    }

    /// The last recorded Insert session, if any
    pub fn last_insert(&self) -> Option<&RecordedInsert> {
        self.last_insert.as_ref()
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

    /// Store a finished Insert session as the one `.` replays
    pub fn set_last_insert(&mut self, session: RecordedInsert) {
        self.last_insert = Some(session);
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
/// The whole session is recorded without a cap, so `.` replays it exactly;
/// growth is bounded by what the user types.
#[derive(Debug)]
pub struct InsertModeRecorder {
    is_recording: bool,
    entry_command: Option<String>,
    text: String,
    keys: Vec<InsertKey>,
}

impl InsertModeRecorder {
    /// Create a new empty recorder
    pub fn new() -> Self {
        Self {
            is_recording: false,
            entry_command: None,
            text: String::new(),
            keys: Vec::new(),
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
    /// * `entry_command` - The command that entered insert mode (`i`, `a`, `A`, `I`, `o`, `O`, `c`, `Alt-c`, `"<reg>c`)
    pub fn start(&mut self, entry_command: Option<String>) {
        self.is_recording = true;
        self.entry_command = entry_command;
        self.text.clear();
        self.keys.clear();
    }

    /// Record a character typed in insert mode
    ///
    /// Characters are appended to the text buffer.
    pub fn record_char(&mut self, ch: char) {
        if !self.is_recording {
            return;
        }

        self.text.push(ch);
    }

    /// Record an arrow key movement at the current text position
    ///
    /// Keys share one ordered list with editing keys so replay keeps their
    /// original order relative to the text.
    pub fn record_movement(&mut self, movement: Movement) {
        self.record_key(InsertAction::Move(movement));
    }

    /// Record an Insert-mode editing key at the current text position
    ///
    /// Ordered like [`Self::record_movement`].
    pub fn record_edit(&mut self, command: InsertCommand) {
        self.record_key(InsertAction::Edit(command));
    }

    fn record_key(&mut self, action: InsertAction) {
        if !self.is_recording {
            return;
        }

        self.keys.push(InsertKey {
            after_chars: self.text.chars().count(),
            action,
        });
    }

    /// Finish recording and return the recorded action
    ///
    /// Stops recording and returns an [`RecordedInsert`] containing all
    /// recorded text, keys, and the entry command. Resets the recorder to
    /// empty state.
    pub fn finish(&mut self) -> RecordedInsert {
        self.is_recording = false;

        RecordedInsert {
            entry_command: self.entry_command.take(),
            text: std::mem::take(&mut self.text),
            keys: std::mem::take(&mut self.keys),
        }
    }
}

impl Default for InsertModeRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repeat_buffer_default() {
        let buffer = RepeatBuffer::default();
        assert!(buffer.is_empty());
        assert!(buffer.last_insert().is_none());
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
        for ch in "hello".chars() {
            recorder.record_char(ch);
        }

        let session = recorder.finish();
        assert!(!recorder.is_recording());
        assert_eq!(session.entry_command, Some("i".to_string()));
        assert_eq!(session.text, "hello");
        assert_eq!(session.keys, []);
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

        let session = recorder.finish();
        assert_eq!(session.entry_command, Some("a".to_string()));
        assert_eq!(session.text, "hi!");
        let left = InsertAction::Move(Movement::Left);
        assert_eq!(
            session.keys,
            vec![
                InsertKey {
                    after_chars: 2,
                    action: left
                },
                InsertKey {
                    after_chars: 2,
                    action: left
                },
            ]
        );
    }

    #[test]
    fn test_insert_mode_recorder_empty_recording() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);

        let session = recorder.finish();
        assert_eq!(session.entry_command, None);
        assert_eq!(session.text, "");
        assert_eq!(session.keys, []);
    }

    #[test]
    fn test_insert_mode_recorder_no_recording_without_start() {
        let mut recorder = InsertModeRecorder::new();

        recorder.record_char('x');
        recorder.record_movement(Movement::Left);

        let session = recorder.finish();
        assert_eq!(session.entry_command, None);
        assert_eq!(session.text, "");
        assert_eq!(session.keys, []);
    }

    #[test]
    fn test_insert_mode_recorder_keeps_long_text() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);
        for _ in 0..5000 {
            recorder.record_char('x');
        }

        assert_eq!(recorder.finish().text.len(), 5000);
    }

    #[test]
    fn test_insert_mode_recorder_keeps_all_keys_in_order_past_old_caps() {
        let mut recorder = InsertModeRecorder::new();
        recorder.start(None);
        for _ in 0..1100 {
            recorder.record_char('a');
        }
        for _ in 0..150 {
            recorder.record_movement(Movement::Left);
        }
        recorder.record_char('b');
        recorder.record_movement(Movement::Right);

        let RecordedInsert { text, keys, .. } = recorder.finish();
        assert_eq!(text.len(), 1101);
        assert_eq!(keys.len(), 151);
        assert!(keys[..150].iter().all(|k| k.after_chars == 1100));
        assert_eq!(keys[150].after_chars, 1101);
        assert_eq!(keys[150].action, InsertAction::Move(Movement::Right));
    }

    #[test]
    fn test_insert_mode_recorder_resets_after_finish() {
        let mut recorder = InsertModeRecorder::new();

        recorder.start(Some("a".to_string()));
        recorder.record_char('a');
        let _ = recorder.finish();

        recorder.start(Some("i".to_string()));
        recorder.record_char('b');
        let session = recorder.finish();

        assert_eq!(session.entry_command, Some("i".to_string()));
        assert_eq!(session.text, "b");
    }
}
