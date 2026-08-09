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

use std::time::{Duration, Instant};

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
pub trait SessionState: private::Sealed {
    /// Compute a session's elapsed time given its start and completion timestamps.
    ///
    /// Each state defines how "elapsed" is measured: states without a fixed
    /// end point (`Active`, `Abandoned`) measure time up to now, while
    /// `Completed` measures the fixed duration up to `completed_at`.
    ///
    /// Internal dispatch helper for `PlayableScenario::elapsed()`; hidden
    /// because the trait is sealed and has no external implementors.
    #[doc(hidden)]
    fn session_elapsed(started_at: Instant, completed_at: Option<Instant>) -> Duration;
}

// Implement sealed trait for our state markers
impl private::Sealed for Active {}
impl private::Sealed for Completed {}
impl private::Sealed for Abandoned {}

impl SessionState for Active {
    fn session_elapsed(started_at: Instant, _completed_at: Option<Instant>) -> Duration {
        started_at.elapsed()
    }
}

impl SessionState for Completed {
    fn session_elapsed(started_at: Instant, completed_at: Option<Instant>) -> Duration {
        completed_at
            .map(|end| end.duration_since(started_at))
            .unwrap_or_else(|| started_at.elapsed())
    }
}

impl SessionState for Abandoned {
    fn session_elapsed(started_at: Instant, _completed_at: Option<Instant>) -> Duration {
        started_at.elapsed()
    }
}

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

    #[test]
    fn test_active_session_elapsed_measures_since_start() {
        let started_at = Instant::now() - Duration::from_millis(50);
        let elapsed = Active::session_elapsed(started_at, None);
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[test]
    fn test_active_session_elapsed_ignores_completed_at() {
        let started_at = Instant::now() - Duration::from_millis(50);
        let completed_at = Some(Instant::now());
        let elapsed = Active::session_elapsed(started_at, completed_at);
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[test]
    fn test_completed_session_elapsed_uses_fixed_duration() {
        let started_at = Instant::now();
        let completed_at = started_at + Duration::from_millis(30);
        let elapsed = Completed::session_elapsed(started_at, Some(completed_at));
        assert_eq!(elapsed, Duration::from_millis(30));
    }

    #[test]
    fn test_completed_session_elapsed_falls_back_to_now_when_missing() {
        let started_at = Instant::now() - Duration::from_millis(20);
        let elapsed = Completed::session_elapsed(started_at, None);
        assert!(elapsed >= Duration::from_millis(20));
    }

    #[test]
    fn test_abandoned_session_elapsed_measures_since_start() {
        let started_at = Instant::now() - Duration::from_millis(20);
        let elapsed = Abandoned::session_elapsed(started_at, None);
        assert!(elapsed >= Duration::from_millis(20));
    }
}
