//! Typed failures of the Helix simulator.
//!
//! The simulator reports every rejected command as a [`SimulatorError`] so callers can
//! tell an unknown key from an invalid regex without matching on text. The conversion to the
//! sanitized, UI-facing [`UserError`] happens at the boundary, in the `From` impl below.

use crate::security::UserError;
use helix_core::regex;

/// Longest slice of caller-supplied text echoed back in an error message.
const MAX_ECHOED_CHARS: usize = 64;

/// Bound and sanitize caller-supplied text before it is stored in an error.
///
/// Error messages are single-line and shown to the user, so only ASCII graphic
/// characters and spaces survive, capped at 64 characters.
pub(crate) fn bound(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(MAX_ECHOED_CHARS)
        .collect()
}

/// Why a `:` command-line invocation was rejected.
///
/// Text fields are bounded and sanitized when the error is built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandLineError {
    /// The input did not start with `:`.
    #[error("not a command: '{input}'")]
    NotACommand {
        /// The offending input.
        input: String,
    },

    /// Nothing followed the `:`.
    #[error("empty command")]
    Empty,

    /// The command needs an argument and got none.
    #[error(":{name} requires one argument")]
    MissingArgument {
        /// The command name.
        name: String,
    },

    /// The command got more than one argument.
    #[error(":{name} takes exactly one argument")]
    TooManyArguments {
        /// The command name.
        name: String,
    },

    /// The argument is not an unsigned number.
    #[error(":{name} {argument}: not a number")]
    NotANumber {
        /// The command name.
        name: String,
        /// The offending argument.
        argument: String,
    },

    /// The command name is not a supported typable command.
    #[error("unknown command ':{name}'")]
    UnknownCommand {
        /// The command name.
        name: String,
    },
}

/// A command the simulator could not carry out.
///
/// Every variant leaves the editor state unchanged.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SimulatorError {
    /// The key names no registered command.
    #[error("unknown command '{command}'")]
    UnknownCommand {
        /// The offending key, bounded and sanitized.
        command: String,
    },

    /// A count was given to a command that does not take one.
    #[error("command does not accept a count")]
    CountNotAccepted,

    /// The pattern is not a valid regex.
    #[error("invalid regex: {0}")]
    InvalidRegex(#[source] regex::Error),

    /// The pattern is longer than the allowed maximum.
    #[error("pattern exceeds maximum length of {max} characters")]
    PatternTooLong {
        /// The maximum pattern length in characters.
        max: usize,
    },

    /// The bare command needs a pattern that only its prompt supplies.
    #[error("command needs a pattern from its prompt")]
    PatternRequired,

    /// `align` met a range that spans several lines.
    #[error("align cannot work with multi line selections")]
    AlignSpansLines,

    /// The command would create more cursors than the simulator allows.
    #[error("operation would create more than {max} cursors")]
    TooManyCursors {
        /// The maximum number of cursors.
        max: usize,
    },

    /// The operation would leave no selection.
    #[error("no selections remaining")]
    NoSelections,

    /// A register prefix was followed by an operation that does not use registers.
    #[error("unknown register operation '{op}'")]
    UnknownRegisterOp {
        /// The operation character.
        op: char,
    },

    /// A `:` command line reached Insert mode, where it would be typed as text.
    #[error("command-line is not available in insert mode")]
    CommandLineInInsertMode,

    /// A `:` command line was rejected.
    #[error(transparent)]
    CommandLine(#[from] CommandLineError),
}

impl From<SimulatorError> for UserError {
    fn from(err: SimulatorError) -> Self {
        UserError::CommandFailed {
            context: err.to_string(),
            cause: Some(err.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::ErrorCause;
    use std::assert_matches;

    #[test]
    fn bound_strips_control_and_non_ascii_and_caps_length() {
        assert_eq!(bound("a\nb\tc\u{e9}"), "abc");
        assert_eq!(bound(&"x".repeat(500)).len(), MAX_ECHOED_CHARS);
    }

    #[test]
    fn converts_to_command_failed_with_the_message() {
        let user = UserError::from(SimulatorError::NoSelections);
        assert!(matches!(user, UserError::CommandFailed { .. }));
        assert!(user.to_string().contains("no selections remaining"));
        assert_matches!(
            std::error::Error::source(&user).and_then(|e| e.downcast_ref::<ErrorCause>()),
            Some(ErrorCause::Simulator(SimulatorError::NoSelections))
        );
    }
}
