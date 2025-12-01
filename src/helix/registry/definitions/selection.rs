//! Selection command definitions
//!
//! Registers selection commands (x, X, %, ;)

use crate::helix::commands::*;
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Category, CommandMetadata};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::editing;

/// Register all selection commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    registry.register(Command::new(
        CommandMetadata::new(
            "select_line",
            CMD_SELECT_LINE,
            "Select line",
            "Select the current line. Use 'x' then 'd' to delete a line in Helix.",
            Category::Selection,
            false,
            None,
        ),
        editing::select_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "extend_line",
            CMD_EXTEND_LINE,
            "Extend to line bounds",
            "Extend the selection to line boundaries.",
            Category::Selection,
            false,
            None,
        ),
        editing::extend_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "select_all",
            CMD_SELECT_ALL,
            "Select all",
            "Select the entire document.",
            Category::Selection,
            false,
            None,
        ),
        editing::select_all,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "collapse_selection",
            CMD_COLLAPSE_SELECTION,
            "Collapse selection",
            "Collapse the selection to a single cursor at the head position.",
            Category::Selection,
            false,
            None,
        ),
        editing::collapse_selection,
    ));
}
