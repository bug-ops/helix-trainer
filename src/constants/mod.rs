//! Application-wide constants
//!
//! This module contains all hardcoded values extracted into named constants
//! for better maintainability and documentation.

pub mod difficulty;
pub mod gameplay;
pub mod learning;
pub mod timing;
pub mod ui;
pub mod xp;

// Re-export commonly used constants
pub(crate) use difficulty::*;
pub use gameplay::*;
pub use learning::*;
pub use timing::*;
pub(crate) use ui::*;
pub(crate) use xp::*;
