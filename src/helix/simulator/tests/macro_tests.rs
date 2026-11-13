//! Macro record/replay (`Q`/`q`) integration tests for `HelixSimulator`
//!
//! Verifies recording/replay through the real dispatch path
//! (`AnyModeSimulator::execute_command`), including the case that catches
//! mis-threading of the `macro_recorder` field through the Normal<->Insert
//! struct-literal sites: a macro spanning an insert excursion.

use crate::grammar::commands::*;
use crate::helix::simulator::AnyModeSimulator;

#[test]
fn test_macro_toggle_recording_state() {
    let mut sim = AnyModeSimulator::new("hello".to_string());
    assert!(!sim.is_recording_macro());

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    assert!(sim.is_recording_macro());

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    assert!(!sim.is_recording_macro());
}

#[test]
fn test_macro_records_and_replays_insert_excursion() {
    let mut sim = AnyModeSimulator::new("ab\ncd".to_string());

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // Q: start
    assert!(sim.is_recording_macro());

    sim.execute_command(CMD_APPEND).unwrap(); // a: append after cursor, enters Insert
    sim.execute_command("!").unwrap(); // typed while Insert
    sim.execute_command(CMD_ESCAPE).unwrap(); // back to Normal
    sim.execute_command(CMD_MOVE_DOWN).unwrap(); // j
    sim.execute_command(CMD_GOTO_LINE_START).unwrap(); // gh - normalize column

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // Q: stop
    assert!(!sim.is_recording_macro());

    let state = sim.display();
    assert_eq!(state.content(), "a!b\ncd");

    // Replay from row 1 ("cd"), where the recording left the cursor.
    sim.execute_command(CMD_REPLAY_MACRO).unwrap(); // q

    let state = sim.display();
    assert_eq!(state.content(), "a!b\nc!d");
}

#[test]
fn test_macro_replay_noop_when_nothing_stored() {
    let mut sim = AnyModeSimulator::new("hello".to_string());
    let result = sim.execute_command(CMD_REPLAY_MACRO);
    assert!(result.is_ok());
    assert_eq!(sim.display().content(), "hello");
}

#[test]
fn test_macro_q_types_literal_char_in_insert_mode() {
    let mut sim = AnyModeSimulator::new(String::new());
    sim.execute_command(CMD_INSERT).unwrap(); // i: enter Insert mode
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // 'Q' typed literally

    let state = sim.display();
    assert_eq!(state.content(), "Q");
    assert!(!sim.is_recording_macro());
}

#[test]
fn test_macro_replay_does_not_replay_its_own_toggle_commands() {
    let mut sim = AnyModeSimulator::new("abc".to_string());

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // Q: start
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // only real command recorded
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // Q: stop

    assert!(!sim.is_recording_macro());
    sim.execute_command(CMD_REPLAY_MACRO).unwrap(); // q: replay
    // If 'Q'/'q' had been captured into the macro, replaying it would
    // toggle recording back on.
    assert!(!sim.is_recording_macro());
}

/// Regression test: pressing `q` while recording a *different* macro must
/// not replay the previously-stored one. Before the fix, the document would
/// visibly change (the stored macro's effects applied via
/// `execute_macro_replay`) while `MacroRecorder::record` silently dropped
/// everything during the replay (`is_replaying == true`), so the
/// in-progress recording ended up containing neither the literal `q` nor
/// the replayed macro's expansion - a macro that doesn't match what was
/// seen on screen.
#[test]
fn test_macro_replay_is_noop_while_recording_a_different_macro() {
    let mut sim = AnyModeSimulator::new("xxxx".to_string());

    // Record and store macro A: delete one char.
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    assert_eq!(sim.display().content(), "xxx");

    // Start recording macro B; press Q mid-recording.
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    sim.execute_command(CMD_REPLAY_MACRO).unwrap(); // must be a no-op
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();

    // Document unchanged by the Q press - only the real B commands ran.
    assert_eq!(sim.display().content(), "xxx");

    // Replaying B (delete was NOT part of B, only the move) must not delete.
    sim.execute_command(CMD_REPLAY_MACRO).unwrap();
    assert_eq!(sim.display().content(), "xxx");
}

/// Documents an invariant relevant to the `execute_repeat_impl` two-arm
/// restore (S4/R3): a *stored* macro can never end mid-Insert with no
/// trailing Escape. `Q` only toggles recording while in Normal mode (see
/// `execute_command_any_mode`'s mode gate); pressed while in Insert mode it
/// is inserted as a literal character instead (`test_macro_q_types_literal_char_in_insert_mode`).
/// So the *only* way to stop a recording that entered Insert mode is to
/// `Escape` back to Normal first - and that `Escape` itself gets captured
/// by the recording tap (see `test_macro_records_and_replays_insert_excursion`).
/// A macro replay therefore can only end in Insert mode if a command
/// *within* the macro fails partway through, not from a "successfully
/// recorded, no trailing Escape" macro - that shape cannot be produced via
/// `Q`/`q` at all.
#[test]
fn test_q_while_insert_mode_does_not_stop_recording() {
    let mut sim = AnyModeSimulator::new(String::new());

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // Q: start
    sim.execute_command(CMD_APPEND).unwrap(); // a: enter Insert
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap(); // 'Q' typed literally, not a stop

    assert!(sim.is_insert_mode());
    assert!(sim.is_recording_macro()); // still recording - the toggle never fired
    assert_eq!(sim.display().content(), "Q");
}

#[test]
fn test_dot_repeat_while_recording_captures_expansion_not_literal_dot() {
    let mut sim = AnyModeSimulator::new("aaaa".to_string());

    // Prime the repeat buffer with an Insert session, outside recording.
    sim.execute_command(CMD_INSERT).unwrap();
    sim.execute_command("Z").unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.display().content(), "Zaaaa");

    // Record a macro consisting of just "." (repeat last insert).
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    sim.execute_command(CMD_REPEAT).unwrap(); // expands to another insert
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();

    assert_eq!(sim.display().content(), "ZZaaaa");

    // Replaying the macro inserts again, proving the *expansion* was
    // captured rather than a literal '.' (which would be a no-op replay
    // target here, since CMD_REPEAT itself never reaches the macro-recording
    // tap - it early-returns before it).
    sim.execute_command(CMD_REPLAY_MACRO).unwrap();
    assert_eq!(sim.display().content(), "ZZZaaaa");
}

#[test]
fn test_macro_longer_than_one_hundred_commands_is_stored_whole() {
    const EXTRA_X: usize = 120;
    let mut sim = AnyModeSimulator::new(String::new());

    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    sim.execute_command(CMD_INSERT).unwrap();
    for _ in 0..EXTRA_X {
        sim.execute_command("x").unwrap();
    }
    sim.execute_command(CMD_ESCAPE).unwrap();
    sim.execute_command(CMD_TOGGLE_MACRO_RECORDING).unwrap();
    assert!(!sim.is_insert_mode());

    sim.execute_command(CMD_REPLAY_MACRO).unwrap();

    assert!(!sim.is_insert_mode());
    assert_eq!(sim.display().content().len(), 2 * EXTRA_X);
}
