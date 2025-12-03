//! Application-wide constants
//!
//! This module contains all hardcoded values extracted into named constants
//! for better maintainability and documentation.

pub mod difficulty;
pub mod gameplay;
pub mod timing;
pub mod ui;
pub mod xp;

// Re-export commonly used constants
pub use difficulty::*;
pub use gameplay::*;
pub use timing::*;
pub use ui::*;
pub use xp::*;
