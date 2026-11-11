//! Basic simulator creation and initialization tests

use crate::helix::error::SimulatorError;
use crate::helix::simulator::{AnyModeSimulator, Mode};
use std::assert_matches;

#[test]
fn test_create_simulator() {
    let sim = AnyModeSimulator::new("hello world".to_string());
    let state = sim.display();
    assert_eq!(state.content(), "hello world");
    assert_eq!(state.cursor_position().0, 0);
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_initial_mode() {
    let sim = AnyModeSimulator::new("test".to_string());
    assert_eq!(sim.mode(), Mode::Normal);
}

#[test]
fn test_unknown_command() {
    let mut sim = AnyModeSimulator::new("test".to_string());
    assert_matches!(
        sim.execute_command("unknown"),
        Err(SimulatorError::UnknownCommand { command }) if command == "unknown"
    );
}
