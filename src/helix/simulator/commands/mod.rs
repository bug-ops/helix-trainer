//! Command execution and dispatch

mod clipboard;
mod editing;
mod movement;

use super::{AnyModeSimulator, HelixSimulator, InsertMode, Mode, NormalMode};
use crate::helix::commands::*;
use crate::helix::repeat::is_repeatable_command;
use crate::security::UserError;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Convert a command string to KeyEvents
///
/// This helper converts string commands (like "dd", "x", "gg") back into
/// the KeyEvent sequence that would have generated them.
fn cmd_to_key_events(cmd: &str) -> Vec<KeyEvent> {
    // Multi-key sequences
    if cmd == CMD_DELETE_LINE {
        return vec![
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        ];
    }
    if cmd == CMD_GOTO_FILE_START {
        return vec![
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
        ];
    }

    // Special keys
    if cmd == CMD_ESCAPE {
        return vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)];
    }
    if cmd == CMD_BACKSPACE {
        return vec![KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_LEFT {
        return vec![KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_RIGHT {
        return vec![KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_UP {
        return vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_DOWN {
        return vec![KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)];
    }

    // Replace command (e.g., "rx" -> r + x)
    if cmd.starts_with('r') && cmd.len() == 2 {
        let ch = cmd.chars().nth(1).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ];
    }

    // Single character commands
    // Check length first for performance (cheaper than iterator operations)
    if cmd.len() == 1
        && let Some(ch) = cmd.chars().next()
    {
        vec![KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)]
    } else {
        // Unknown or complex command - return empty
        Vec::new()
    }
}

/// Check if a command is an insert mode entry command
///
/// Returns true for commands that transition from Normal to Insert mode.
fn is_insert_command(cmd: &str) -> bool {
    cmd == CMD_INSERT
        || cmd == CMD_APPEND
        || cmd == CMD_INSERT_LINE_START
        || cmd == CMD_APPEND_LINE_END
        || cmd == CMD_OPEN_BELOW
        || cmd == CMD_OPEN_ABOVE
        || cmd == CMD_CHANGE
}

/// Execute a command in Insert mode (internal)
///
/// Handles text input, special keys (Escape, Backspace), and arrow key navigation.
/// Records actions in the insert mode recorder unless we're currently repeating.
fn execute_insert_mode_command_internal(
    sim: &mut HelixSimulator<InsertMode>,
    cmd: &str,
) -> Result<(), UserError> {
    if cmd == CMD_ESCAPE {
        // Finish insert mode recording before exiting (unless repeating)
        if !sim.is_repeating {
            let action = sim.repeat_buffer.insert_recorder_mut().finish();
            sim.repeat_buffer.set_last_action(action);
        }
        // Mode transition handled by caller
        Ok(())
    } else if cmd == CMD_BACKSPACE {
        sim.backspace()
    } else if cmd == CMD_ARROW_LEFT {
        let result = movement::move_left(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Left);
        }
        result
    } else if cmd == CMD_ARROW_RIGHT {
        let result = movement::move_right(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Right);
        }
        result
    } else if cmd == CMD_ARROW_UP {
        let result = movement::move_up(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Up);
        }
        result
    } else if cmd == CMD_ARROW_DOWN {
        let result = movement::move_down(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Down);
        }
        result
    } else {
        // Regular text input
        let result = sim.insert_text(cmd);
        if result.is_ok() && !sim.is_repeating {
            // Record each character
            for ch in cmd.chars() {
                sim.repeat_buffer.insert_recorder_mut().record_char(ch);
            }
        }
        result
    }
}

