//! Mini-game arcade-style training mode
//!
//! This module provides an arcade-style gameplay experience where scenarios "rain down"
//! with time pressure and progressive difficulty. Unlike deliberate practice mode,
//! mini-games focus on speed and muscle memory building.
//!
//! # Architecture
//!
//! The mini-game system consists of:
//! - `MiniGameSession` - Main game session state and logic
//! - `MiniGameStats` - Score, lives, streak, multiplier tracking
//! - `MiniGameState` - State machine for game flow
//! - `DifficultyController` - Adaptive difficulty and scenario selection
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::minigame::MiniGameSession;
//! use helix_trainer::config::ScenarioCollection;
//! use std::sync::Arc;
//!
//! let scenarios = Arc::new(ScenarioCollection::load()?);
//! let mut session = MiniGameSession::new(scenarios, None);
//!
//! session.start();
//! session.handle_command("x")?; // select line
//! session.handle_command("d")?; // delete selection
//!
//! if session.check_completion() {
//!     session.advance_to_next();
//! }
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```

mod difficulty;
mod scorer;
mod scoring;
mod session;
mod state;
mod stats;

#[cfg(test)]
mod tests;

// Re-export public types
pub use difficulty::{DifficultyController, LevelChange, PerformancePoint};
pub use scorer::ScenarioScorer;
pub use scoring::{PerformanceTier, ScoreBreakdown, ScoreCalculator};
pub use session::{ActiveMiniScenario, MiniGameSession};
pub use state::MiniGameState;
pub use stats::{MiniGameStats, MultiplierChange, MultiplierState};
