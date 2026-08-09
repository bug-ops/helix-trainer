//! Macro recording and replay (`q`/`Q`) for the Helix simulator
//!
//! `q` toggles recording of successfully-executed command strings into an
//! unnamed macro register; `Q` replays them by feeding each one back through
//! `execute_command_any_mode` - the same dispatch path as live input, so
//! replay never duplicates dispatch logic.
//!
//! # Relationship to `.`-repeat
//!
//! This is a second, independent recorder alongside [`crate::helix::repeat`].
//! The two are recorded through fundamentally different call sites and
//! shapes for real reasons: `.`-repeat stores `KeyEvent`s (needed to
//! faithfully replay count prefixes and mode-specific insert sequences),
//! while macros store resolved command *strings*, since the KeyEvent
//! conversion (`cmd_to_key_events`) is lossy for parameterised commands like
//! `:goto 3` or `s <pattern>`. Unifying them is deferred; see the `TODO`
//! below.
//!
//! `.` pressed while recording is captured, but not as the literal `.`
//! character - `execute_repeat_impl` replays the last action by calling
//! `execute_command_any_mode` once per expanded command, and each of those
//! calls hits the recording tap independently (with `is_replaying == false`,
//! since macro-replay state and repeat-replay state are separate flags).
//! Recording therefore captures the *expansion* of `.`, not `.` itself. This
//! is deterministic (it doesn't depend on repeat-buffer state at replay
//! time), which is arguably better than literal-`.` capture would be.
//!
//! `Q` pressed while recording is, by contrast, a deliberate no-op rather
//! than a nested replay: replayed commands run with `is_replaying == true`,
//! which is exactly what makes [`MacroRecorder::record`] skip them, so a
//! `Q`-triggered replay's effects would apply to the document while not
//! being captured into the macro currently being recorded - the resulting
//! macro would silently not match what the user watched happen on screen.
//! `execute_command_any_mode` checks `is_recording_macro()` before calling
//! `execute_macro_replay()` and skips the call entirely when recording is
//! active.
//!
//! # Known limitation
//!
//! Macro state lives on the simulator, not the screen. Bare `q` is consumed
//! by the Task/MiniGame screens themselves in a few states (MiniGame
//! game-over -> `QuitApp`, MiniGame paused -> `MiniGameBackToMenu`; see
//! `input/handlers.rs`), so pausing or ending a session mid-recording
//! silently drops the in-progress recording along with the rest of the
//! simulator state. Not addressed here - out of scope for this change.
//!
//! # Deferred
//!
//! - TODO: named macro registers (`"<reg>q`) - `RegisterFile` stores a
//!   single `String` per register, not a `Vec<String>`, so this needs a
//!   register-file change beyond this MVP's single unnamed register.
//! - TODO: unifying `.`-repeat's recorder with this one - see "Relationship
//!   to `.`-repeat" above; tracked as a follow-up, not attempted here.

use crate::security::limits::MAX_MACRO_LENGTH;

/// Maximum macro-replay recursion depth
///
/// Kept as a counter separate from `.`-repeat's `MAX_REPEAT_DEPTH`
/// (`simulator/mod.rs`) rather than sharing it: the two budgets bound
/// different recursion, and conflating them would let a macro replay
/// borrow the repeat budget or vice versa. In practice `Q` is skipped
/// outright while already replaying (see [`MacroRecorder::begin_replay`]),
/// so depth cannot exceed 1 through that path today; this cap is defense in
/// depth against future changes to that invariant.
const MAX_MACRO_DEPTH: usize = 10;

/// Records and replays a single unnamed `q`/`Q` macro
#[derive(Debug, Default)]
pub struct MacroRecorder {
    /// `Some` while recording; accumulates command strings as they execute
    recording: Option<Vec<String>>,
    /// The last macro stopped-and-stored by `q`, ready for `Q` to replay
    stored: Vec<String>,
    /// Set for the duration of a `Q` replay, so the recording tap in
    /// `execute_command_any_mode` does not capture replayed commands
    is_replaying: bool,
    /// Current macro-replay recursion depth
    replay_depth: usize,
}

impl MacroRecorder {
    /// Create a new, empty recorder (nothing recorded, nothing stored)
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a macro is currently being recorded
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Whether a macro replay (`Q`) is currently in progress
    pub fn is_replaying(&self) -> bool {
        self.is_replaying
    }

