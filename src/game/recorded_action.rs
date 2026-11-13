//! What a player did, kept typed
//!
//! A session records every executed action as a [`RecordedAction`]: a parsed
//! Normal-mode command or typed Insert-mode input. The review-card id of a
//! command is derived once, when the action is recorded, so completion checks,
//! FSRS and quests never re-parse a string.

use std::borrow::Cow;
use std::time::Duration;

use crate::helix::InsertInput;
use crate::helix::normal_command::NormalCommand;
use crate::ids::CommandId;

/// One executed action: a Normal-mode command or Insert-mode input.
///
/// # Examples
///
/// ```
/// use helix_trainer::game::RecordedAction;
/// use helix_trainer::helix::InsertInput;
/// use helix_trainer::helix::normal_command::NormalCommand;
///
/// let delete = RecordedAction::Command(NormalCommand::parse("3d")?);
/// assert_eq!(delete.text(), "3d");
/// let typed = RecordedAction::Insert(InsertInput::parse("x"));
/// assert_eq!(typed.text(), "x");
/// # Ok::<(), helix_trainer::helix::error::SimulatorError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordedAction {
    /// A parsed Normal-mode command, count included.
    Command(NormalCommand),
    /// Typed text, `Escape` or an Insert-mode key.
    Insert(InsertInput),
}

impl RecordedAction {
    /// The canonical command string of the action.
    #[must_use]
    pub fn text(&self) -> Cow<'_, str> {
        match self {
            Self::Command(command) => Cow::Owned(command.to_string()),
            Self::Insert(input) => Cow::Borrowed(input.command()),
        }
    }

    /// The review-card id: Insert-mode input never makes a card.
    fn command_id(&self) -> Option<CommandId> {
        match self {
            Self::Command(command) => command.command_id(),
            Self::Insert(_) => None,
        }
    }
}

/// An action taken during gameplay with its timestamp.
///
/// Stores the typed action and the time it happened, for action-sequence and
/// timing analytics.
///
/// # Examples
///
/// ```
/// use helix_trainer::game::{RecordedAction, UserAction};
/// use helix_trainer::helix::normal_command::NormalCommand;
/// use std::time::Duration;
///
/// let delete = RecordedAction::Command(NormalCommand::parse("d")?);
/// let action = UserAction::new(delete, Duration::from_secs(1));
/// assert_eq!(action.command_id().map(|id| id.as_str()), Some("d"));
/// assert_eq!(action.timestamp, Duration::from_secs(1));
/// # Ok::<(), helix_trainer::helix::error::SimulatorError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAction {
    action: RecordedAction,
    command_id: Option<CommandId>,
    /// Elapsed time when action was performed
    pub timestamp: Duration,
}

impl UserAction {
    /// Record `action` as taken at `timestamp`.
    #[must_use]
    pub fn new(action: RecordedAction, timestamp: Duration) -> Self {
        let command_id = action.command_id();
        Self {
            action,
            command_id,
            timestamp,
        }
    }

    /// What was done.
    #[must_use]
    pub fn action(&self) -> &RecordedAction {
        &self.action
    }

    /// The review-card id of the action, `None` for Insert-mode input and `Escape`.
    #[must_use]
    pub fn command_id(&self) -> Option<&CommandId> {
        self.command_id.as_ref()
    }
}
