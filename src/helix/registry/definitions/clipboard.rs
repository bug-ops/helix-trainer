//! Clipboard command definitions
//!
//! Registers clipboard commands (y, p, P)

use crate::helix::commands::{CMD_PASTE_AFTER, CMD_PASTE_BEFORE, CMD_YANK};
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{
    Binding, Category, CommandMetadata, CountBehavior, HelixName, Repeat,
};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::clipboard;

/// Register all clipboard commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("yank")),
            CMD_YANK,
            Category::Clipboard,
        )
        .with_summary("Yank (copy)")
        .with_help("Copy the current selection to the clipboard.")
        .with_count_behavior(CountBehavior::Ignore),
        clipboard::yank,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("paste_after")),
            CMD_PASTE_AFTER,
            Category::Clipboard,
        )
        .with_summary("Paste after cursor")
        .with_help("Paste clipboard contents after the cursor.")
        .with_repeat(Repeat::Repeatable),
        clipboard::paste_after,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("paste_before")),
            CMD_PASTE_BEFORE,
            Category::Clipboard,
        )
        .with_summary("Paste before cursor")
        .with_help("Paste clipboard contents before the cursor.")
        .with_repeat(Repeat::Repeatable),
        clipboard::paste_before,
    ));
}
