//! Movement command definitions
//!
//! Registers all movement commands (h, j, k, l, w, b, e, etc.)

use crate::grammar::commands::{
    CMD_FIND_CHAR, CMD_FIND_CHAR_REVERSE, CMD_FLIP_SELECTIONS, CMD_GOTO_FILE_START,
    CMD_GOTO_FIRST_NONBLANK, CMD_GOTO_FIRST_NONWHITESPACE, CMD_GOTO_LAST_LINE, CMD_GOTO_LINE_END,
    CMD_GOTO_LINE_START, CMD_GOTO_NEXT_PARAGRAPH, CMD_GOTO_PREV_PARAGRAPH, CMD_HALF_PAGE_DOWN,
    CMD_HALF_PAGE_UP, CMD_MATCH_BRACKETS, CMD_MOVE_DOWN, CMD_MOVE_LEFT,
    CMD_MOVE_LONG_WORD_BACKWARD, CMD_MOVE_LONG_WORD_END, CMD_MOVE_LONG_WORD_FORWARD,
    CMD_MOVE_RIGHT, CMD_MOVE_UP, CMD_MOVE_WORD_BACKWARD, CMD_MOVE_WORD_END, CMD_MOVE_WORD_FORWARD,
    CMD_PAGE_DOWN, CMD_PAGE_UP, CMD_REPEAT_LAST_MOTION, CMD_SELECT_MODE, CMD_TILL_CHAR,
    CMD_TILL_CHAR_REVERSE,
};
use crate::helix::registry::command_registry::{Command, CommandRegistry};
use crate::helix::registry::metadata::{
    Binding, Category, CommandMetadata, CountBehavior, HelixName,
};
use crate::helix::simulator::NormalMode;
use crate::helix::simulator::commands::{movement, select_mode, view};

/// Register all movement commands
pub fn register(registry: &mut CommandRegistry<NormalMode>) {
    // Basic cursor movement
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_char_left")),
            CMD_MOVE_LEFT,
            Category::Movement,
        )
        .with_summary("Move cursor left")
        .with_help("Move the cursor one character to the left. Stops at line start."),
        |sim| movement::move_left(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_char_right")),
            CMD_MOVE_RIGHT,
            Category::Movement,
        )
        .with_summary("Move cursor right")
        .with_help("Move the cursor one character to the right. Stops at line end."),
        |sim| movement::move_right(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_visual_line_down")),
            CMD_MOVE_DOWN,
            Category::Movement,
        )
        .with_summary("Move cursor down")
        .with_help("Move the cursor one line down. Maintains column position."),
        |sim| movement::move_down(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_visual_line_up")),
            CMD_MOVE_UP,
            Category::Movement,
        )
        .with_summary("Move cursor up")
        .with_help("Move the cursor one line up. Maintains column position."),
        |sim| movement::move_up(sim, 1),
    ));

    // Word movement
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_next_word_start")),
            CMD_MOVE_WORD_FORWARD,
            Category::Movement,
        )
        .with_summary("Move to next word start")
        .with_help("Move to the start of the next word. Words are delimited by punctuation."),
        |sim| movement::move_next_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_prev_word_start")),
            CMD_MOVE_WORD_BACKWARD,
            Category::Movement,
        )
        .with_summary("Move to previous word start")
        .with_help("Move to the start of the previous word."),
        |sim| movement::move_prev_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_next_word_end")),
            CMD_MOVE_WORD_END,
            Category::Movement,
        )
        .with_summary("Move to next word end")
        .with_help("Move to the end of the current or next word."),
        |sim| movement::move_next_word_end(sim, 1),
    ));

    // WORD movement (whitespace-delimited)
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_next_long_word_start")),
            CMD_MOVE_LONG_WORD_FORWARD,
            Category::Movement,
        )
        .with_summary("Move to next WORD start")
        .with_help("Move to the start of the next WORD (whitespace-delimited)."),
        |sim| movement::move_next_long_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_prev_long_word_start")),
            CMD_MOVE_LONG_WORD_BACKWARD,
            Category::Movement,
        )
        .with_summary("Move to previous WORD start")
        .with_help("Move to the start of the previous WORD (whitespace-delimited)."),
        |sim| movement::move_prev_long_word_start(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("move_next_long_word_end")),
            CMD_MOVE_LONG_WORD_END,
            Category::Movement,
        )
        .with_summary("Move to next WORD end")
        .with_help("Move to the end of the current or next WORD (whitespace-delimited)."),
        |sim| movement::move_next_long_word_end(sim, 1),
    ));

    // Line movement
    // Note: In Helix, '0' and '$' are NOT line movement commands
    // Use 'gh' (goto_line_start) and 'gl' (goto_line_end) instead
    // The goto commands are registered below with CMD_GOTO_LINE_START and CMD_GOTO_LINE_END

    // Document movement
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_file_start")),
            CMD_GOTO_FILE_START,
            Category::Movement,
        )
        .with_summary("Go to file start")
        .with_help("Move to the first line of the document.")
        .with_count_behavior(CountBehavior::Native),
        movement::move_document_start,
    ));

    // Goto mode commands
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_line_start")),
            CMD_GOTO_LINE_START,
            Category::Movement,
        )
        .with_summary("Go to line start")
        .with_help("Move to the first character of the current line (goto mode)."),
        movement::move_line_start,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_line_end")),
            CMD_GOTO_LINE_END,
            Category::Movement,
        )
        .with_summary("Go to line end")
        .with_help("Move to the last character of the current line (goto mode)."),
        movement::move_line_end,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_first_nonwhitespace")),
            CMD_GOTO_FIRST_NONWHITESPACE,
            Category::Movement,
        )
        .with_summary("Go to first non-whitespace")
        .with_help("Move to the first non-whitespace character of the current line."),
        movement::goto_first_nonwhitespace,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_last_line")),
            CMD_GOTO_LAST_LINE,
            Category::Movement,
        )
        .with_summary("Go to last line")
        .with_help("Move to the last line of the document (goto mode)."),
        movement::goto_last_line,
    ));

    // Match brackets
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("match_brackets")),
            CMD_MATCH_BRACKETS,
            Category::Movement,
        )
        .with_summary("Match brackets")
        .with_help("Jump to the matching bracket, brace, or parenthesis."),
        movement::match_brackets,
    ));

    // Flip selections
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("flip_selections")),
            CMD_FLIP_SELECTIONS,
            Category::Movement,
        )
        .with_summary("Flip selection direction")
        .with_help("Swap the anchor and cursor of the selection.")
        .with_count_behavior(CountBehavior::Ignore),
        movement::flip_selections,
    ));

    // Select mode
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("select_mode")),
            CMD_SELECT_MODE,
            Category::Movement,
        )
        .with_summary("Enter select mode")
        .with_help("Toggle select mode, where motions extend the selection.")
        .with_count_behavior(CountBehavior::Ignore),
        select_mode::toggle,
    ));
}

