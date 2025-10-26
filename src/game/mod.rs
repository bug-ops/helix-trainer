//! Game engine and session management.
//!
//! This module contains the core game logic, including scenario management,
//! user action tracking, and scoring.

pub mod editor_state;
pub mod scorer;
pub mod session;

pub use editor_state::{CursorPosition, EditorState, Selection};
pub use scorer::{PerformanceRating, Scorer};
pub use session::{Feedback, GameSession, SessionState, UserAction};
