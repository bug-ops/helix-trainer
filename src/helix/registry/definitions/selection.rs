//! Selection command definitions
//!
//! Registers selection commands (x, X, %, ;, Alt-s, &, _, Alt--, Alt-_, C, Alt-C, K, Alt-K, Ctrl-c, ,, Alt-,).
//! `s`/`S` (regex select/split) are dispatched separately - see the comment
//! above the registration block below.

use crate::helix::commands::{
    CMD_ALIGN_SELECTIONS, CMD_COLLAPSE_SELECTION, CMD_COPY_SELECTION_NEXT, CMD_COPY_SELECTION_PREV,
    CMD_EXTEND_LINE, CMD_KEEP_MATCHING, CMD_KEEP_PRIMARY_SELECTION, CMD_MERGE_CONSECUTIVE,
    CMD_MERGE_SELECTIONS, CMD_REMOVE_MATCHING, CMD_REMOVE_PRIMARY_SELECTION, CMD_SELECT_ALL,
    CMD_SELECT_LINE, CMD_SHRINK_TO_LINE_BOUNDS, CMD_SPLIT_SELECTION_NEWLINES, CMD_TOGGLE_COMMENTS,
    CMD_TRIM_SELECTIONS,
};
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{
    Binding, Category, CommandMetadata, CountBehavior, HelixName, Repeat,
};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::editing;
use crate::helix::simulator::commands::selection;

/// Register all selection commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // Existing selection commands
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("extend_line_below")),
            CMD_SELECT_LINE,
            Category::Selection,
        )
        .with_summary("Select line")
        .with_help("Select the current line. Use 'x' then 'd' to delete a line in Helix."),
        editing::select_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("extend_to_line_bounds")),
            CMD_EXTEND_LINE,
            Category::Selection,
        )
        .with_summary("Extend to line bounds")
        .with_help("Extend the selection to line boundaries."),
        editing::extend_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("select_all")),
            CMD_SELECT_ALL,
            Category::Selection,
        )
        .with_summary("Select all")
        .with_help("Select the entire document."),
        editing::select_all,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("collapse_selection")),
            CMD_COLLAPSE_SELECTION,
            Category::Selection,
        )
        .with_summary("Collapse selection")
        .with_help("Collapse the selection to a single cursor at the head position."),
        editing::collapse_selection,
    ));

    // Selection manipulation commands
    //
    // 's'/'S' (select_regex/split_selection) are NOT registered here: both
    // take a regex pattern argument, which `CommandHandler`'s fixed
    // `fn(&mut HelixSimulator<M>) -> Result<(), SimulatorError>` signature cannot
    // carry. They are dispatched directly from `execute_normal_mode_command_internal`
    // via the atomic `"s <pattern>"` / `"S <pattern>"` command encoding
    // assembled by `RegexPromptPending`, the same way register-scoped ops
    // and `:`-command-line invocations bypass the registry.

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("split_selection_on_newline")),
            CMD_SPLIT_SELECTION_NEWLINES,
            Category::Selection,
        )
        .with_summary("Split on newlines")
        .with_help("Split the selection on newlines, creating one selection per line."),
        selection::split_selection_newlines,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("align_selections")),
            CMD_ALIGN_SELECTIONS,
            Category::Selection,
        )
        .with_summary("Align selections")
        .with_help("Align all selections to the same column by inserting spaces."),
        selection::align_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("trim_selections")),
            CMD_TRIM_SELECTIONS,
            Category::Selection,
        )
        .with_summary("Trim selections")
        .with_help("Trim leading and trailing whitespace from selections."),
        selection::trim_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("merge_selections")),
            CMD_MERGE_SELECTIONS,
            Category::Selection,
        )
        .with_summary("Merge all selections")
        .with_help("Merge all selections into a single selection spanning from first to last."),
        selection::merge_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("merge_consecutive_selections")),
            CMD_MERGE_CONSECUTIVE,
            Category::Selection,
        )
        .with_summary("Merge consecutive")
        .with_help("Merge only adjacent/consecutive selections."),
        selection::merge_consecutive_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("copy_selection_on_next_line")),
            CMD_COPY_SELECTION_NEXT,
            Category::Selection,
        )
        .with_summary("Copy to next line")
        .with_help("Copy the selection to the next line below."),
        selection::copy_selection_next_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("copy_selection_on_prev_line")),
            CMD_COPY_SELECTION_PREV,
            Category::Selection,
        )
        .with_summary("Copy to prev line")
        .with_help("Copy the selection to the previous line above."),
        selection::copy_selection_prev_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("keep_selections")),
            CMD_KEEP_MATCHING,
            Category::Selection,
        )
        .with_summary("Keep matching")
        .with_help("Keep only selections that match the given regex pattern."),
        selection::keep_selections_matching,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("remove_selections")),
            CMD_REMOVE_MATCHING,
            Category::Selection,
        )
        .with_summary("Remove matching")
        .with_help("Remove selections that match the given regex pattern."),
        selection::remove_selections_matching,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("toggle_comments")),
            CMD_TOGGLE_COMMENTS,
            Category::Selection,
        )
        .with_summary("Toggle comments")
        .with_help("Toggle line comments on selected lines.")
        .with_repeat(Repeat::Repeatable)
        .with_count_behavior(CountBehavior::Ignore),
        selection::toggle_comments,
    ));

    // Primary selection management
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("keep_primary_selection")),
            CMD_KEEP_PRIMARY_SELECTION,
            Category::Selection,
        )
        .with_summary("Keep primary")
        .with_help("Keep only the primary selection, discarding all others."),
        selection::keep_primary_selection,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("remove_primary_selection")),
            CMD_REMOVE_PRIMARY_SELECTION,
            Category::Selection,
        )
        .with_summary("Remove primary")
        .with_help("Remove the primary selection, keeping other selections.")
        .with_count_behavior(CountBehavior::Ignore),
        selection::remove_primary_selection,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("shrink_to_line_bounds")),
            CMD_SHRINK_TO_LINE_BOUNDS,
            Category::Selection,
        )
        .with_summary("Shrink to line")
        .with_help("Shrink selection to line bounds for line-oriented editing."),
        selection::shrink_to_line_bounds,
    ));
}
