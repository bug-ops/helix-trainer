//! Display helpers shared by the gameplay modes

use crate::grammar::commands::{
    CMD_ARROW_DOWN, CMD_ARROW_LEFT, CMD_ARROW_RIGHT, CMD_ARROW_UP, CMD_BACKSPACE, CMD_ESCAPE,
};

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings.
/// This is used by both training and arcade modes for consistent
/// key history display.
///
/// # Examples
///
/// ```
/// use helix_trainer::game::format_key_for_display;
///
/// assert_eq!(format_key_for_display("Left"), "←");
/// assert_eq!(format_key_for_display("Backspace"), "⌫");
/// assert_eq!(format_key_for_display("x"), "x");
/// ```
pub fn format_key_for_display(command: &str) -> String {
    match command {
        CMD_ARROW_LEFT => "←".to_string(),
        CMD_ARROW_RIGHT => "→".to_string(),
        CMD_ARROW_UP => "↑".to_string(),
        CMD_ARROW_DOWN => "↓".to_string(),
        CMD_BACKSPACE => "⌫".to_string(),
        CMD_ESCAPE => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
    }
}
