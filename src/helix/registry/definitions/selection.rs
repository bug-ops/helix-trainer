//! Selection command definitions
//!
//! Registers selection commands (x, X, %, ;, s, S, Alt-s, &, _, Alt--, Alt-_, C, Alt-C, K, Alt-K, Ctrl-c)

use crate::helix::commands::*;
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Category, CommandMetadata};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::editing;
use crate::helix::simulator::commands::selection;

/// Register all selection commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // Existing selection commands
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

    // Selection manipulation commands
    registry.register(Command::new(
        CommandMetadata::new(
            "select_regex",
            CMD_SELECT_REGEX,
            "Select regex",
            "Select all matches of a regex pattern within the current selection.",
            Category::Selection,
            false,
            None,
        ),
        selection::select_regex,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "split_selection",
            CMD_SPLIT_SELECTION,
            "Split selection",
            "Split the selection on a regex pattern delimiter.",
            Category::Selection,
            false,
            None,
        ),
        selection::split_selection,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "split_selection_newlines",
            CMD_SPLIT_SELECTION_NEWLINES,
            "Split on newlines",
            "Split the selection on newlines, creating one selection per line.",
            Category::Selection,
            false,
            None,
        ),
        selection::split_selection_newlines,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "align_selections",
            CMD_ALIGN_SELECTIONS,
            "Align selections",
            "Align all selections to the same column by inserting spaces.",
            Category::Selection,
            false,
            None,
        ),
        selection::align_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "trim_selections",
            CMD_TRIM_SELECTIONS,
            "Trim selections",
            "Trim leading and trailing whitespace from selections.",
            Category::Selection,
            false,
            None,
        ),
        selection::trim_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "merge_selections",
            CMD_MERGE_SELECTIONS,
            "Merge all selections",
            "Merge all selections into a single selection spanning from first to last.",
            Category::Selection,
            false,
            None,
        ),
        selection::merge_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "merge_consecutive",
            CMD_MERGE_CONSECUTIVE,
            "Merge consecutive",
            "Merge only adjacent/consecutive selections.",
            Category::Selection,
            false,
            None,
        ),
        selection::merge_consecutive_selections,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "copy_selection_next_line",
            CMD_COPY_SELECTION_NEXT,
            "Copy to next line",
            "Copy the selection to the next line below.",
            Category::Selection,
            false,
            None,
        ),
        selection::copy_selection_next_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "copy_selection_prev_line",
            CMD_COPY_SELECTION_PREV,
            "Copy to prev line",
            "Copy the selection to the previous line above.",
            Category::Selection,
            false,
            None,
        ),
        selection::copy_selection_prev_line,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "keep_selections_matching",
            CMD_KEEP_MATCHING,
            "Keep matching",
            "Keep only selections that match the given regex pattern.",
            Category::Selection,
            false,
            None,
        ),
        selection::keep_selections_matching,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "remove_selections_matching",
            CMD_REMOVE_MATCHING,
            "Remove matching",
            "Remove selections that match the given regex pattern.",
            Category::Selection,
            false,
            None,
        ),
        selection::remove_selections_matching,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "toggle_comments",
            CMD_TOGGLE_COMMENTS,
            "Toggle comments",
            "Toggle line comments on selected lines.",
            Category::Selection,
            true, // repeatable
            None,
        ),
        selection::toggle_comments,
    ));
}
