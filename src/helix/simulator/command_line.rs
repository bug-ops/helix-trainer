//! Parsing and execution of `:`-prefixed command-line invocations
//!
//! Only real, existing Helix typable commands are implemented here — see
//! [`CommandLine::parse`] for the exact set. This is intentionally narrow:
//! the trainer must never teach a command Helix does not have.

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;
use helix_core::Selection;

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
    /// [`UserError::CommandFailed`] with a sanitized, bounded message.
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
    pub fn parse(input: &str) -> Result<Self, UserError> {
        let body = input.strip_prefix(':').ok_or_else(|| {
            UserError::command_failed(format!("not a command: '{}'", bound(input)))
        })?;

        let mut parts = body.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| UserError::command_failed("empty command"))?;

        match name {
            "goto" | "g" => {
                let arg = parts.next().ok_or_else(|| {
                    UserError::command_failed(format!(":{} requires one argument", bound(name)))
                })?;
                if parts.next().is_some() {
                    return Err(UserError::command_failed(format!(
                        ":{} takes exactly one argument",
                        bound(name)
                    )));
                }
                let n: usize = arg.parse().map_err(|_| {
                    UserError::command_failed(format!(
                        ":{} {}: not a number",
                        bound(name),
                        bound(arg)
                    ))
                })?;
                Ok(Self::Goto(n))
            }
            other => Err(UserError::command_failed(format!(
                "unknown command ':{}'",
                bound(other)
            ))),
        }
    }

    /// Execute this command against the simulator.
    pub fn execute<M: EditorMode>(&self, sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
        match self {
            Self::Goto(n) => execute_goto(sim, *n),
        }
    }
}

/// `:goto N`, transcribed from helix-term 25.07.1's `goto_line` implementation:
/// `N` is 1-based, clamped to the document's last real line (excluding a
/// trailing empty line produced by a final newline), and moves the cursor to
/// line *start* (not first non-blank). `:goto 0` is a silent no-op, matching
/// Helix wrapping the argument in `NonZeroUsize::new`.
fn execute_goto<M: EditorMode>(sim: &mut HelixSimulator<M>, n: usize) -> Result<(), UserError> {
    if n == 0 {
        return Ok(());
    }

    let len_lines = sim.doc.len_lines();
    let last_line_is_empty = len_lines > 0 && sim.doc.line(len_lines - 1).len_chars() == 0;
    let max_line = if last_line_is_empty {
        len_lines.saturating_sub(2)
    } else {
        len_lines.saturating_sub(1)
    };

    let line_idx = (n - 1).min(max_line);
    let pos = sim.doc.line_to_char(line_idx);
    sim.selection = Selection::point(pos);
    Ok(())
}

/// Bound and sanitize text before embedding it in a user-facing error.
///
/// `UserError::CommandFailed`'s `Display` impl echoes `context` verbatim, so
/// anything derived from raw command-line input must be scrubbed here first.
/// Stricter than `sanitize_terminal_output` (which additionally allows
/// `\n`/`\t`): this text is embedded in a single-line error message, where a
/// literal newline or tab has no legitimate use.
fn bound(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;

    #[test]
    fn parses_goto_and_alias() {
        assert_eq!(CommandLine::parse(":goto 3").unwrap(), CommandLine::Goto(3));
        assert_eq!(CommandLine::parse(":g 3").unwrap(), CommandLine::Goto(3));
    }

    #[test]
    fn rejects_missing_colon() {
        assert!(CommandLine::parse("goto 3").is_err());
    }

    #[test]
    fn rejects_wrong_arity() {
        assert!(CommandLine::parse(":goto").is_err());
        assert!(CommandLine::parse(":goto 1 2").is_err());
    }

    #[test]
    fn rejects_non_numeric_argument() {
        assert!(CommandLine::parse(":goto abc").is_err());
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(CommandLine::parse(":nope").is_err());
        assert!(CommandLine::parse(":sort").is_err());
        assert!(CommandLine::parse(":clear-register a").is_err());
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
        sim.selection = Selection::point(5);
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
