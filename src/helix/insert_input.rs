//! Typed input of an Insert-mode simulator
//!
//! In Insert mode a command string is either typed text, `Escape`, or one of
//! the non-text keys ([`InsertAction`]). [`InsertInput`] names that split once,
//! so callers that already know which they have need not re-parse a string.

use super::repeat::InsertAction;
use crate::grammar::commands::CMD_ESCAPE;

/// One unit of input to an Insert-mode simulator.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::InsertInput;
///
/// assert_eq!(InsertInput::parse("Escape"), InsertInput::Escape);
/// assert_eq!(InsertInput::parse("x"), InsertInput::Text("x".to_string()));
/// assert_eq!(InsertInput::parse("Ctrl-w").command(), "Ctrl-w");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertInput {
    /// Typed text, inserted as is.
    Text(String),
    /// A non-text key: an arrow or an editing key such as `Ctrl-w` or `Tab`.
    Key(InsertAction),
    /// `Escape`: leave Insert mode.
    Escape,
}

impl InsertInput {
    /// Classify a command string the way the Insert-mode dispatcher always has.
    #[must_use]
    pub fn parse(cmd: &str) -> Self {
        if cmd == CMD_ESCAPE {
            Self::Escape
        } else if let Some(action) = InsertAction::parse(cmd) {
            Self::Key(action)
        } else {
            Self::Text(cmd.to_string())
        }
    }

    /// The canonical command string that replays this input.
    #[must_use]
    pub fn command(&self) -> &str {
        match self {
            Self::Text(text) => text,
            Self::Key(action) => action.command(),
            Self::Escape => CMD_ESCAPE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::repeat::Movement;
    use crate::helix::simulator::InsertCommand;

    #[test]
    fn parse_matches_the_dispatcher_branches() {
        assert_eq!(InsertInput::parse("Escape"), InsertInput::Escape);
        assert_eq!(
            InsertInput::parse("Left"),
            InsertInput::Key(InsertAction::Move(Movement::Left))
        );
        assert_eq!(
            InsertInput::parse("Alt-Backspace"),
            InsertInput::Key(InsertAction::Edit(InsertCommand::DeleteWordBackward))
        );
        assert_eq!(InsertInput::parse("a"), InsertInput::Text("a".to_string()));
        assert_eq!(InsertInput::parse(""), InsertInput::Text(String::new()));
    }

    #[test]
    fn command_round_trips_through_parse() {
        for cmd in ["Escape", "Up", "Ctrl-u", "Tab", "x", "hello", " "] {
            let input = InsertInput::parse(cmd);
            assert_eq!(InsertInput::parse(input.command()), input, "{cmd}");
        }
    }
}
