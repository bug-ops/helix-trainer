//! Movement command definitions
//!
//! Registers all movement commands (h, j, k, l, w, b, e, etc.)

use crate::helix::commands::*;
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{Category, CommandMetadata};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::movement;

/// Register all movement commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // Basic cursor movement
    registry.register(Command::new(
        CommandMetadata::new(
            "move_char_left",
            CMD_MOVE_LEFT,
            "Move cursor left",
            "Move the cursor one character to the left. Stops at line start.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_left(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_char_right",
            CMD_MOVE_RIGHT,
            "Move cursor right",
            "Move the cursor one character to the right. Stops at line end.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_right(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_visual_line_down",
            CMD_MOVE_DOWN,
            "Move cursor down",
            "Move the cursor one line down. Maintains column position.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_down(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_visual_line_up",
            CMD_MOVE_UP,
            "Move cursor up",
            "Move the cursor one line up. Maintains column position.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_up(sim, 1),
    ));

    // Word movement
    registry.register(Command::new(
        CommandMetadata::new(
            "move_next_word_start",
            CMD_MOVE_WORD_FORWARD,
            "Move to next word start",
            "Move to the start of the next word. Words are delimited by punctuation.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_next_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_prev_word_start",
            CMD_MOVE_WORD_BACKWARD,
            "Move to previous word start",
            "Move to the start of the previous word.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_prev_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_next_word_end",
            CMD_MOVE_WORD_END,
            "Move to next word end",
            "Move to the end of the current or next word.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_next_word_end(sim, 1),
    ));

    // WORD movement (whitespace-delimited)
    registry.register(Command::new(
        CommandMetadata::new(
            "move_next_long_word_start",
            CMD_MOVE_LONG_WORD_FORWARD,
            "Move to next WORD start",
            "Move to the start of the next WORD (whitespace-delimited).",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_next_long_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_prev_long_word_start",
            CMD_MOVE_LONG_WORD_BACKWARD,
            "Move to previous WORD start",
            "Move to the start of the previous WORD (whitespace-delimited).",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_prev_long_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "move_next_long_word_end",
            CMD_MOVE_LONG_WORD_END,
            "Move to next WORD end",
            "Move to the end of the current or next WORD (whitespace-delimited).",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::move_next_long_word_end(sim, 1),
    ));

    // Line movement
    // Note: In Helix, '0' and '$' are NOT line movement commands
    // Use 'gh' (goto_line_start) and 'gl' (goto_line_end) instead
    // The goto commands are registered below with CMD_GOTO_LINE_START and CMD_GOTO_LINE_END

    // Document movement
    registry.register(Command::new(
        CommandMetadata::new(
            "goto_file_start",
            CMD_GOTO_FILE_START,
            "Go to file start",
            "Move to the first line of the document.",
            Category::Movement,
            false,
            None,
        ),
        movement::move_document_start,
    ));

    // Goto mode commands
    registry.register(Command::new(
        CommandMetadata::new(
            "goto_line_start",
            CMD_GOTO_LINE_START,
            "Go to line start",
            "Move to the first character of the current line (goto mode).",
            Category::Movement,
            false,
            None,
        ),
        movement::move_line_start,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "goto_line_end",
            CMD_GOTO_LINE_END,
            "Go to line end",
            "Move to the last character of the current line (goto mode).",
            Category::Movement,
            false,
            None,
        ),
        movement::move_line_end,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "goto_first_nonwhitespace",
            CMD_GOTO_FIRST_NONWHITESPACE,
            "Go to first non-whitespace",
            "Move to the first non-whitespace character of the current line.",
            Category::Movement,
            false,
            None,
        ),
        movement::goto_first_nonwhitespace,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "goto_last_line",
            CMD_GOTO_LAST_LINE,
            "Go to last line",
            "Move to the last line of the document (goto mode).",
            Category::Movement,
            false,
            None,
        ),
        movement::goto_last_line,
    ));

    // Match brackets
    registry.register(Command::new(
        CommandMetadata::new(
            "match_brackets",
            CMD_MATCH_BRACKETS,
            "Match brackets",
            "Jump to the matching bracket, brace, or parenthesis.",
            Category::Movement,
            false,
            None,
        ),
        movement::match_brackets,
    ));

    // Flip selections
    registry.register(Command::new(
        CommandMetadata::new(
            "flip_selections",
            CMD_FLIP_SELECTIONS,
            "Flip selection direction",
            "Swap the anchor and cursor of the selection.",
            Category::Movement,
            false,
            None,
        ),
        movement::flip_selections,
    ));

    // Select mode (no-op, visual indicator)
    registry.register(Command::new(
        CommandMetadata::new(
            "select_mode",
            CMD_SELECT_MODE,
            "Enter select mode",
            "Toggle selection extension mode (visual enhancement).",
            Category::Movement,
            false,
            None,
        ),
        |_sim| Ok(()), // No-op
    ));
}

