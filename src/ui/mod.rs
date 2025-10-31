//! Terminal user interface components
//!
//! This module contains all TUI-related functionality built with ratatui.
//! It implements the Elm Architecture pattern for predictable, testable state management.
//!
//! # Architecture Overview
//!
//! The UI follows the Elm Architecture with three main components:
//!
//! - **State (`state.rs`)**: Centralized `AppState` containing all UI state
//! - **Messages (`state.rs`)**: User actions that trigger state changes
//! - **Update (`state.rs`)**: Pure function that transforms state based on messages
//! - **Rendering (`render.rs`)**: Pure functions that render UI based on state
//!
//! # Example Usage
//!
//! ```ignore
//! use helix_trainer::ui::{AppState, Message, update, render, Screen};
//! use helix_trainer::config::ScenarioLoader;
//!
//! // Load scenarios
//! let loader = ScenarioLoader::with_default_paths()?;
//! let scenarios = loader.load_from_file("scenarios.toml")?.scenarios;
//!
//! // Initialize app state
//! let mut app_state = AppState::new(scenarios);
//!
//! // In event loop:
//! let message = Message::MenuDown;
//! update(&mut app_state, message)?;
//!
//! // Render:
//! terminal.draw(|f| render(f, &app_state))?;
//! ```

pub mod render;
pub mod state;

pub use render::render;
pub use state::{AppState, Message, Screen, update};
