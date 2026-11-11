//! Search command definitions
//!
//! Registers search commands (/, ?, n, N, *, Alt-*)

use crate::grammar::commands::{
    CMD_SEARCH_BACKWARD, CMD_SEARCH_FORWARD, CMD_SEARCH_NEXT, CMD_SEARCH_PREV,
    CMD_SEARCH_SELECTION, CMD_SEARCH_WORD,
};
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Binding, Category, CommandMetadata, HelixName};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::search;

/// Register all search commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("search")),
            CMD_SEARCH_FORWARD,
            Category::Search,
        )
        .with_summary("Search forward")
        .with_help(
            "Search forward for a regex pattern. Type the pattern and press Enter to find matches.",
        ),
        search::search_forward,
    ));

    registry.register(Command::new(
        CommandMetadata::new(Binding::Helix(HelixName::new("rsearch")), CMD_SEARCH_BACKWARD, Category::Search)
.with_summary("Search backward")
.with_help("Search backward for a regex pattern. Type the pattern and press Enter to find matches."),
        search::search_backward,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("search_next")),
            CMD_SEARCH_NEXT,
            Category::Search,
        )
        .with_summary("Next match")
        .with_help("Jump to the next search match. Wraps around at the end of the document."),
        search::goto_next_match,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("search_prev")),
            CMD_SEARCH_PREV,
            Category::Search,
        )
        .with_summary("Previous match")
        .with_help("Jump to the previous search match. Wraps around at the start of the document."),
        search::goto_prev_match,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("search_selection_detect_word_boundaries")),
            CMD_SEARCH_WORD,
            Category::Search,
        )
        .with_summary("Search selection as a word")
        .with_help(
            "Search for the selection text (escaped), adding word boundaries where the selection starts or ends at a word edge; the selection is unchanged.",
        ),
        search::search_word_under_cursor,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("search_selection")),
            CMD_SEARCH_SELECTION,
            Category::Search,
        )
        .with_summary("Search selection")
        .with_help("Search for the current selection text without word boundaries."),
        search::search_selection,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_commands_registered() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register(&mut registry);

        assert!(
            registry.contains(CMD_SEARCH_FORWARD),
            "Missing search_forward"
        );
        assert!(
            registry.contains(CMD_SEARCH_BACKWARD),
            "Missing search_backward"
        );
        assert!(registry.contains(CMD_SEARCH_NEXT), "Missing search_next");
        assert!(registry.contains(CMD_SEARCH_PREV), "Missing search_prev");
        assert!(registry.contains(CMD_SEARCH_WORD), "Missing search_word");
        assert!(
            registry.contains(CMD_SEARCH_SELECTION),
            "Missing search_selection"
        );
    }

    #[test]
    fn test_search_commands_category() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register(&mut registry);

        let search_cmds = registry.commands_in_category(Category::Search);
        assert!(
            search_cmds.len() >= 6,
            "Expected at least 6 search commands"
        );
    }
}
