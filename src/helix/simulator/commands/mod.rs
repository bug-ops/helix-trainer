//! Command execution and dispatch

mod clipboard;
mod editing;
mod movement;

use super::{HelixSimulator, Mode};
use crate::security::UserError;

/// Execute a Helix command
///
/// Routes commands to appropriate handlers based on mode and command type.
pub(super) fn execute_command(sim: &mut HelixSimulator, cmd: &str) -> Result<(), UserError> {
    // In Insert mode, handle special keys and text input
    if sim.mode == Mode::Insert {
        return match cmd {
            "Escape" => {
                sim.mode = Mode::Normal;
                Ok(())
            }
            "Backspace" => sim.backspace(),
            "ArrowLeft" => movement::move_left(sim, 1),
            "ArrowRight" => movement::move_right(sim, 1),
            "ArrowUp" => movement::move_up(sim, 1),
            "ArrowDown" => movement::move_down(sim, 1),
            _ => sim.insert_text(cmd),
        };
    }

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

    Ok(())
}