/// Register parametric movement commands (fx, Fx, tx, Tx)
///
/// These commands require a target character and are handled specially
/// by the dispatcher. This function registers metadata only.
pub fn register_parametric_metadata(registry: &mut CommandRegistry<NormalMode>) {
    // Find char commands - metadata only, handlers are in dispatcher
    registry.register(Command::new(
        CommandMetadata::new(
            "find_next_char",
            CMD_FIND_CHAR,
            "Find character forward",
            "Move to the next occurrence of a character on the current line.",
            Category::Movement,
            false,
            None,
        ),
        |_sim| Ok(()), // Placeholder - actual execution in dispatcher
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "find_prev_char",
            CMD_FIND_CHAR_REVERSE,
            "Find character backward",
            "Move to the previous occurrence of a character on the current line.",
            Category::Movement,
            false,
            None,
        ),
        |_sim| Ok(()),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "find_till_char",
            CMD_TILL_CHAR,
            "Till character forward",
            "Move just before the next occurrence of a character.",
            Category::Movement,
            false,
            None,
        ),
        |_sim| Ok(()),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "till_prev_char",
            CMD_TILL_CHAR_REVERSE,
            "Till character backward",
            "Move just after the previous occurrence of a character.",
            Category::Movement,
            false,
            None,
        ),
        |_sim| Ok(()),
    ));

    // Paragraph movement
    registry.register(Command::new(
        CommandMetadata::new(
            "goto_prev_paragraph",
            CMD_GOTO_PREV_PARAGRAPH,
            "Previous paragraph",
            "Move to the start of the previous paragraph (blank line boundary).",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::goto_prev_paragraph(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "goto_next_paragraph",
            CMD_GOTO_NEXT_PARAGRAPH,
            "Next paragraph",
            "Move to the start of the next paragraph (blank line boundary).",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::goto_next_paragraph(sim, 1),
    ));

    // First non-blank (trainer convenience alias for gs; "^" is not a real
    // Helix binding, so this is marked alias_only and excluded from the
    // helix_name -> CanonicalKeys reverse index).
    registry.register(Command::new(
        CommandMetadata::new_alias(
            "goto_first_nonblank",
            CMD_GOTO_FIRST_NONBLANK,
            "First non-blank",
            "Move to the first non-whitespace character on the current line.",
            Category::Movement,
            false,
            None,
        ),
        movement::goto_first_nonwhitespace,
    ));

    // Repeat last f/F/t/T motion
    registry.register(Command::new(
        CommandMetadata::new(
            "repeat_last_motion",
            CMD_REPEAT_LAST_MOTION,
            "Repeat last motion",
            "Repeat the last f/F/t/T motion in the same direction.",
            Category::Movement,
            false,
            None,
        ),
        movement::repeat_last_motion,
    ));

    // Page movement commands
    registry.register(Command::new(
        CommandMetadata::new(
            "page_up",
            CMD_PAGE_UP,
            "Page up",
            "Move the cursor up by a full page.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::page_up(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "page_down",
            CMD_PAGE_DOWN,
            "Page down",
            "Move the cursor down by a full page.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::page_down(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "page_cursor_half_up",
            CMD_HALF_PAGE_UP,
            "Half page up",
            "Move the cursor up by half a page.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::half_page_up(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            "page_cursor_half_down",
            CMD_HALF_PAGE_DOWN,
            "Half page down",
            "Move the cursor down by half a page.",
            Category::Movement,
            false,
            None,
        ),
        |sim| movement::half_page_down(sim, 1),
    ));
}
