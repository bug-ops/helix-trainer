//! Command execution and dispatch

mod clipboard;
mod editing;
mod movement;

use super::{HelixSimulator, Mode};
use crate::helix::repeat::is_repeatable_command;
use crate::security::UserError;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Convert a command string to KeyEvents
///
/// This helper converts string commands (like "dd", "x", "gg") back into
/// the KeyEvent sequence that would have generated them.
fn cmd_to_key_events(cmd: &str) -> Vec<KeyEvent> {
    match cmd {
        // Multi-key sequences
        "dd" => vec![
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        ],
        "gg" => vec![
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
        ],
        // Special keys
        "Escape" => vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)],
        "Backspace" => vec![KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)],
        "ArrowLeft" => vec![KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)],
        "ArrowRight" => vec![KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)],
        "ArrowUp" => vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)],
        "ArrowDown" => vec![KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)],
        // Replace command (e.g., "rx" -> r + x)
        cmd if cmd.starts_with('r') && cmd.len() == 2 => {
            let ch = cmd.chars().nth(1).unwrap();
            vec![
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
            ]
        }
        // Single character commands
        _ => {
            if let Some(ch) = cmd.chars().next()
                && cmd.len() == 1
            {
                vec![KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)]
            } else {
                // Unknown or complex command - return empty
                Vec::new()
            }
        }
    }
}

/// Execute a Helix command
///
/// Routes commands to appropriate handlers based on mode and command type.
/// If the command is repeatable, it will be recorded in the repeat buffer.
pub(super) fn execute_command(sim: &mut HelixSimulator, cmd: &str) -> Result<(), UserError> {
    // Convert command to KeyEvents for potential recording
    let key_events = cmd_to_key_events(cmd);

    // Determine if we should record this command (before execution)
    // Only record in Normal mode for repeatable commands
    let should_record = !key_events.is_empty()
        && sim.mode == Mode::Normal
        && key_events.iter().all(is_repeatable_command);

    // Store mode before execution (for recording)
    let mode_before = sim.mode;

    // Check if we're entering insert mode (for insert recording)
    let entering_insert =
        sim.mode == Mode::Normal && matches!(cmd, "i" | "a" | "I" | "A" | "o" | "O" | "c");

    // In Insert mode, handle special keys and text input
    if sim.mode == Mode::Insert {
        let result = match cmd {
            "Escape" => {
                // Finish insert mode recording before exiting
                let action = sim.repeat_buffer.insert_recorder_mut().finish();
                sim.repeat_buffer.set_last_action(action);
                sim.mode = Mode::Normal;
                Ok(())
            }
            "Backspace" => {
                // Record backspace as deleted character (not implemented in recorder yet)
                sim.backspace()
            }
            "ArrowLeft" => {
                let result = movement::move_left(sim, 1);
                if result.is_ok() {
                    sim.repeat_buffer
                        .insert_recorder_mut()
                        .record_movement(crate::helix::repeat::Movement::Left);
                }
                result
            }
            "ArrowRight" => {
                let result = movement::move_right(sim, 1);
                if result.is_ok() {
                    sim.repeat_buffer
                        .insert_recorder_mut()
                        .record_movement(crate::helix::repeat::Movement::Right);
                }
                result
            }
            "ArrowUp" => {
                let result = movement::move_up(sim, 1);
                if result.is_ok() {
                    sim.repeat_buffer
                        .insert_recorder_mut()
                        .record_movement(crate::helix::repeat::Movement::Up);
                }
                result
            }
            "ArrowDown" => {
                let result = movement::move_down(sim, 1);
                if result.is_ok() {
                    sim.repeat_buffer
                        .insert_recorder_mut()
                        .record_movement(crate::helix::repeat::Movement::Down);
                }
                result
            }
            _ => {
                let result = sim.insert_text(cmd);
                if result.is_ok() {
                    // Record each character
                    for ch in cmd.chars() {
                        sim.repeat_buffer.insert_recorder_mut().record_char(ch);
                    }
                }
                result
            }
        };
        return result;
    }

    // Execute the command in Normal mode
    match cmd {
        // Movement commands - single character
        "h" => movement::move_left(sim, 1)?,
        "l" => movement::move_right(sim, 1)?,
        "j" => movement::move_down(sim, 1)?,
        "k" => movement::move_up(sim, 1)?,

        // Word movement
        "w" => movement::move_next_word_start(sim, 1)?,
        "b" => movement::move_prev_word_start(sim, 1)?,
        "e" => movement::move_next_word_end(sim, 1)?,

        // Line movement
        "0" => movement::move_line_start(sim)?,
        "$" => movement::move_line_end(sim)?,

        // Document movement
        "gg" => movement::move_document_start(sim)?,
        "G" => movement::move_document_end(sim)?,

        // Deletion commands
        "x" => editing::delete_char(sim)?,
        "dd" => editing::delete_line(sim)?,
        "c" => sim.change_selection()?,
        "J" => editing::join_lines(sim)?,

        // Indentation
        ">" => editing::indent_line(sim)?,
        "<" => editing::dedent_line(sim)?,

        // Yank and paste
        "y" => clipboard::yank(sim)?,
        "p" => clipboard::paste_after(sim)?,
        "P" => clipboard::paste_before(sim)?,

        // Mode changes and editing
        "i" => {
            sim.mode = Mode::Insert;
        }
        "a" => sim.append()?,
        "I" => sim.insert_at_line_start()?,
        "A" => sim.append_at_line_end()?,
        "o" => sim.open_below()?,
        "O" => sim.open_above()?,
        "Escape" => {
            sim.mode = Mode::Normal;
        }

        // Character operations
        cmd if cmd.starts_with('r') && cmd.len() == 2 => {
            let ch = cmd.chars().nth(1).unwrap();
            sim.replace_char(ch)?;
        }

        // Undo/Redo
        "u" => sim.undo()?,
        "U" => sim.redo()?,
        "ctrl-r" => sim.redo()?, // Alternative redo binding

        // Unknown command
        _ => return Err(UserError::OperationFailed),
    }

    // If command succeeded and should be recorded, record it
    if should_record {
        // Convert Mode from simulator to repeat module
        let repeat_mode = match mode_before {
            Mode::Normal => crate::helix::repeat::Mode::Normal,
            Mode::Insert => crate::helix::repeat::Mode::Insert,
        };
        sim.repeat_buffer.record_command(key_events, repeat_mode);
    }

    // If we just entered insert mode, start recording
    if entering_insert {
        sim.repeat_buffer.insert_recorder_mut().start();
    }

    Ok(())
}
