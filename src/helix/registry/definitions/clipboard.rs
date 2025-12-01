//! Clipboard command definitions
//!
//! Registers clipboard commands (y, p, P)

use crate::helix::commands::*;
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Category, CommandMetadata};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::clipboard;

/// Register all clipboard commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    registry.register(Command::new(
        CommandMetadata::new(
            "yank",
            CMD_YANK,
            "Yank (copy)",
            "Copy the current selection to the clipboard.",
            Category::Clipboard,
            false,
            None,
        ),
        clipboard::yank,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "paste_after",
            CMD_PASTE_AFTER,
            "Paste after cursor",
            "Paste clipboard contents after the cursor.",
            Category::Clipboard,
            true,
            None,
        ),
        clipboard::paste_after,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "paste_before",
            CMD_PASTE_BEFORE,
            "Paste before cursor",
            "Paste clipboard contents before the cursor.",
            Category::Clipboard,
            true,
            None,
        ),
        clipboard::paste_before,
    ));
}
