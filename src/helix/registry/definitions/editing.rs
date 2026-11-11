//! Editing command definitions
//!
//! Registers all editing commands (d, c, J, >, <, ~, etc.)

use crate::helix::commands::{
    CMD_APPEND, CMD_APPEND_LINE_END, CMD_CHANGE, CMD_CHANGE_SELECTION_NOYANK, CMD_CTRL_R,
    CMD_DEDENT, CMD_DELETE_SELECTION, CMD_DELETE_SELECTION_NOYANK, CMD_ESCAPE, CMD_INDENT,
    CMD_INSERT, CMD_INSERT_LINE_START, CMD_JOIN_LINES, CMD_JOIN_SELECTIONS_SPACE, CMD_OPEN_ABOVE,
    CMD_OPEN_BELOW, CMD_REDO, CMD_REPEAT, CMD_REPLACE, CMD_REPLACE_WITH_YANKED, CMD_SWITCH_CASE,
    CMD_SWITCH_CASE_ALT, CMD_SWITCH_TO_UPPERCASE, CMD_UNDO,
};
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{
    Binding, Category, CommandMetadata, CountBehavior, HelixName, ModeTransition, Repeat,
};
use crate::helix::simulator::commands::editing;
use crate::helix::simulator::{HelixSimulator, NormalMode};

