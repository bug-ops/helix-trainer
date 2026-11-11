//! The key grammar shared by every layer
//!
//! Small, dependency-free vocabulary that the input layer, the simulator and
//! the identity types all need, so no one of them has to own it and none has
//! to import another to name it.

pub mod find;
