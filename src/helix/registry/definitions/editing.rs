//! Editing command definitions
//!
//! Registers all editing commands (d, c, J, >, <, ~, etc.)

use crate::helix::commands::*;
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Category, CommandMetadata, ModeTransition};
use crate::helix::simulator::commands::editing;
use crate::helix::simulator::{HelixSimulator, NormalMode};

/// Register all editing commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // Delete selection
    registry.register(Command::new(
        CommandMetadata::new(
            "delete_selection",
            CMD_DELETE_SELECTION,
            "Delete selection",
            "Delete the current selection. In Helix, use 'x' + 'd' to delete a line.",
            Category::Editing,
            true,
            None,
        ),
        editing::delete_selection,
    ));

    // Change selection (delete and enter insert mode)
    registry.register(Command::new(
        CommandMetadata::new(
            "change_selection",
            CMD_CHANGE,
            "Change selection",
            "Delete the selection and enter insert mode.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        HelixSimulator::change_selection,
    ));

    // Join lines
    registry.register(Command::new(
        CommandMetadata::new(
            "join_selections",
            CMD_JOIN_LINES,
            "Join lines",
            "Join the current line with the next line.",
            Category::Editing,
            true,
            None,
        ),
        editing::join_lines,
    ));

    // Indentation
    registry.register(Command::new(
        CommandMetadata::new(
            "indent",
            CMD_INDENT,
            "Indent line",
            "Indent the current line by one level.",
            Category::Editing,
            true,
            None,
        ),
        editing::indent_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "unindent",
            CMD_DEDENT,
            "Dedent line",
            "Remove one level of indentation from the current line.",
            Category::Editing,
            true,
            None,
        ),
        editing::dedent_line,
    ));

    // Case switching
    registry.register(Command::new(
        CommandMetadata::new(
            "switch_case",
            CMD_SWITCH_CASE,
            "Switch case",
            "Toggle the case of the selected text.",
            Category::Editing,
            true,
            None,
        ),
        editing::switch_case,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "switch_to_lowercase",
            CMD_SWITCH_CASE_ALT,
            "Switch case (alt)",
            "Toggle the case of the selected text (alternative key).",
            Category::Editing,
            true,
            None,
        ),
        editing::switch_case,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "switch_to_uppercase",
            CMD_SWITCH_TO_UPPERCASE,
            "Switch to uppercase",
            "Convert the selected text to uppercase.",
            Category::Editing,
            true,
            None,
        ),
        editing::switch_to_uppercase,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "replace_with_yanked",
            CMD_REPLACE_WITH_YANKED,
            "Replace with yanked",
            "Replace the current selection with yanked text.",
            Category::Editing,
            true,
            None,
        ),
        editing::replace_with_yanked,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "join_selections_space",
            CMD_JOIN_SELECTIONS_SPACE,
            "Join with space",
            "Join lines in selection with spaces.",
            Category::Editing,
            true,
            None,
        ),
        editing::join_selections_space,
    ));

    // Replace character (metadata only, handler in dispatcher)
    registry.register(Command::new(
        CommandMetadata::new(
            "replace",
            CMD_REPLACE,
            "Replace character",
            "Replace the character under cursor with the next typed character.",
            Category::Editing,
            true,
            None,
        ),
        |_sim| Ok(()), // Placeholder - actual execution in dispatcher
    ));

    // Insert mode entry commands
    registry.register(Command::new(
        CommandMetadata::new(
            "insert_mode",
            CMD_INSERT,
            "Enter insert mode",
            "Enter insert mode at the cursor position.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        |_sim| Ok(()), // Mode transition handled by caller
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "append_mode",
            CMD_APPEND,
            "Append after cursor",
            "Enter insert mode after the cursor position.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        HelixSimulator::append,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "insert_at_line_start",
            CMD_INSERT_LINE_START,
            "Insert at line start",
            "Enter insert mode at the start of the current line.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        HelixSimulator::insert_at_line_start,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "insert_at_line_end",
            CMD_APPEND_LINE_END,
            "Append at line end",
            "Enter insert mode at the end of the current line.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        HelixSimulator::append_at_line_end,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "open_below",
            CMD_OPEN_BELOW,
            "Open line below",
            "Insert a new line below and enter insert mode.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        HelixSimulator::open_below,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "open_above",
            CMD_OPEN_ABOVE,
            "Open line above",
            "Insert a new line above and enter insert mode.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        ),
        HelixSimulator::open_above,
    ));

    // Escape (no-op in normal mode; real Helix command is `normal_mode`)
    registry.register(Command::new(
        CommandMetadata::new(
            "normal_mode",
            CMD_ESCAPE,
            "Escape",
            "Return to normal mode (no-op if already in normal mode).",
            Category::ModeChange,
            false,
            None,
        ),
        |_sim| Ok(()), // No-op in normal mode
    ));

    // Repeat (metadata only, handled specially). Trainer-internal
    // convenience concept: Helix's dot-repeat is not a `MappableCommand` in
    // the upstream keymap, so this has no real Helix command name.
    registry.register(Command::new(
        CommandMetadata::new_alias(
            "repeat",
            CMD_REPEAT,
            "Repeat last action",
            "Repeat the last editing action.",
            Category::Editing,
            false,
            None,
        ),
        |_sim| Ok(()), // Handled specially by execute_command_any_mode
    ));

    // Undo/Redo
    registry.register(Command::new(
        CommandMetadata::new(
            "undo",
            CMD_UNDO,
            "Undo",
            "Undo the last change.",
            Category::Undo,
            false,
            None,
        ),
        HelixSimulator::undo,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "redo",
            CMD_REDO,
            "Redo",
            "Redo the last undone change.",
            Category::Undo,
            false,
            None,
        ),
        HelixSimulator::redo,
    ));

    // Trainer convenience binding: Ctrl-r is not normal-mode redo in Helix
    // (that's `U`/`A-U` -> redo/later), so this is alias_only.
    registry.register(Command::new(
        CommandMetadata::new_alias(
            "redo_alt",
            CMD_CTRL_R,
            "Redo (Ctrl-r)",
            "Redo the last undone change (alternative key).",
            Category::Undo,
            false,
            None,
        ),
        HelixSimulator::redo,
    ));
}