    /// Toggle recording (`q`): start if idle, stop-and-store if active
    ///
    /// Deliberately unconditional: stopping a recording that captured
    /// nothing (e.g. `q` immediately followed by `q`) overwrites `stored`
    /// with the empty recording, discarding any previously-stored macro.
    /// This matches real Helix's `q`/`Q` parity - there is no separate
    /// "cancel recording without saving" gesture.
    pub fn toggle(&mut self) {
        match self.recording.take() {
            Some(commands) => self.stored = commands,
            None => self.recording = Some(Vec::new()),
        }
    }

    /// Record a successfully-executed command, if currently recording and
    /// not itself part of a replay.
    ///
    /// Silently stops accepting further commands once `MAX_MACRO_LENGTH` is
    /// reached, keeping what was already captured rather than failing or
    /// panicking.
    pub fn record(&mut self, cmd: &str) {
        if self.is_replaying {
            return;
        }
        if let Some(commands) = self.recording.as_mut()
            && commands.len() < MAX_MACRO_LENGTH
        {
            commands.push(cmd.to_string());
        }
    }

    /// The commands stored by the last completed recording
    pub fn stored(&self) -> &[String] {
        &self.stored
    }

    /// Attempt to begin a replay.
    ///
    /// Returns `false` (and changes nothing) if there is nothing stored, a
    /// replay is already in progress, or the depth budget is exhausted.
    /// Callers must pair a successful `begin_replay` with [`Self::end_replay`].
    pub fn begin_replay(&mut self) -> bool {
        if self.is_replaying || self.stored.is_empty() || self.replay_depth >= MAX_MACRO_DEPTH {
            return false;
        }
        self.is_replaying = true;
        self.replay_depth += 1;
        true
    }

    /// End a replay started by a successful [`Self::begin_replay`]
    pub fn end_replay(&mut self) {
        self.is_replaying = false;
        self.replay_depth = self.replay_depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_recorder_is_idle() {
        let recorder = MacroRecorder::new();
        assert!(!recorder.is_recording());
        assert!(!recorder.is_replaying());
        assert!(recorder.stored().is_empty());
    }

    #[test]
    fn toggle_starts_and_stops_recording() {
        let mut recorder = MacroRecorder::new();
        recorder.toggle();
        assert!(recorder.is_recording());

        recorder.record("h");
        recorder.record("d");
        recorder.toggle();

        assert!(!recorder.is_recording());
        assert_eq!(recorder.stored(), ["h", "d"]);
    }

    #[test]
    fn record_is_noop_when_not_recording() {
        let mut recorder = MacroRecorder::new();
        recorder.record("h");
        recorder.toggle(); // start
        recorder.toggle(); // stop, nothing was recorded before start
        assert!(recorder.stored().is_empty());
    }

    #[test]
    fn record_stops_accepting_past_max_length_without_panicking() {
        let mut recorder = MacroRecorder::new();
        recorder.toggle();
        for _ in 0..MAX_MACRO_LENGTH + 50 {
            recorder.record("h");
        }
        recorder.toggle();
        assert_eq!(recorder.stored().len(), MAX_MACRO_LENGTH);
    }

    #[test]
    fn begin_replay_fails_when_nothing_stored() {
        let mut recorder = MacroRecorder::new();
        assert!(!recorder.begin_replay());
    }

    #[test]
    fn begin_replay_fails_while_already_replaying() {
        let mut recorder = MacroRecorder::new();
        recorder.toggle();
        recorder.record("h");
        recorder.toggle();

        assert!(recorder.begin_replay());
        assert!(!recorder.begin_replay());

        recorder.end_replay();
        assert!(recorder.begin_replay());
    }

    #[test]
    fn record_is_noop_during_replay() {
        let mut recorder = MacroRecorder::new();
        recorder.toggle();
        recorder.record("h");
        recorder.toggle();

        recorder.begin_replay();
        recorder.record("j"); // must not be captured
        recorder.end_replay();

        assert_eq!(recorder.stored(), ["h"]);
    }

    #[test]
    fn end_replay_depth_never_underflows() {
        let mut recorder = MacroRecorder::new();
        // end_replay without a matching begin_replay must not panic.
        recorder.end_replay();
        recorder.end_replay();
        assert!(!recorder.is_replaying());
    }
}
