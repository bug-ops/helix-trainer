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
/// Capacity of the serialized profile-save writer's request queue.
///
/// Saves are infrequent (debounced) relative to this capacity; a full
/// queue would mean the writer is pathologically behind, in which case
/// callers fall back to a synchronous write rather than blocking.
pub const PROFILE_SAVE_QUEUE_CAPACITY: usize = 16;

// Data loading
/// Timeout for async data loading operations (5 seconds)
pub const DATA_LOADING_TIMEOUT: Duration = Duration::from_secs(5);

// Review timing
/// Optimal time for command review (3 seconds)
pub const OPTIMAL_REVIEW_TIME: Duration = Duration::from_secs(3);
