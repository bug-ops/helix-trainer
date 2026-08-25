//! Shared command execution context for different gameplay modes
//!
//! This module provides a trait-based abstraction for command execution
//! that unifies logic between training mode and arcade mode, reducing
//! code duplication while maintaining type safety.
//!
//! # Key Traits
//!
//! - [`CommandContext`]: Read-only access to command buffer and mode
//! - [`CommandExecutor`]: Execute commands with count prefix support

use crate::helix::commands::*;
use crate::security::UserError;

/// Format a key command for display in key history
///
/// Converts internal command names to user-friendly display strings.
/// This is used by both training and arcade modes for consistent
/// key history display.
///
/// # Examples
///
/// ```
/// use helix_trainer::game::format_key_for_display;
///
/// assert_eq!(format_key_for_display("Left"), "←");
/// assert_eq!(format_key_for_display("Backspace"), "⌫");
/// assert_eq!(format_key_for_display("x"), "x");
/// ```
pub fn format_key_for_display(command: &str) -> String {
    match command {
        CMD_ARROW_LEFT => "←".to_string(),
        CMD_ARROW_RIGHT => "→".to_string(),
        CMD_ARROW_UP => "↑".to_string(),
        CMD_ARROW_DOWN => "↓".to_string(),
        CMD_BACKSPACE => "⌫".to_string(),
        CMD_ESCAPE => "Esc".to_string(),
        "\n" => "↵".to_string(),
        " " => "Space".to_string(),
        cmd if cmd.len() == 1 => cmd.to_string(),
        cmd => cmd.to_string(),
    }
}

/// Read-only access to command execution context
///
/// Provides information needed for command parsing and buffering,
/// without requiring execution capabilities. This separation allows
/// handlers to maintain control over actual command execution and
/// state transitions.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::CommandContext;
///
/// fn check_mode<C: CommandContext>(ctx: &C) -> &'static str {
///     if ctx.is_insert_mode() {
///         "INSERT"
///     } else {
///         "NORMAL"
///     }
/// }
/// ```
pub trait CommandContext {
    /// Get reference to command buffer for multi-key commands
    fn command_buffer(&self) -> &str;

    /// Check if currently in insert mode
    fn is_insert_mode(&self) -> bool;

    /// Get last command executed (for repeat functionality)
    fn last_command(&self) -> Option<&str>;
}

/// Result of parsing command buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    /// Complete command ready to execute
    Complete(String),
    /// Waiting for more input (partial multi-key command)
    Partial,
    /// Invalid sequence, should clear buffer
    Invalid,
}

impl ParsedCommand {
    /// Check if this is a complete command
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(_))
    }

    /// Check if waiting for more input
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial)
    }

    /// Check if invalid sequence
    pub fn is_invalid(&self) -> bool {
        matches!(self, Self::Invalid)
    }

    /// Get the command string if complete
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Complete(cmd) => Some(cmd),
            _ => None,
        }
    }
}

/// Parse command buffer to determine if a complete command is available
///
/// Returns a ParsedCommand indicating whether the buffer contains a complete
/// command, is waiting for more input, or contains an invalid sequence.
///
/// This function delegates to the KeyTrie in the command registry for
/// consistent multi-key sequence handling.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::command_context::{parse_command_buffer, ParsedCommand};
/// use std::assert_matches;
///
/// assert_matches!(parse_command_buffer("gg"), ParsedCommand::Complete(_));
/// assert_matches!(parse_command_buffer("g"), ParsedCommand::Partial);
/// assert_matches!(parse_command_buffer("xyz"), ParsedCommand::Invalid);
/// ```
pub fn parse_command_buffer(buffer: &str) -> ParsedCommand {
    use crate::helix::registry::{KeyMatch, normal_registry};

    // Delegate to KeyTrie for resolution
    let trie = normal_registry().key_trie();
    match trie.resolve(buffer) {
        KeyMatch::Complete(cmd) => ParsedCommand::Complete(cmd),
        KeyMatch::Partial => ParsedCommand::Partial,
        KeyMatch::Invalid => ParsedCommand::Invalid,
    }
}

