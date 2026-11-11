//! User input handling
//!
//! Converts keyboard events into application messages based on current screen state.
//!
//! # Architecture
//!
//! Key-to-command mapping is handled by the [`typestate`] state machine; the
//! screen routing of raw key events lives in [`crate::ui::key_dispatch`].
//!
//! # Typestate-Based Input Handling
//!
//! The [`typestate`] module provides compile-time guarantees about input state
//! transitions. It encodes the current input context (base state, waiting for
//! character argument, building count prefix, etc.) at the type level.
//!
//! See [`typestate`] for the state machine implementation. The Task and Arcade
//! screens share one interpretation of a key through [`resolve`].

pub mod keymap;
pub(crate) mod resolve;
pub mod typestate;
