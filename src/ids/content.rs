//! Validated identifiers of TOML-defined content: scenarios and quests.

use super::IdError;
use crate::security::limits::MAX_ID_LENGTH;
use serde::{Deserialize, Serialize, Serializer};
use std::borrow::Borrow;
use std::fmt;
use std::str::FromStr;

fn validate(raw: &str) -> Result<(), IdError> {
    if raw.is_empty() {
        return Err(IdError::Empty);
    }
    if raw.len() > MAX_ID_LENGTH {
        return Err(IdError::TooLong { max: MAX_ID_LENGTH });
    }
    if !raw.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(IdError::InvalidCharacters);
    }
    Ok(())
}

macro_rules! content_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(Box<str>);

        impl $name {
            /// Validate `raw` and wrap it.
            ///
            /// # Errors
            ///
            /// Returns [`IdError`] when `raw` is empty, longer than 64 bytes, or contains a
            /// character other than alphanumerics and `_`.
            pub fn parse(raw: &str) -> Result<Self, IdError> {
                validate(raw)?;
                Ok(Self(raw.into()))
            }

            /// The identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(raw: String) -> Result<Self, IdError> {
                validate(&raw)?;
                Ok(Self(raw.into_boxed_str()))
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(raw: &str) -> Result<Self, IdError> {
                Self::parse(raw)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }
    };
}

content_id! {
    /// Identifier of a training scenario.
    ///
    /// One to 64 bytes of alphanumerics and `_`. Deserialization applies the same rule, so an
    /// invalid id in a scenario file or a saved profile is a load error of that document.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ids::ScenarioId;
    ///
    /// assert_eq!(ScenarioId::parse("delete_line_001")?.as_str(), "delete_line_001");
    /// assert!(ScenarioId::parse("bad id").is_err());
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    ScenarioId
}

content_id! {
    /// Identifier of a daily quest or quest template.
    ///
    /// Same rule as [`ScenarioId`], but a distinct type so the two cannot be mixed up.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ids::QuestId;
    ///
    /// assert_eq!(QuestId::parse("quest_easy_1")?.to_string(), "quest_easy_1");
    /// assert!(QuestId::parse("").is_err());
    /// # Ok::<(), helix_trainer::ids::IdError>(())
    /// ```
    QuestId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_long_and_invalid() {
        assert_eq!(ScenarioId::parse(""), Err(IdError::Empty));
        assert_eq!(
            ScenarioId::parse(&"a".repeat(65)),
            Err(IdError::TooLong { max: 64 })
        );
        assert_eq!(ScenarioId::parse("a-b"), Err(IdError::InvalidCharacters));
        assert_eq!(QuestId::parse("a b"), Err(IdError::InvalidCharacters));
        assert!(ScenarioId::parse(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn serde_round_trips_as_plain_string() {
        let id = ScenarioId::parse("abc_1").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"abc_1\"");
        assert_eq!(serde_json::from_str::<ScenarioId>(&json).unwrap(), id);
    }

    #[test]
    fn deserialization_validates() {
        assert!(serde_json::from_str::<QuestId>("\"bad id\"").is_err());
        assert!(serde_json::from_str::<QuestId>("\"\"").is_err());
        let too_long = format!("\"{}\"", "a".repeat(65));
        assert!(serde_json::from_str::<ScenarioId>(&too_long).is_err());
    }

    #[test]
    fn borrow_allows_str_lookup() {
        let mut map = std::collections::HashMap::new();
        map.insert(QuestId::parse("q1").unwrap(), 1);
        assert_eq!(map.get("q1"), Some(&1));
    }
}
