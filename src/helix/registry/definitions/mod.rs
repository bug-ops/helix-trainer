//! Command definitions
//!
//! This module contains all command registrations organized by category.

pub mod clipboard;
pub mod editing;
pub mod movement;
pub mod selection;

use crate::helix::registry::command_registry::CommandRegistry;
use crate::helix::simulator::NormalMode;

/// Register all normal mode commands
pub fn register_all(registry: &mut CommandRegistry<NormalMode>) {
    movement::register(registry);
    movement::register_parametric_metadata(registry);
    editing::register(registry);
    clipboard::register(registry);
    selection::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_all_commands() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register_all(&mut registry);

        // Should have registered many commands
        assert!(
            registry.len() >= 40,
            "Expected at least 40 commands, got {}",
            registry.len()
        );
    }

    #[test]
    fn test_movement_commands_registered() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register_all(&mut registry);

        // Check key movement commands
        assert!(registry.contains("h"), "Missing move_left");
        assert!(registry.contains("j"), "Missing move_down");
        assert!(registry.contains("k"), "Missing move_up");
        assert!(registry.contains("l"), "Missing move_right");
        assert!(registry.contains("w"), "Missing move_word_forward");
        assert!(registry.contains("b"), "Missing move_word_backward");
        assert!(registry.contains("gg"), "Missing goto_file_start");
        assert!(registry.contains("G"), "Missing goto_file_end");
    }

    #[test]
    fn test_editing_commands_registered() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register_all(&mut registry);

        assert!(registry.contains("d"), "Missing delete_selection");
        assert!(registry.contains("c"), "Missing change");
        assert!(registry.contains("J"), "Missing join_lines");
        assert!(registry.contains("i"), "Missing insert");
        assert!(registry.contains("a"), "Missing append");
    }

    #[test]
    fn test_clipboard_commands_registered() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register_all(&mut registry);

        assert!(registry.contains("y"), "Missing yank");
        assert!(registry.contains("p"), "Missing paste_after");
        assert!(registry.contains("P"), "Missing paste_before");
    }

    #[test]
    fn test_selection_commands_registered() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register_all(&mut registry);

        assert!(registry.contains("x"), "Missing select_line");
        assert!(registry.contains("X"), "Missing extend_line");
        assert!(registry.contains("%"), "Missing select_all");
        assert!(registry.contains(";"), "Missing collapse_selection");
    }
}