/// Execute a command in Normal mode (internal, public for repeat functionality)
///
/// Routes commands to appropriate handlers (movement, editing, clipboard, etc.).
/// Returns an error for unknown commands. Does NOT handle mode transitions.
pub(super) fn execute_normal_mode_command_internal(
    sim: &mut HelixSimulator<NormalMode>,
    cmd: &str,
) -> Result<(), UserError> {
    // Movement commands - single character
    if cmd == CMD_MOVE_LEFT {
        movement::move_left(sim, 1)?;
    } else if cmd == CMD_MOVE_RIGHT {
        movement::move_right(sim, 1)?;
    } else if cmd == CMD_MOVE_DOWN {
        movement::move_down(sim, 1)?;
    } else if cmd == CMD_MOVE_UP {
        movement::move_up(sim, 1)?;
    }
    // Word movement
    else if cmd == CMD_MOVE_WORD_FORWARD {
        movement::move_next_word_start(sim, 1)?;
    } else if cmd == CMD_MOVE_WORD_BACKWARD {
        movement::move_prev_word_start(sim, 1)?;
    } else if cmd == CMD_MOVE_WORD_END {
        movement::move_next_word_end(sim, 1)?;
    }
    // Line movement
    else if cmd == CMD_MOVE_LINE_START {
        movement::move_line_start(sim)?;
    } else if cmd == CMD_MOVE_LINE_END {
        movement::move_line_end(sim)?;
    }
    // Document movement
    else if cmd == CMD_GOTO_FILE_START {
        movement::move_document_start(sim)?;
    } else if cmd == CMD_GOTO_FILE_END {
        movement::move_document_end(sim)?;
    }
    // Deletion commands
    else if cmd == CMD_DELETE_CHAR {
        editing::delete_char(sim)?;
    } else if cmd == CMD_DELETE_LINE {
        editing::delete_line(sim)?;
    } else if cmd == CMD_CHANGE {
        sim.change_selection()?;
    } else if cmd == CMD_JOIN_LINES {
        editing::join_lines(sim)?;
    }
    // Indentation
    else if cmd == CMD_INDENT {
        editing::indent_line(sim)?;
    } else if cmd == CMD_DEDENT {
        editing::dedent_line(sim)?;
    }
    // Yank and paste
    else if cmd == CMD_YANK {
        clipboard::yank(sim)?;
    } else if cmd == CMD_PASTE_AFTER {
        clipboard::paste_after(sim)?;
    } else if cmd == CMD_PASTE_BEFORE {
        clipboard::paste_before(sim)?;
    }
    // Mode changes and editing (these prepare for mode transition but don't execute it)
    else if cmd == CMD_INSERT {
        // Mode transition handled by caller
    } else if cmd == CMD_APPEND {
        sim.append()?;
    } else if cmd == CMD_INSERT_LINE_START {
        sim.insert_at_line_start()?;
    } else if cmd == CMD_APPEND_LINE_END {
        sim.append_at_line_end()?;
    } else if cmd == CMD_OPEN_BELOW {
        sim.open_below()?;
    } else if cmd == CMD_OPEN_ABOVE {
        sim.open_above()?;
    } else if cmd == CMD_ESCAPE {
        // No-op in normal mode
    }
    // Character operations - replace command (e.g., "rx")
    else if cmd.starts_with('r') && cmd.len() == 2 {
        let ch = cmd.chars().nth(1).unwrap();
        sim.replace_char(ch)?;
    }
    // Repeat last action - this shouldn't reach here but handle gracefully
    else if cmd == CMD_REPEAT {
        // Repeat is handled specially by execute_command_any_mode
        // If we reach here, something is wrong but don't error
        return Ok(());
    }
    // Undo/Redo
    else if cmd == CMD_UNDO {
        sim.undo()?;
    } else if cmd == CMD_REDO {
        sim.redo()?;
    } else if cmd == "ctrl-r" {
        // Alternative redo binding
        sim.redo()?;
    } else {
        // Unknown command
        return Err(UserError::OperationFailed);
    }

    Ok(())
}