/// Register parametric movement commands (fx, Fx, tx, Tx)
///
/// These commands require a target character and are handled specially
/// by the dispatcher. This function registers metadata only.
pub(crate) fn register_parametric_metadata(registry: &mut CommandRegistry<NormalMode>) {
    // Find char commands - metadata only, handlers are in dispatcher
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("find_next_char")),
            CMD_FIND_CHAR,
            Category::Movement,
        )
        .with_summary("Find character forward")
        .with_help("Move to the next occurrence of a character on the current line."),
        |_sim| Ok(()), // Placeholder - actual execution in dispatcher
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("find_prev_char")),
            CMD_FIND_CHAR_REVERSE,
            Category::Movement,
        )
        .with_summary("Find character backward")
        .with_help("Move to the previous occurrence of a character on the current line."),
        |_sim| Ok(()),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("find_till_char")),
            CMD_TILL_CHAR,
            Category::Movement,
        )
        .with_summary("Till character forward")
        .with_help("Move just before the next occurrence of a character."),
        |_sim| Ok(()),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("till_prev_char")),
            CMD_TILL_CHAR_REVERSE,
            Category::Movement,
        )
        .with_summary("Till character backward")
        .with_help("Move just after the previous occurrence of a character."),
        |_sim| Ok(()),
    ));

    // Paragraph movement
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_prev_paragraph")),
            CMD_GOTO_PREV_PARAGRAPH,
            Category::Movement,
        )
        .with_summary("Previous paragraph")
        .with_help("Move to the start of the previous paragraph (blank line boundary)."),
        |sim| movement::goto_prev_paragraph(sim, 1),
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("goto_next_paragraph")),
            CMD_GOTO_NEXT_PARAGRAPH,
            Category::Movement,
        )
        .with_summary("Next paragraph")
        .with_help("Move to the start of the next paragraph (blank line boundary)."),
        |sim| movement::goto_next_paragraph(sim, 1),
    ));

    // First non-blank (trainer convenience alias for gs; "^" is not a real
    // Helix binding, so this is a `TrainerAlias` and excluded from the
    // helix_name -> CanonicalKeys reverse index).
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::TrainerAlias,
            CMD_GOTO_FIRST_NONBLANK,
            Category::Movement,
        )
        .with_summary("First non-blank")
        .with_help("Move to the first non-whitespace character on the current line."),
        movement::goto_first_nonwhitespace,
    ));

    // Repeat last f/F/t/T motion
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("repeat_last_motion")),
            CMD_REPEAT_LAST_MOTION,
            Category::Movement,
        )
        .with_summary("Repeat last motion")
        .with_help("Repeat the last f/F/t/T motion in the same direction."),
        movement::repeat_last_motion,
    ));

    // Page movement commands
    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("page_up")),
            CMD_PAGE_UP,
            Category::Movement,
        )
        .with_summary("Page up")
        .with_help("Move the cursor up by a full page."),
        view::page_up,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("page_down")),
            CMD_PAGE_DOWN,
            Category::Movement,
        )
        .with_summary("Page down")
        .with_help("Move the cursor down by a full page."),
        view::page_down,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("page_cursor_half_up")),
            CMD_HALF_PAGE_UP,
            Category::Movement,
        )
        .with_summary("Half page up")
        .with_help("Move the cursor up by half a page."),
        view::half_page_up,
    ));

    registry.register(Command::new(
        CommandMetadata::new(
            Binding::Helix(HelixName::new("page_cursor_half_down")),
            CMD_HALF_PAGE_DOWN,
            Category::Movement,
        )
        .with_summary("Half page down")
        .with_help("Move the cursor down by half a page."),
        view::half_page_down,
    ));
}
