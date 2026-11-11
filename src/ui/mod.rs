//! Terminal user interface components
//!
//! This module contains all TUI-related functionality built with ratatui.
//! It implements the Elm Architecture pattern for predictable, testable state management.
//!
//! # Architecture Overview
//!
//! The UI follows the Elm Architecture with three main components:
//!
//! - **State (`state/`)**: Centralized `AppState` containing all UI state
//! - **Messages (`state/`)**: User actions that trigger state changes
//! - **Update (`state/`)**: Function that transforms state based on messages and returns side effects
//! - **Rendering (`render/`)**: Pure functions that render UI based on state
//!
//! # Example Usage
//!
//! ```
//! use helix_trainer::gamification::{ProfileStorage, UserProfile};
//! use helix_trainer::learning::PerformanceTracker;
//! use helix_trainer::ui::{AppState, Message, render, update};
//! use ratatui::{Terminal, backend::TestBackend};
//!
//! let storage = ProfileStorage::with_path(std::env::temp_dir().join("doc-profile.json"));
//! let mut app_state = AppState::new(vec![], UserProfile::new(), storage, PerformanceTracker::new());
//!
//! // In the event loop: apply a message, then run the returned side effects.
//! let mut effects = Vec::new();
//! update(&mut app_state, Message::ModeSelectionDown, &mut effects)?;
//!
//! // Render the new state (a test backend stands in for the terminal).
//! let mut terminal = Terminal::new(TestBackend::new(80, 24))?;
//! terminal.draw(|f| render(f, &mut app_state))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod notification;
pub mod render;
pub mod state;

pub use notification::{Notification, NotificationQueue, NotificationType};
pub use render::render;
pub use state::{AppState, Message, NavTarget, update};
