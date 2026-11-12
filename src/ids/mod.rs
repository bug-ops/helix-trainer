//! Typed identifiers: scenario, quest, FSRS card and normalized-command strings.
//!
//! Each identifier is a newtype whose existence proves its invariant, so a scenario id can
//! no longer be passed where a quest id or a review-card id is expected. Persisted and TOML
//! values are validated at deserialization, and the on-disk JSON shape stays a plain string.

/// Implement the string accessors every id newtype over `Box<str>` shares: `as_str`,
/// `Display`, `Borrow<str>` and `Serialize` (as a plain string). Each type keeps its own
/// validating constructors.
macro_rules! str_id_impls {
    ($name:ident) => {
        impl $name {
            /// The identifier as a string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl ::std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }
    };
}

mod command;
mod content;
mod normalized;

pub use command::CommandId;
pub use content::{QuestId, ScenarioId};
pub use normalized::NormalizedCommand;

use thiserror::Error;

/// Why a string was rejected as an identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IdError {
    /// The identifier is empty.
    #[error("identifier is empty")]
    Empty,
    /// The identifier is longer than the allowed number of bytes.
    #[error("identifier is longer than {max} bytes")]
    TooLong {
        /// Maximum accepted length in bytes.
        max: usize,
    },
    /// The identifier contains a character outside alphanumerics and `_`.
    #[error("identifier may contain only alphanumeric characters and underscores")]
    InvalidCharacters,
    /// The string is not the canonical form of a review-card id.
    #[error("identifier is not a canonical command card id")]
    NotCanonical,
}
