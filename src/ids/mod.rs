//! Typed identifiers: scenario, quest, FSRS card and normalized-command strings.
//!
//! Each identifier is a newtype whose existence proves its invariant, so a scenario id can
//! no longer be passed where a quest id or a review-card id is expected. Persisted and TOML
//! values are validated at deserialization, and the on-disk JSON shape stays a plain string.

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