/// Record command in repeat buffer if needed (Normal mode)
///
/// Records normal mode commands if:
/// - Command has valid key events
/// - We're not currently repeating
/// - All keys are repeatable
///
/// Also starts insert mode recording if entering insert mode.
fn record_command_if_needed_normal(
    sim: &mut HelixSimulator<NormalMode>,
    key_events: &[KeyEvent],
    mode_before: Mode,
    entering_insert: bool,
) {
    // Determine if we should record this command
    // Only record for repeatable commands, and NOT during repeat
    let should_record =
        !key_events.is_empty() && !sim.is_repeating && key_events.iter().all(is_repeatable_command);

    if should_record {
        sim.repeat_buffer
            .record_command(key_events.to_vec(), mode_before);
    }

    // If we just entered insert mode, start recording
    if entering_insert {
        sim.repeat_buffer.insert_recorder_mut().start();
    }
}

/// Execute a Helix command on AnyModeSimulator (main entry point)
///
/// Routes commands to appropriate handlers based on mode and command type.
/// Handles mode transitions and repeat buffer recording.
pub(super) fn execute_command_any_mode(
    sim: &mut AnyModeSimulator,
    cmd: &str,
) -> Result<(), UserError> {
    // Handle repeat command specially at the wrapper level
    if cmd == CMD_REPEAT {
        return sim.execute_repeat();
    }

    // For commands that might trigger mode transitions, we need to use take/replace
    // Take the current simulator out, process it, and put back the result
    let placeholder = AnyModeSimulator::Normal(HelixSimulator::new(String::new()));
    let old_sim = std::mem::replace(sim, placeholder);

    let (result, new_sim) = match old_sim {
        AnyModeSimulator::Normal(mut normal_sim) => {
            // Execute in normal mode
            let key_events = cmd_to_key_events(cmd);

            // Determine if this is an insert command
            let is_insert_cmd = is_insert_command(cmd);

            // Only start recording if NOT repeating
            let should_start_recording = !normal_sim.is_repeating && is_insert_cmd;

            let result = execute_normal_mode_command_internal(&mut normal_sim, cmd);

            if result.is_ok() {
                record_command_if_needed_normal(
                    &mut normal_sim,
                    &key_events,
                    Mode::Normal,
                    should_start_recording,
                );

                // Transition to insert mode if this is an insert command
                // (even during repeat - we need to actually enter insert mode)
                if is_insert_cmd {
                    (
                        result,
                        AnyModeSimulator::Insert(normal_sim.enter_insert_mode()),
                    )
                } else {
                    (result, AnyModeSimulator::Normal(normal_sim))
                }
            } else {
                (result, AnyModeSimulator::Normal(normal_sim))
            }
        }
        AnyModeSimulator::Insert(mut insert_sim) => {
            let exiting = cmd == CMD_ESCAPE;
            let result = execute_insert_mode_command_internal(&mut insert_sim, cmd);

            if result.is_ok() && exiting {
                (
                    result,
                    AnyModeSimulator::Normal(insert_sim.exit_insert_mode()),
                )
            } else {
                (result, AnyModeSimulator::Insert(insert_sim))
            }
        }
    };

    // Put the new simulator back
    *sim = new_sim;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for is_insert_command()
    mod is_insert_command_tests {
        use super::*;

        #[test]
        fn test_insert_command_insert() {
            assert!(is_insert_command(CMD_INSERT));
        }

        #[test]
        fn test_insert_command_append() {
            assert!(is_insert_command(CMD_APPEND));
        }

        #[test]
        fn test_insert_command_insert_line_start() {
            assert!(is_insert_command(CMD_INSERT_LINE_START));
        }

        #[test]
        fn test_insert_command_append_line_end() {
            assert!(is_insert_command(CMD_APPEND_LINE_END));
        }

        #[test]
        fn test_insert_command_open_below() {
            assert!(is_insert_command(CMD_OPEN_BELOW));
        }

        #[test]
        fn test_insert_command_open_above() {
            assert!(is_insert_command(CMD_OPEN_ABOVE));
        }

        #[test]
        fn test_insert_command_change() {
            assert!(is_insert_command(CMD_CHANGE));
        }

        #[test]
        fn test_insert_command_movement_not_insert() {
            assert!(!is_insert_command(CMD_MOVE_LEFT));
            assert!(!is_insert_command(CMD_MOVE_RIGHT));
            assert!(!is_insert_command(CMD_MOVE_UP));
            assert!(!is_insert_command(CMD_MOVE_DOWN));
        }

        #[test]
        fn test_insert_command_editing_not_insert() {
            assert!(!is_insert_command(CMD_DELETE_CHAR));
            assert!(!is_insert_command(CMD_DELETE_LINE));
            assert!(!is_insert_command(CMD_YANK));
            assert!(!is_insert_command(CMD_PASTE_AFTER));
        }

        #[test]
        fn test_insert_command_escape_not_insert() {
            assert!(!is_insert_command(CMD_ESCAPE));
        }

        #[test]
        fn test_insert_command_random_string() {
            assert!(!is_insert_command("xyz"));
            assert!(!is_insert_command(""));
        }
    }

    // Unit tests for cmd_to_key_events()
    mod cmd_to_key_events_tests {
        use super::*;

        #[test]
        fn test_cmd_to_key_events_delete_line() {
            let events = cmd_to_key_events(CMD_DELETE_LINE);
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].code, KeyCode::Char('d'));
            assert_eq!(events[1].code, KeyCode::Char('d'));
        }

        #[test]
        fn test_cmd_to_key_events_goto_file_start() {
            let events = cmd_to_key_events(CMD_GOTO_FILE_START);
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].code, KeyCode::Char('g'));
            assert_eq!(events[1].code, KeyCode::Char('g'));
        }

        #[test]
        fn test_cmd_to_key_events_escape() {
            let events = cmd_to_key_events(CMD_ESCAPE);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Esc);
        }

        #[test]
        fn test_cmd_to_key_events_backspace() {
            let events = cmd_to_key_events(CMD_BACKSPACE);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Backspace);
        }

        #[test]
        fn test_cmd_to_key_events_arrow_keys() {
            let left = cmd_to_key_events(CMD_ARROW_LEFT);
            assert_eq!(left.len(), 1);
            assert_eq!(left[0].code, KeyCode::Left);

            let right = cmd_to_key_events(CMD_ARROW_RIGHT);
            assert_eq!(right.len(), 1);
            assert_eq!(right[0].code, KeyCode::Right);

            let up = cmd_to_key_events(CMD_ARROW_UP);
            assert_eq!(up.len(), 1);
            assert_eq!(up[0].code, KeyCode::Up);

            let down = cmd_to_key_events(CMD_ARROW_DOWN);
            assert_eq!(down.len(), 1);
            assert_eq!(down[0].code, KeyCode::Down);
        }

        #[test]
        fn test_cmd_to_key_events_replace_command() {
            let events = cmd_to_key_events("rx");
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].code, KeyCode::Char('r'));
            assert_eq!(events[1].code, KeyCode::Char('x'));
        }

        #[test]
        fn test_cmd_to_key_events_single_char() {
            let events = cmd_to_key_events("h");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Char('h'));

            let events2 = cmd_to_key_events("x");
            assert_eq!(events2.len(), 1);
            assert_eq!(events2[0].code, KeyCode::Char('x'));
        }

        #[test]
        fn test_cmd_to_key_events_empty_string() {
            let events = cmd_to_key_events("");
            assert!(events.is_empty());
        }

        #[test]
        fn test_cmd_to_key_events_unknown_multi_char() {
            let events = cmd_to_key_events("xyz");
            assert!(events.is_empty());
        }

        #[test]
        fn test_cmd_to_key_events_all_modifiers_none() {
            let events = cmd_to_key_events("a");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].modifiers, KeyModifiers::NONE);
        }
    }
}