/// Extract count prefix and command from a command string
///
/// Returns (count, command) where count defaults to 1 if no prefix.
///
/// # Examples
/// - "3h" -> (3, "h")
/// - "12j" -> (12, "j")
/// - "h" -> (1, "h")
/// - "gg" -> (1, "gg")
pub fn extract_count_and_command(cmd: &str) -> (usize, &str) {
    use crate::helix::registry::split_count_prefix;

    match split_count_prefix(cmd) {
        (Some(count), rest) if !rest.is_empty() => (count, rest),
        _ => (1, cmd),
    }
}

/// Trait for executing commands with count prefix support
///
/// Provides unified command execution logic for both training and arcade modes.
/// Implementations handle the actual command execution through their simulator,
/// while this trait provides the common count prefix handling.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::CommandExecutor;
///
/// fn execute_with_count<E: CommandExecutor>(executor: &mut E, cmd: &str) -> Result<(), UserError> {
///     // "3d" executes "d" three times but counts as one action
///     executor.execute_with_count(cmd)
/// }
/// ```
pub trait CommandExecutor {
    /// Execute a single base command (without count prefix)
    ///
    /// This is the low-level execution method that implementations must provide.
    /// It should execute exactly one command through the simulator.
    fn execute_single(&mut self, command: &str) -> Result<(), UserError>;

