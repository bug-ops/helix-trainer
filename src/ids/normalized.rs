//! Normalized command strings used by quests and scenario metadata.

use super::IdError;
use crate::helix::commands::normalize_command_id;
use crate::security::limits::MAX_COMMAND_LINE_LEN;
use serde::{Deserialize, Serialize, Serializer};
use std::borrow::Borrow;
use std::fmt;

/// A command string in the form produced by [`normalize_command_id`].
///
/// Counts and arguments that the normalizer keeps stay (`3h`, `"y`), so this is not an FSRS
/// card id; convert explicitly with [`CommandId::from_action`](super::CommandId::from_action).
/// Deserialization normalizes its input, so matching is never stricter than normalizing a
/// string and comparing it exactly.
///
/// # Examples
///
/// ```
/// use helix_trainer::ids::NormalizedCommand;
///
/// assert_eq!(NormalizedCommand::from_raw("\"ay")?.as_str(), "\"y");
/// assert_eq!(NormalizedCommand::from_raw("3h")?.as_str(), "3h");
/// assert!(NormalizedCommand::from_raw("").is_err());
/// # Ok::<(), helix_trainer::ids::IdError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct NormalizedCommand(Box<str>);

impl NormalizedCommand {
    /// Normalize `raw` and wrap the result.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::Empty`] for an empty command and [`IdError::TooLong`] above
    /// [`MAX_COMMAND_LINE_LEN`] bytes.
    pub fn from_raw(raw: &str) -> Result<Self, IdError> {
        let normalized = normalize_command_id(raw);
        if normalized.is_empty() {
            return Err(IdError::Empty);
        }
        if normalized.len() > MAX_COMMAND_LINE_LEN {
            return Err(IdError::TooLong {
                max: MAX_COMMAND_LINE_LEN,
            });
        }
        Ok(Self(normalized.into()))
    }

    /// The normalized command as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NormalizedCommand {
    type Error = IdError;

    fn try_from(raw: String) -> Result<Self, IdError> {
        Self::from_raw(&raw)
    }
}

impl fmt::Display for NormalizedCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Borrow<str> for NormalizedCommand {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Serialize for NormalizedCommand {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_idempotent() {
        for raw in [
            "h", "3h", "\"ay", "\"y", ":g 3", ":goto", "s foo", "s", "Ctrl-b", "ma(",
        ] {
            let once = NormalizedCommand::from_raw(raw).unwrap();
            let twice = NormalizedCommand::from_raw(once.as_str()).unwrap();
            assert_eq!(once, twice, "{raw}");
        }
    }

    #[test]
    fn rejects_empty_and_over_long() {
        assert_eq!(NormalizedCommand::from_raw(""), Err(IdError::Empty));
        assert!(NormalizedCommand::from_raw(&"x".repeat(MAX_COMMAND_LINE_LEN + 1)).is_err());
        assert!(NormalizedCommand::from_raw(&"x".repeat(MAX_COMMAND_LINE_LEN)).is_ok());
    }

    #[test]
    fn deserialization_canonicalizes() {
        let cmd: NormalizedCommand = serde_json::from_str("\"\\\"ay\"").unwrap();
        assert_eq!(cmd.as_str(), "\"y");
        assert!(serde_json::from_str::<NormalizedCommand>("\"\"").is_err());
    }
}
