//! Helix Keybindings Trainer
//!
//! An interactive terminal user interface (TUI) application for learning Helix editor keybindings.
//! The trainer presents interactive scenarios where users practice Helix commands and receive
//! immediate feedback on their performance.
//!
//! # Architecture
//!
//! The application is organized into several modules:
//!
//! - `config`: Configuration and scenario loading
//! - `game`: Game engine and session management
//! - `helix`: Helix editor integration and PTY control
//! - `ui`: Terminal user interface components built with ratatui
//! - `security`: Security utilities, validation, and error handling

pub mod config;
pub mod game;
pub mod helix;
pub mod security;
pub mod ui;