    /// Execute a command with count prefix support
    ///
    /// Parses the command to extract count prefix (e.g., "3d" -> count=3, cmd="d"),
    /// then executes the base command `count` times. This counts as ONE action
    /// for scoring purposes.
    ///
    /// # Arguments
    /// * `command` - Full command string, possibly with count prefix
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(UserError)` if any execution fails
    fn execute_with_count(&mut self, command: &str) -> Result<(), UserError> {
        let (count, base_cmd) = extract_count_and_command(command);

        for _ in 0..count {
            self.execute_single(base_cmd)?;
        }

        Ok(())
    }

    /// Check if scenario is completed after command execution
    fn check_completion(&self) -> bool;
}

/// Result of processing command input through the buffer
///
/// Returned by `process_command_input` to indicate what action
/// the caller should take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandInputResult {
    /// Execute command directly (insert mode or complete command)
    Execute(String),
    /// Invalid sequence - buffer was cleared, no action needed
    Invalid,
    /// Waiting for more input (partial multi-key command)
    Partial,
}

impl CommandInputResult {
    /// Check if this result requires command execution
    pub fn should_execute(&self) -> bool {
        matches!(self, Self::Execute(_))
    }

    /// Get the command to execute, if any
    pub fn command(&self) -> Option<&str> {
        match self {
            Self::Execute(cmd) => Some(cmd),
            _ => None,
        }
    }
}

/// Trait for types that provide command buffer access
///
/// Used by `process_command_input` to manage the command buffer
/// for multi-key command sequences.
pub trait CommandBuffer {
    /// Get reference to the command buffer
    fn buffer(&self) -> &str;

    /// Push input to the command buffer
    fn push(&mut self, input: &str);

    /// Clear the command buffer
    fn clear(&mut self);
}

/// Process command input through the buffer with unified logic
///
/// This function handles the common pattern of:
/// 1. In insert mode: execute directly
/// 2. In normal mode: buffer input, parse, and determine action
///
/// # Arguments
/// * `buffer` - Command buffer for multi-key sequences
/// * `input` - The input key/command to process
/// * `is_insert_mode` - Whether currently in insert mode
///
/// # Returns
/// * `CommandInputResult::Execute(cmd)` - Execute the command
/// * `CommandInputResult::Invalid` - Invalid sequence, buffer cleared
/// * `CommandInputResult::Partial` - Waiting for more input
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::game::{CommandBuffer, CommandInputResult, process_command_input};
///
/// let mut buffer = MyBuffer::new();
/// match process_command_input(&mut buffer, "g", false) {
///     CommandInputResult::Partial => { /* waiting for more */ }
///     CommandInputResult::Execute(cmd) => { /* execute cmd */ }
///     CommandInputResult::Invalid => { /* invalid sequence */ }
/// }
/// ```
pub fn process_command_input<B: CommandBuffer>(
    buffer: &mut B,
    input: &str,
    is_insert_mode: bool,
) -> CommandInputResult {
    // In insert mode, execute directly without buffering
    if is_insert_mode {
        return CommandInputResult::Execute(input.to_string());
    }

    // Normal mode: buffer and parse
    buffer.push(input);

    match parse_command_buffer(buffer.buffer()) {
        ParsedCommand::Invalid => {
            buffer.clear();
            CommandInputResult::Invalid
        }
        ParsedCommand::Complete(cmd) => {
            buffer.clear();
            CommandInputResult::Execute(cmd)
        }
        ParsedCommand::Partial => CommandInputResult::Partial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn test_parse_command_buffer_single_key() {
        assert_matches!(parse_command_buffer("j"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("k"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("d"), ParsedCommand::Complete(_));

        assert_eq!(parse_command_buffer("j").command(), Some("j"));
    }

    #[test]
    fn test_parse_command_buffer_goto_commands() {
        assert_matches!(parse_command_buffer("gg"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("gh"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("gl"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("gs"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("ge"), ParsedCommand::Complete(_));

        assert_eq!(parse_command_buffer("gg").command(), Some("gg"));
    }

    #[test]
    fn test_parse_command_buffer_partial() {
        assert_matches!(parse_command_buffer("g"), ParsedCommand::Partial);
        assert_matches!(parse_command_buffer("r"), ParsedCommand::Partial);
        assert_matches!(parse_command_buffer("f"), ParsedCommand::Partial);
    }

    #[test]
    fn test_parse_command_buffer_replace() {
        assert_matches!(parse_command_buffer("ra"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("rx"), ParsedCommand::Complete(_));

        assert_eq!(parse_command_buffer("ra").command(), Some("ra"));
    }

    #[test]
    fn test_parse_command_buffer_find() {
        assert_matches!(parse_command_buffer("fa"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("Fx"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("te"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("TY"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_invalid() {
        assert_matches!(parse_command_buffer("xyz"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("ggg"), ParsedCommand::Invalid);
    }

    #[test]
    fn test_parsed_command_helpers() {
        let complete = ParsedCommand::Complete("j".to_string());
        assert!(complete.is_complete());
        assert!(!complete.is_partial());
        assert!(!complete.is_invalid());
        assert_eq!(complete.command(), Some("j"));

        let partial = ParsedCommand::Partial;
        assert!(!partial.is_complete());
        assert!(partial.is_partial());
        assert_eq!(partial.command(), None);

        let invalid = ParsedCommand::Invalid;
        assert!(!invalid.is_complete());
        assert!(invalid.is_invalid());
    }

    // Edge Cases

    #[test]
    fn test_parse_command_buffer_empty_buffer() {
        // Empty buffer should be treated as invalid (no command)
        let result = parse_command_buffer("");
        // Single-key logic treats len==1 as complete, but empty is len==0
        // Falls through to Invalid
        assert_matches!(result, ParsedCommand::Invalid);
    }

    #[test]
    fn test_parse_command_buffer_replace_special_chars() {
        // Replace with space
        assert_matches!(parse_command_buffer("r "), ParsedCommand::Complete(_));
        assert_eq!(parse_command_buffer("r ").command(), Some("r "));

        // Replace with newline
        assert_matches!(parse_command_buffer("r\n"), ParsedCommand::Complete(_));

        // Replace with tab
        assert_matches!(parse_command_buffer("r\t"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_find_special_chars() {
        // Find space
        assert_matches!(parse_command_buffer("f "), ParsedCommand::Complete(_));

        // Find uppercase reverse with space
        assert_matches!(parse_command_buffer("F "), ParsedCommand::Complete(_));

        // Till newline
        assert_matches!(parse_command_buffer("t\n"), ParsedCommand::Complete(_));

        // Till reverse with tab
        assert_matches!(parse_command_buffer("T\t"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_all_movement_commands() {
        // Basic movement
        assert_matches!(parse_command_buffer("h"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("l"), ParsedCommand::Complete(_));

        // Word movement
        assert_matches!(parse_command_buffer("w"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("b"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("e"), ParsedCommand::Complete(_));

        // WORD movement (uppercase)
        assert_matches!(parse_command_buffer("W"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("B"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("E"), ParsedCommand::Complete(_));

        // Line bounds
        // Note: "0" is NOT a command in Helix - use "gh" for goto line start
        // Note: "$" is NOT a line end command in Helix - use "gl" for goto line end
        assert_matches!(parse_command_buffer("0"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("$"), ParsedCommand::Invalid);
    }

    #[test]
    fn test_parse_command_buffer_all_editing_commands() {
        // Insert modes
        assert_matches!(parse_command_buffer("i"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("a"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("I"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("A"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("o"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("O"), ParsedCommand::Complete(_));

        // Change/delete
        assert_matches!(parse_command_buffer("c"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("d"), ParsedCommand::Complete(_));

        // Other editing
        assert_matches!(parse_command_buffer("J"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer(">"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("<"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("~"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_selection_commands() {
        assert_matches!(parse_command_buffer("x"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("X"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("%"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer(";"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("v"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_clipboard_commands() {
        assert_matches!(parse_command_buffer("y"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("p"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("P"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_undo_commands() {
        assert_matches!(parse_command_buffer("u"), ParsedCommand::Complete(_));
        assert_matches!(parse_command_buffer("U"), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_special_commands() {
        // Match mode prefix (waiting for second key)
        assert_matches!(parse_command_buffer("m"), ParsedCommand::Partial);

        // Match brackets (mm)
        assert_matches!(parse_command_buffer("mm"), ParsedCommand::Complete(_));

        // Repeat
        assert_matches!(parse_command_buffer("."), ParsedCommand::Complete(_));
    }

    #[test]
    fn test_parse_command_buffer_partial_all_prefixes() {
        // Goto prefix
        assert_matches!(parse_command_buffer("g"), ParsedCommand::Partial);

        // Match mode prefix
        assert_matches!(parse_command_buffer("m"), ParsedCommand::Partial);

        // Replace prefix
        assert_matches!(parse_command_buffer("r"), ParsedCommand::Partial);

        // Find prefixes
        assert_matches!(parse_command_buffer("f"), ParsedCommand::Partial);
        assert_matches!(parse_command_buffer("F"), ParsedCommand::Partial);

        // Till prefixes
        assert_matches!(parse_command_buffer("t"), ParsedCommand::Partial);
        assert_matches!(parse_command_buffer("T"), ParsedCommand::Partial);
    }

    #[test]
    fn test_parse_command_buffer_invalid_long_sequences() {
        // Three or more characters (except valid multi-key)
        assert_matches!(parse_command_buffer("rrr"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("fff"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("abc"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("jjj"), ParsedCommand::Invalid);

        // Invalid goto sequences
        assert_matches!(parse_command_buffer("gx"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("gz"), ParsedCommand::Invalid);
    }

    #[test]
    fn test_parse_command_buffer_invalid_double_char() {
        // dd is no longer valid (Helix uses xd for delete line)
        assert_matches!(parse_command_buffer("dd"), ParsedCommand::Invalid);

        // Other invalid doubles
        assert_matches!(parse_command_buffer("jj"), ParsedCommand::Invalid);
        assert_matches!(parse_command_buffer("kk"), ParsedCommand::Invalid);
    }

    #[test]
    fn test_parse_command_buffer_all_goto_variants() {
        // File bounds
        assert_eq!(
            parse_command_buffer("gg"),
            ParsedCommand::Complete("gg".to_string())
        );

        // Line bounds
        assert_eq!(
            parse_command_buffer("gh"),
            ParsedCommand::Complete("gh".to_string())
        );
        assert_eq!(
            parse_command_buffer("gl"),
            ParsedCommand::Complete("gl".to_string())
        );

        // First non-whitespace
        assert_eq!(
            parse_command_buffer("gs"),
            ParsedCommand::Complete("gs".to_string())
        );

        // Last line
        assert_eq!(
            parse_command_buffer("ge"),
            ParsedCommand::Complete("ge".to_string())
        );
    }

    #[test]
    fn test_parse_command_buffer_case_sensitivity() {
        // Note: 'G' is NOT a command in Helix - use 'ge' for goto last line
        assert!(parse_command_buffer("G").is_invalid());

        // Uppercase find command should work
        assert!(parse_command_buffer("F").is_partial()); // Needs char
        assert!(parse_command_buffer("Fx").is_complete());

        // Both are partial, but represent different commands
        assert!(parse_command_buffer("f").is_partial());
        assert!(parse_command_buffer("F").is_partial());

        // Complete forms differ in command string
        assert_ne!(
            parse_command_buffer("fx").command(),
            parse_command_buffer("Fx").command()
        );

        // Test other case-sensitive commands
        assert!(parse_command_buffer("w").is_complete()); // word forward
        assert!(parse_command_buffer("W").is_complete()); // WORD forward
    }

    // Count prefix tests
    #[test]
    fn test_parse_command_buffer_count_prefix_partial() {
        // Just digits should be partial
        assert!(parse_command_buffer("3").is_partial());
        assert!(parse_command_buffer("12").is_partial());
        assert!(parse_command_buffer("999").is_partial());
    }

    #[test]
    fn test_parse_command_buffer_count_prefix_complete() {
        // Count + single-key command
        assert_eq!(
            parse_command_buffer("3h"),
            ParsedCommand::Complete("3h".to_string())
        );
        assert_eq!(
            parse_command_buffer("5j"),
            ParsedCommand::Complete("5j".to_string())
        );
        assert_eq!(
            parse_command_buffer("10w"),
            ParsedCommand::Complete("10w".to_string())
        );
    }

    #[test]
    fn test_parse_command_buffer_count_prefix_invalid() {
        // Count + goto prefix is invalid
        assert!(parse_command_buffer("3g").is_invalid());
        // Count + char-input prefix is invalid
        assert!(parse_command_buffer("3f").is_invalid());
        assert!(parse_command_buffer("3r").is_invalid());
        // Count + multi-key is invalid
        assert!(parse_command_buffer("3gg").is_invalid());
    }

    #[test]
    fn test_extract_count_and_command_with_count() {
        assert_eq!(extract_count_and_command("3h"), (3, "h"));
        assert_eq!(extract_count_and_command("12j"), (12, "j"));
        assert_eq!(extract_count_and_command("999k"), (999, "k"));
    }

    #[test]
    fn test_extract_count_and_command_no_count() {
        assert_eq!(extract_count_and_command("h"), (1, "h"));
        assert_eq!(extract_count_and_command("gg"), (1, "gg"));
        assert_eq!(extract_count_and_command("fx"), (1, "fx"));
    }

    #[test]
    fn test_extract_count_and_command_edge_cases() {
        // Just digits - should return (1, original)
        assert_eq!(extract_count_and_command("3"), (1, "3"));
        // Empty - should return (1, empty)
        assert_eq!(extract_count_and_command(""), (1, ""));
    }
}
