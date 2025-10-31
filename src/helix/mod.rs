//! Helix editor integration using helix-core library
//!
//! This module provides a Helix editor simulator using the battle-tested
//! helix-core library. It handles text editing operations with proper
//! unicode support, undo/redo, and multi-cursor capabilities.
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::helix::HelixSimulator;
//!
//! let mut sim = HelixSimulator::new("hello world".to_string());
//! sim.execute_command("w")?;  // Move to next word
//! let state = sim.get_state()?;
//! assert_eq!(state.cursor_position().col, 6);
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```

pub mod executor;
pub mod simulator;

pub use executor::CommandExecutor;
pub use simulator::{HelixSimulator, Mode};