/// Register all editing commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // Delete selection
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("delete_selection")),
            CMD_DELETE_SELECTION,
            Category::Editing,
        )
        .with_summary("Delete selection")
        .with_help("Delete the current selection. In Helix, use 'x' + 'd' to delete a line.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::delete_selection,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("delete_selection_noyank")),
            CMD_DELETE_SELECTION_NOYANK,
            Category::Editing,
        )
        .with_summary("Delete selection without yanking")
        .with_help("Delete the current selection without writing to a register.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::delete_selection_noyank,
    ));

    // Change selection (delete and enter insert mode)
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("change_selection")),
            CMD_CHANGE,
            Category::ModeChange,
        )
        .with_summary("Change selection")
        .with_help("Delete the selection and enter insert mode.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Ignore),
        HelixSimulator::change_selection,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("change_selection_noyank")),
            CMD_CHANGE_SELECTION_NOYANK,
            Category::ModeChange,
        )
        .with_summary("Change selection without yanking")
        .with_help("Delete the selection and enter insert mode, without writing to a register.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Ignore),
        HelixSimulator::change_selection_noyank,
    ));

    // Join lines
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("join_selections")),
            CMD_JOIN_LINES,
            Category::Editing,
        )
        .with_summary("Join lines")
        .with_help("Join the current line with the next line.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::join_lines,
    ));

    // Indentation
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("indent")),
            CMD_INDENT,
            Category::Editing,
        )
        .with_summary("Indent line")
        .with_help("Indent the current line by one level.")
        .with_repeat(Repeat::Repeatable),
        editing::indent_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("unindent")),
            CMD_DEDENT,
            Category::Editing,
        )
        .with_summary("Dedent line")
        .with_help("Remove one level of indentation from the current line.")
        .with_repeat(Repeat::Repeatable),
        editing::dedent_line,
    ));

    // Case switching
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("switch_case")),
            CMD_SWITCH_CASE,
            Category::Editing,
        )
        .with_summary("Switch case")
        .with_help("Toggle the case of the selected text.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::switch_case,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("switch_to_lowercase")),
            CMD_SWITCH_CASE_ALT,
            Category::Editing,
        )
        .with_summary("Switch to lowercase")
        .with_help("Convert the selected text to lowercase.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::switch_to_lowercase,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("switch_to_uppercase")),
            CMD_SWITCH_TO_UPPERCASE,
            Category::Editing,
        )
        .with_summary("Switch to uppercase")
        .with_help("Convert the selected text to uppercase.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::switch_to_uppercase,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("replace_with_yanked")),
            CMD_REPLACE_WITH_YANKED,
            Category::Editing,
        )
        .with_summary("Replace with yanked")
        .with_help("Replace the current selection with yanked text.")
        .with_repeat(Repeat::Repeatable),
        editing::replace_with_yanked,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("join_selections_space")),
            CMD_JOIN_SELECTIONS_SPACE,
            Category::Editing,
        )
        .with_summary("Join with space")
        .with_help("Join lines in selection with spaces.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        editing::join_selections_space,
    ));

    // Replace character (metadata only, handler in dispatcher)
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("replace")),
            CMD_REPLACE,
            Category::Editing,
        )
        .with_summary("Replace character")
        .with_help("Replace the character under cursor with the next typed character.")
        .with_repeat(Repeat::Repeatable),
        |_sim| Ok(()), // Placeholder - actual execution in dispatcher
    ));

    // Insert mode entry commands
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("insert_mode")),
            CMD_INSERT,
            Category::ModeChange,
        )
        .with_summary("Enter insert mode")
        .with_help("Enter insert mode at the cursor position.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Ignore),
        |_sim| Ok(()), // Mode transition handled by caller
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("append_mode")),
            CMD_APPEND,
            Category::ModeChange,
        )
        .with_summary("Append after cursor")
        .with_help("Enter insert mode after the cursor position.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Ignore),
        HelixSimulator::append,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("insert_at_line_start")),
            CMD_INSERT_LINE_START,
            Category::ModeChange,
        )
        .with_summary("Insert at line start")
        .with_help("Enter insert mode at the start of the current line.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Ignore),
        HelixSimulator::insert_at_line_start,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("insert_at_line_end")),
            CMD_APPEND_LINE_END,
            Category::ModeChange,
        )
        .with_summary("Append at line end")
        .with_help("Enter insert mode at the end of the current line.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Ignore),
        HelixSimulator::append_at_line_end,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("open_below")),
            CMD_OPEN_BELOW,
            Category::ModeChange,
        )
        .with_summary("Open line below")
        .with_help("Insert a new line below and enter insert mode.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Native),
        HelixSimulator::open_below,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("open_above")),
            CMD_OPEN_ABOVE,
            Category::ModeChange,
        )
        .with_summary("Open line above")
        .with_help("Insert a new line above and enter insert mode.")
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert)
        .with_count_behavior(CountBehavior::Native),
        HelixSimulator::open_above,
    ));

    // Escape (no-op in normal mode; real Helix command is `normal_mode`)
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("normal_mode")),
            CMD_ESCAPE,
            Category::ModeChange,
        )
        .with_summary("Escape")
        .with_help("Return to normal mode (no-op if already in normal mode)."),
        |_sim| Ok(()), // No-op in normal mode
    ));

    // Repeat (metadata only, handled specially). Trainer-internal
    // convenience concept: Helix's dot-repeat is not a `MappableCommand` in
    // the upstream keymap, so this has no real Helix command name.
    registry.register(Command::new(
        CommandMetadata::new(Binding::TrainerAlias, CMD_REPEAT, Category::Editing)
            .with_summary("Repeat last action")
            .with_help("Repeat the last editing action.")
            .with_count_behavior(CountBehavior::Ignore),
        |_sim| Ok(()), // Handled specially by execute_command_any_mode
    ));

    // Undo/Redo
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("undo")),
            CMD_UNDO,
            Category::Undo,
        )
        .with_summary("Undo")
        .with_help("Undo the last change."),
        HelixSimulator::undo,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("redo")),
            CMD_REDO,
            Category::Undo,
        )
        .with_summary("Redo")
        .with_help("Redo the last undone change."),
        HelixSimulator::redo,
    ));

    // Trainer convenience binding: Ctrl-r is not normal-mode redo in Helix
    // (that's `U`/`A-U` -> redo/later), so this is a `TrainerAlias`.
    registry.register(Command::new(
        CommandMetadata::new(Binding::TrainerAlias, CMD_CTRL_R, Category::Undo)
            .with_summary("Redo (Ctrl-r)")
            .with_help("Redo the last undone change (alternative key)."),
        HelixSimulator::redo,
    ));
}
