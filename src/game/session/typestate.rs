//! Type-state pattern for GameSession
//!
//! This module implements compile-time session state management using the typestate pattern.
//! It ensures that session operations are only available in valid states, preventing bugs
//! like recording actions on completed sessions.
//!
//! # Architecture
//!
//! - Zero-sized marker types: `Active`, `Completed`, `Abandoned`
//! - Sealed trait `SessionState` to control which types can be session states
//! - Generic `GameSession<State>` parameterized by state marker
//!
//! # State Transitions
//!
//! ```text
//!        new()
//!          |
//!          v
//!     [Active] ----abandon()----> [Abandoned]
//!          |
//!   record_action()
//!          |
//!          v
//!   SessionAfterAction::Completed
//!          |
//!          v
//!     [Completed]
//! ```

/// State marker for active game session
///
/// Sessions in this state can record actions, get hints, and check for completion.
#[derive(Debug)]
pub struct Active;

/// State marker for completed game session
///
/// Sessions in this state can calculate scores and provide feedback.
#[derive(Debug)]
pub struct Completed;

/// State marker for abandoned game session
///
/// Sessions in this state can only provide basic feedback (score = 0).
#[derive(Debug)]
pub struct Abandoned;

/// Private module for sealed trait pattern
///
/// This prevents external crates from implementing `SessionState`.
mod private {
    /// Sealed trait marker
    ///
    /// By placing this in a private module, we prevent external implementation.
    pub trait Sealed {}
}

/// Trait for valid session states
///
/// This is a sealed trait - only `Active`, `Completed`, and `Abandoned` can implement it.
/// This ensures type safety and allows us to add methods in the future without breaking changes.
pub trait SessionState: private::Sealed {}

// Implement sealed trait for our state markers
impl private::Sealed for Active {}
impl private::Sealed for Completed {}
impl private::Sealed for Abandoned {}

impl SessionState for Active {}
impl SessionState for Completed {}
impl SessionState for Abandoned {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_markers_implement_debug() {
        // All markers should implement Debug
        let _active = format!("{:?}", Active);
        let _completed = format!("{:?}", Completed);
        let _abandoned = format!("{:?}", Abandoned);
    }
}
