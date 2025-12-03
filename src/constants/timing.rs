//! Timing-related constants
//!
//! All duration and timing values used throughout the application.

use std::time::Duration;

// Event loop timing
/// Interval for animation/tick updates (100ms)
pub const ANIMATION_TICK_INTERVAL: Duration = Duration::from_millis(100);
/// Interval for countdown ticks (1 second)
pub const COUNTDOWN_TICK_INTERVAL: Duration = Duration::from_secs(1);
/// Delay before showing results after success (1.5 seconds)
pub const SUCCESS_SCREEN_DELAY: Duration = Duration::from_millis(1500);

// Notification timing
/// Default duration for notification display (3 seconds)
pub const DEFAULT_NOTIFICATION_DURATION: Duration = Duration::from_secs(3);

// Profile/data saving
/// Debounce interval for profile saves (5 seconds)
pub const PROFILE_SAVE_DEBOUNCE: Duration = Duration::from_secs(5);

// Security/session timing
/// Command execution timeout (30 seconds)
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
/// Session timeout (1 hour)
pub const SESSION_TIMEOUT: Duration = Duration::from_secs(3600);
/// Minimum interval between scenario loads (100ms)
pub const MIN_LOAD_INTERVAL: Duration = Duration::from_millis(100);

// Data loading
/// Timeout for async data loading operations (5 seconds)
pub const DATA_LOADING_TIMEOUT: Duration = Duration::from_secs(5);

// Review timing
/// Optimal time for command review (3 seconds)
pub const OPTIMAL_REVIEW_TIME: Duration = Duration::from_secs(3);
