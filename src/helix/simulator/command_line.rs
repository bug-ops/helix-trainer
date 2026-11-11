//! Parsing and execution of `:`-prefixed command-line invocations
//!
//! Only real, existing Helix typable commands are implemented here — see
//! [`CommandLine::parse`] for the exact set. This is intentionally narrow:
//! the trainer must never teach a command Helix does not have.

use crate::helix::error::{CommandLineError, SimulatorError, bound};
use crate::helix::simulator::{EditorMode, HelixSimulator};

/// A parsed `:` command-line invocation, ready to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLine {
    /// `:goto N` / `:g N` — move the cursor to the start of line `N` (1-based).
    Goto(usize),
}

impl CommandLine {
    /// Parse a `:`-prefixed command-line string (the leading colon included).
    ///
    /// Supports exactly `:goto N` and its alias `:g N`. Any other command
    /// name, wrong argument count, or non-numeric argument is a
    /// [`CommandLineError`] with a sanitized, bounded message.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::simulator::command_line::CommandLine;
    ///
    /// assert_eq!(CommandLine::parse(":goto 3").unwrap(), CommandLine::Goto(3));
    /// assert_eq!(CommandLine::parse(":g 3").unwrap(), CommandLine::Goto(3));
    /// assert!(CommandLine::parse(":nope").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a [`SimulatorError`] if `input` does not start with `:`, names no command or an unknown
    /// command, or has a missing, surplus or non-numeric argument.
    pub fn parse(input: &str) -> Result<Self, CommandLineError> {
        let body = input
            .strip_prefix(':')
            .ok_or_else(|| CommandLineError::NotACommand {
                input: bound(input),
            })?;

        let mut parts = body.split_whitespace();
        let name = parts.next().ok_or(CommandLineError::Empty)?;

        match name {
            "goto" | "g" => {
                let arg = parts
                    .next()
                    .ok_or_else(|| CommandLineError::MissingArgument { name: bound(name) })?;
                if parts.next().is_some() {
                    return Err(CommandLineError::TooManyArguments { name: bound(name) });
                }
                let n: usize = arg.parse().map_err(|_| CommandLineError::NotANumber {
                    name: bound(name),
                    argument: bound(arg),
                })?;
                Ok(Self::Goto(n))
            }
            other => Err(CommandLineError::UnknownCommand { name: bound(other) }),
        }
    }

    /// Execute this command against the simulator.
    ///
    /// # Errors
    ///
    /// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
    /// with the fallible commands that share the registry's function type.
    pub fn execute<M: EditorMode>(
        &self,
        sim: &mut HelixSimulator<M>,
    ) -> Result<(), SimulatorError> {
        match self {
            Self::Goto(n) => {
                execute_goto(sim, *n);
                Ok(())
            }
        }
    }
}

/// `:goto N`, transcribed from helix-term 25.07.1's `goto_line` implementation:
/// `N` is 1-based, clamped to the document's last real line (excluding a
/// trailing empty line produced by a final newline), and moves the cursor to
/// line *start* (not first non-blank). `:goto 0` is a silent no-op, matching
/// Helix wrapping the argument in `NonZeroUsize::new`.
fn execute_goto<M: EditorMode>(sim: &mut HelixSimulator<M>, n: usize) {
    if let Some(count) = std::num::NonZeroUsize::new(n) {
        super::commands::movement::goto_line(sim, count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;
    use std::assert_matches;

    #[test]
    fn parses_goto_and_alias() {
        assert_eq!(CommandLine::parse(":goto 3").unwrap(), CommandLine::Goto(3));
        assert_eq!(CommandLine::parse(":g 3").unwrap(), CommandLine::Goto(3));
    }

    #[test]
    fn rejects_missing_colon() {
        assert_matches!(
            CommandLine::parse("goto 3"),
            Err(CommandLineError::NotACommand { input }) if input == "goto 3"
        );
    }

    #[test]
    fn rejects_empty_command() {
        assert_matches!(CommandLine::parse(":"), Err(CommandLineError::Empty));
    }

    #[test]
    fn rejects_wrong_arity() {
        assert_matches!(
            CommandLine::parse(":goto"),
            Err(CommandLineError::MissingArgument { name }) if name == "goto"
        );
        assert_matches!(
            CommandLine::parse(":goto 1 2"),
            Err(CommandLineError::TooManyArguments { name }) if name == "goto"
        );
    }

    #[test]
    fn rejects_non_numeric_argument() {
        assert_matches!(
            CommandLine::parse(":goto abc"),
            Err(CommandLineError::NotANumber { name, argument })
                if name == "goto" && argument == "abc"
        );
    }

    #[test]
    fn rejects_unknown_command() {
        for input in [":nope", ":sort", ":clear-register a"] {
            assert_matches!(
                CommandLine::parse(input),
                Err(CommandLineError::UnknownCommand { .. }),
                "{input}"
            );
        }
    }

    #[test]
    fn goto_moves_cursor_to_line_start() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("aaa\nbbb\nccc\n".to_string());
        CommandLine::Goto(2).execute(&mut sim).unwrap();
        assert_eq!(sim.selection.primary().head, 4); // start of "bbb"
    }

    #[test]
    fn goto_zero_is_silent_noop() {
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("aaa\nbbb\nccc\n".to_string());
        sim.selection = helix_core::Selection::point(5);
        CommandLine::Goto(0).execute(&mut sim).unwrap();
        assert_eq!(sim.selection.primary().head, 5); // unchanged
    }

    #[test]
    fn goto_clamps_past_end_ignoring_trailing_blank_line() {
        // Trailing newline produces an empty final line; max real line is "ccc".
        let mut sim: HelixSimulator<NormalMode> =
            HelixSimulator::new("aaa\nbbb\nccc\n".to_string());
        CommandLine::Goto(100).execute(&mut sim).unwrap();
        assert_eq!(sim.selection.primary().head, 8); // start of "ccc"
    }

    #[test]
    fn goto_clamps_past_end_without_trailing_newline() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("aaa\nbbb\nccc".to_string());
        CommandLine::Goto(100).execute(&mut sim).unwrap();
        assert_eq!(sim.selection.primary().head, 8); // start of "ccc"
    }

    #[test]
    fn error_message_bounds_and_sanitizes_echoed_input() {
        let long_garbage = "x".repeat(500);
        let err = CommandLine::parse(&format!(":{long_garbage}")).unwrap_err();
        let message = err.to_string();
        assert!(message.len() < 200, "message should be bounded: {message}");
    }

    #[test]
    fn error_message_strips_newlines_and_tabs() {
        // Defense in depth: unreachable from the UI today (the input filter
        // is ascii_graphic+space), but the echoed text must never carry a
        // raw newline/tab/CR into a single-line error message. The
        // "not a command" path is the one place the full raw input (not
        // already whitespace-split) reaches `bound()`.
        let err = CommandLine::parse("no-colon\nwith\ttabs\rhere").unwrap_err();
        let message = err.to_string();
        assert!(!message.contains('\n'));
        assert!(!message.contains('\t'));
        assert!(!message.contains('\r'));
    }
}
