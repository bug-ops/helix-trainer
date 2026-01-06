//! Search command definitions
//!
//! Registers search commands (/, ?, n, N, *, #, Alt-*)

use crate::helix::commands::*;
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Category, CommandMetadata};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::search;

/// Register all search commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    registry.register(Command::new(
        CommandMetadata::new(
            "search_forward",
            CMD_SEARCH_FORWARD,
            "Search forward",
            "Search forward for a regex pattern. Type the pattern and press Enter to find matches.",
            Category::Search,
            false,
            None,
        ),
        search::search_forward,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "search_backward",
            CMD_SEARCH_BACKWARD,
            "Search backward",
            "Search backward for a regex pattern. Type the pattern and press Enter to find matches.",
            Category::Search,
            false,
            None,
        ),
        search::search_backward,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "search_next",
            CMD_SEARCH_NEXT,
            "Next match",
            "Jump to the next search match. Wraps around at the end of the document.",
            Category::Search,
            false,
            None,
        ),
        search::goto_next_match,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "search_prev",
            CMD_SEARCH_PREV,
            "Previous match",
            "Jump to the previous search match. Wraps around at the start of the document.",
            Category::Search,
            false,
            None,
        ),
        search::goto_prev_match,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "search_word",
            CMD_SEARCH_WORD,
            "Search word",
            "Search for the word under cursor with word boundaries.",
            Category::Search,
            false,
            None,
        ),
        search::search_word_under_cursor,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "search_selection",
            CMD_SEARCH_SELECTION,
            "Search selection",
            "Search for the current selection text without word boundaries.",
            Category::Search,
            false,
            None,
        ),
        search::search_selection,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "search_word_backward",
            CMD_SEARCH_WORD_BACKWARD,
            "Search word backward",
            "Search backward for the word under cursor with word boundaries.",
            Category::Search,
            false,
            None,
        ),
        search::search_word_under_cursor_backward,
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
        assert!(
            registry.contains(CMD_SEARCH_WORD_BACKWARD),
            "Missing search_word_backward"
        );
    }

    #[test]
    fn test_search_commands_category() {
        let mut registry = CommandRegistry::<NormalMode>::new();
        register(&mut registry);

        let search_cmds = registry.commands_in_category(Category::Search);
        assert!(
            search_cmds.len() >= 7,
            "Expected at least 7 search commands"
        );
    }
}
