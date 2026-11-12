//! Notification system for displaying transient messages
//!
//! Provides a queue-based notification system that can show:
//! - Level-up notifications
//! - Achievement unlock notifications
//! - Quest completion notifications
//! - Streak milestone notifications
//!
//! Notifications auto-dismiss after a configurable duration (default 3 seconds).

use crate::constants::{
    DEFAULT_NOTIFICATION_DURATION, MAX_VISIBLE_NOTIFICATIONS, STREAK_FREEZE_MAX_GAP_DAYS,
};
use crate::game::services::CompletionEvent;
use crate::input::keymap::{KeymapStatus, StartupKeymap};
use crate::time::{MonotonicClock, SystemMonotonicClock};
use crate::ui::text;
use rust_i18n::t;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The startup notification text for how the keymap resolved, or `None` when the keymap
/// is disabled and there is nothing to tell.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::AppConfig;
/// use helix_trainer::input::keymap::resolve_startup_overlay;
/// use helix_trainer::ui::keymap_status_message;
///
/// let keymap = resolve_startup_overlay(&AppConfig::default());
/// assert!(keymap_status_message(&keymap).is_none());
/// ```
pub fn keymap_status_message(keymap: &StartupKeymap) -> Option<String> {
    match keymap.status {
        KeymapStatus::Disabled => None,
        KeymapStatus::Applied => Some(
            t!(
                "keymap.startup_summary",
                applied = keymap.report.applied,
                ignored = keymap.report.ignored.len()
            )
            .to_string(),
        ),
        KeymapStatus::NotFound => Some(t!("keymap.startup_not_found").to_string()),
        KeymapStatus::Invalid => Some(t!("keymap.startup_invalid").to_string()),
    }
}

/// Type of notification to display
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    /// User leveled up
    LevelUp {
        /// Level reached.
        new_level: u32,
    },

    /// Achievement unlocked
    Achievement {
        /// Achievement name.
        name: String,
        /// How it was earned.
        description: String,
    },

    /// Quest completed
    QuestComplete {
        /// Quest text.
        description: String,
        /// XP granted.
        xp_reward: u32,
    },

    /// Streak milestone reached
    StreakMilestone {
        /// Streak length that was reached.
        streak: u32,
    },

    /// Streak freeze earned (protects the streak across a brief absence)
    StreakFreezeGranted,

    /// Streak freeze consumed (an absence was protected by an earned freeze)
    StreakFreezeUsed,

    /// Streak broken (reset to zero)
    ///
    /// `freeze_could_not_cover_gap` distinguishes a break that happened
    /// despite a held freeze (the gap was longer than the freeze covers)
    /// from every other break reason (no freeze held, or a
    /// non-positive/backwards-clock gap). Only meaningful when
    /// `was_streak > 0` - this notification is only ever raised for an
    /// actual streak at stake.
    StreakBroken {
        /// Streak length before it was reset.
        was_streak: u32,
        /// Whether a held freeze was too short for the gap.
        freeze_could_not_cover_gap: bool,
    },

    /// Informational message
    Info {
        /// Text to show.
        message: String,
    },

    /// Review session completed
    ReviewSessionComplete {
        /// Reviews completed in the session.
        completed: usize,
        /// Reviews that succeeded.
        success_count: usize,
        /// XP earned during the session.
        xp_earned: u64,
    },

    /// Command mastery level increased
    MasteryLevelUp {
        /// Command that advanced.
        command: String,
        /// Display name of the new level.
        new_level: String,
    },
}

impl From<CompletionEvent> for NotificationType {
    fn from(event: CompletionEvent) -> Self {
        match event {
            CompletionEvent::AchievementUnlocked { id } => Self::Achievement {
                name: text::achievement_name(id),
                description: text::achievement_description(id),
            },
            CompletionEvent::LevelUp { new_level } => Self::LevelUp { new_level },
        }
    }
}

impl From<CompletionEvent> for Notification {
    fn from(event: CompletionEvent) -> Self {
        Self::new(event.into())
    }
}

/// A notification to display in the UI
///
/// It carries no timestamp: the [`NotificationQueue`] stamps it when it is pushed and decides
/// when it expires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// Type of notification
    pub notification_type: NotificationType,

    /// How long to display the notification (default 3 seconds)
    pub duration: Duration,
}

impl Notification {
    /// Create a new notification with default duration (3 seconds)
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ui::notification::{Notification, NotificationType};
    ///
    /// let notif = Notification::new(NotificationType::LevelUp { new_level: 5 });
    /// assert_eq!(notif.duration, std::time::Duration::from_secs(3));
    /// ```
    pub fn new(notification_type: NotificationType) -> Self {
        Self::with_duration(notification_type, DEFAULT_NOTIFICATION_DURATION)
    }

    /// Create a notification with custom duration
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ui::notification::{Notification, NotificationType};
    /// use std::time::Duration;
    ///
    /// let notif = Notification::with_duration(
    ///     NotificationType::LevelUp { new_level: 5 },
    ///     Duration::from_secs(5),
    /// );
    /// assert_eq!(notif.duration, Duration::from_secs(5));
    /// ```
    pub fn with_duration(notification_type: NotificationType, duration: Duration) -> Self {
        Self {
            notification_type,
            duration,
        }
    }

    /// Get title text for this notification
    pub fn title(&self) -> String {
        match &self.notification_type {
            NotificationType::LevelUp { new_level } => {
                t!("notification.level_up_title", level = new_level).to_string()
            }
            NotificationType::Achievement { name, .. } => {
                t!("notification.achievement_title", name = name).to_string()
            }
            NotificationType::QuestComplete { .. } => t!("notification.quest_title").to_string(),
            NotificationType::StreakMilestone { streak } => {
                t!("notification.streak_milestone_title", streak = streak).to_string()
            }
            NotificationType::StreakFreezeGranted => {
                t!("notification.streak_freeze_granted_title").to_string()
            }
            NotificationType::StreakFreezeUsed => {
                t!("notification.streak_freeze_used_title").to_string()
            }
            NotificationType::StreakBroken { .. } => {
                t!("notification.streak_broken_title").to_string()
            }
            NotificationType::Info { .. } => t!("notification.info_title").to_string(),
            NotificationType::ReviewSessionComplete { .. } => {
                t!("notification.review_complete_title").to_string()
            }
            NotificationType::MasteryLevelUp { command, .. } => {
                t!("notification.mastery_title", command = command).to_string()
            }
        }
    }

    /// Get message text for this notification
    pub fn message(&self) -> String {
        match &self.notification_type {
            NotificationType::LevelUp { new_level } => {
                t!("notification.level_up_message", level = new_level).to_string()
            }
            NotificationType::Achievement { description, .. } => description.clone(),
            NotificationType::QuestComplete {
                description,
                xp_reward,
            } => t!(
                "notification.quest_message",
                description = description,
                xp = xp_reward
            )
            .to_string(),
            NotificationType::StreakMilestone { streak } => {
                t!("notification.streak_milestone_message", streak = streak).to_string()
            }
            NotificationType::StreakFreezeGranted => t!(
                "notification.streak_freeze_granted_message",
                days = STREAK_FREEZE_MAX_GAP_DAYS
            )
            .to_string(),
            NotificationType::StreakFreezeUsed => {
                t!("notification.streak_freeze_used_message").to_string()
            }
            NotificationType::StreakBroken {
                was_streak,
                freeze_could_not_cover_gap: true,
            } => t!(
                "notification.streak_broken_freeze_message",
                streak = was_streak,
                days = STREAK_FREEZE_MAX_GAP_DAYS
            )
            .to_string(),
            NotificationType::StreakBroken { was_streak, .. } => {
                t!("notification.streak_broken_message", streak = was_streak).to_string()
            }
            NotificationType::Info { message } => message.clone(),
            NotificationType::ReviewSessionComplete {
                completed,
                success_count,
                xp_earned,
            } => t!(
                "notification.review_complete_message",
                success = success_count,
                completed = completed,
                xp = xp_earned
            )
            .to_string(),
            NotificationType::MasteryLevelUp { new_level, .. } => {
                t!("notification.mastery_message", level = new_level).to_string()
            }
        }
    }

    /// Get color for this notification type
    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match &self.notification_type {
            NotificationType::LevelUp { .. } => Color::Yellow,
            NotificationType::Achievement { .. } => Color::Magenta,
            NotificationType::QuestComplete { .. } => Color::Green,
            NotificationType::StreakMilestone { .. } => Color::Cyan,
            NotificationType::StreakFreezeGranted => Color::Cyan,
            NotificationType::StreakFreezeUsed => Color::Cyan,
            NotificationType::StreakBroken { .. } => Color::Red,
            NotificationType::Info { .. } => Color::Blue,
            NotificationType::ReviewSessionComplete { .. } => Color::Green,
            NotificationType::MasteryLevelUp { .. } => Color::Yellow,
        }
    }
}

/// A queued notification and the instant it was pushed
#[derive(Debug, Clone)]
struct Shown {
    notification: Notification,
    shown_at: Instant,
}

/// Queue for managing notifications
///
/// Reads time from an injected [`MonotonicClock`], so expiry is testable without sleeping.
#[derive(Clone)]
pub struct NotificationQueue {
    /// Active notifications
    notifications: Vec<Shown>,

    /// Maximum number of visible notifications
    max_visible: usize,

    clock: Arc<dyn MonotonicClock>,
}

impl fmt::Debug for NotificationQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotificationQueue")
            .field("notifications", &self.notifications)
            .field("max_visible", &self.max_visible)
            .finish_non_exhaustive()
    }
}

impl NotificationQueue {
    /// Create a new notification queue that reads time from the system clock
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ui::notification::NotificationQueue;
    ///
    /// let queue = NotificationQueue::new();
    /// assert_eq!(queue.count(), 0);
    /// assert!(queue.is_empty());
    /// ```
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemMonotonicClock))
    }

    /// Create a new notification queue that reads time from `clock`
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::time::SystemMonotonicClock;
    /// use helix_trainer::ui::notification::NotificationQueue;
    /// use std::sync::Arc;
    ///
    /// let queue = NotificationQueue::with_clock(Arc::new(SystemMonotonicClock));
    /// assert!(queue.is_empty());
    /// ```
    pub fn with_clock(clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            notifications: Vec::new(),
            max_visible: MAX_VISIBLE_NOTIFICATIONS,
            clock,
        }
    }

    /// Add a notification to the queue, starting its display time now
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ui::notification::{NotificationQueue, Notification, NotificationType};
    ///
    /// let mut queue = NotificationQueue::new();
    /// queue.push(Notification::new(NotificationType::LevelUp { new_level: 2 }));
    /// assert_eq!(queue.count(), 1);
    /// ```
    pub fn push(&mut self, notification: Notification) {
        self.notifications.push(Shown {
            notification,
            shown_at: self.clock.now(),
        });
    }

    /// Remove expired notifications from the queue
    ///
    /// Returns the number of notifications removed.
    pub fn remove_expired(&mut self) -> usize {
        let now = self.clock.now();
        let before = self.notifications.len();
        self.notifications.retain(|shown| {
            now.saturating_duration_since(shown.shown_at) < shown.notification.duration
        });
        before.saturating_sub(self.notifications.len())
    }

    /// Get visible notifications (up to `max_visible`, oldest first)
    ///
    /// Yields the most recent notifications that should be displayed.
    pub fn visible(&self) -> impl ExactSizeIterator<Item = &Notification> {
        let start = self.notifications.len().saturating_sub(self.max_visible);
        self.notifications
            .iter()
            .skip(start)
            .map(|shown| &shown.notification)
    }

    /// Get number of active notifications
    pub fn count(&self) -> usize {
        self.notifications.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    /// Clear all notifications
    pub fn clear(&mut self) {
        self.notifications.clear();
    }
}

impl Default for NotificationQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::FakeMonotonicClock;

    #[test]
    fn keymap_status_messages_keep_their_text() {
        use crate::input::keymap::KeymapOverlay;
        use crate::input::keymap::KeymapReport;

        let message = |status| {
            keymap_status_message(&StartupKeymap {
                overlay: KeymapOverlay::identity(),
                report: KeymapReport::default(),
                status,
            })
        };
        assert_eq!(message(KeymapStatus::Disabled), None);
        assert_eq!(
            message(KeymapStatus::Applied).as_deref(),
            Some("Helix keymap loaded: 0 binding(s) applied, 0 ignored")
        );
        assert_eq!(
            message(KeymapStatus::NotFound).as_deref(),
            Some("Helix keymap enabled, but config.toml was not found — using stock keymap")
        );
        assert_eq!(
            message(KeymapStatus::Invalid).as_deref(),
            Some("Helix keymap config could not be loaded — using stock keymap")
        );
    }

    #[test]
    fn test_notification_creation() {
        let notif = Notification::new(NotificationType::LevelUp { new_level: 5 });
        assert_eq!(notif.title(), "Level Up! Level 5");
        assert_eq!(notif.message(), "You've reached level 5!");
        assert_eq!(notif.duration, Duration::from_secs(3));
    }

    #[test]
    fn test_notification_with_custom_duration() {
        let notif = Notification::with_duration(
            NotificationType::LevelUp { new_level: 5 },
            Duration::from_secs(5),
        );
        assert_eq!(notif.duration, Duration::from_secs(5));
    }

    fn queue_with_fake_clock() -> (NotificationQueue, Arc<FakeMonotonicClock>) {
        let clock = Arc::new(FakeMonotonicClock::new());
        (NotificationQueue::with_clock(clock.clone()), clock)
    }

    #[test]
    fn test_notification_not_expired_immediately() {
        let (mut queue, _clock) = queue_with_fake_clock();
        queue.push(Notification::new(NotificationType::LevelUp {
            new_level: 5,
        }));

        assert_eq!(queue.remove_expired(), 0);
        assert_eq!(queue.count(), 1);
    }

    #[test]
    fn test_notification_expires_after_its_duration_on_the_injected_clock() {
        let (mut queue, clock) = queue_with_fake_clock();
        queue.push(Notification::with_duration(
            NotificationType::LevelUp { new_level: 5 },
            Duration::from_secs(3),
        ));

        clock.advance(Duration::from_millis(2999));
        assert_eq!(queue.remove_expired(), 0);

        clock.advance(Duration::from_millis(1));
        assert_eq!(queue.remove_expired(), 1);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_notifications_expire_independently_of_each_other() {
        let (mut queue, clock) = queue_with_fake_clock();
        queue.push(Notification::new(NotificationType::LevelUp {
            new_level: 1,
        }));
        clock.advance(Duration::from_secs(2));
        queue.push(Notification::new(NotificationType::LevelUp {
            new_level: 2,
        }));

        clock.advance(Duration::from_secs(1));

        assert_eq!(queue.remove_expired(), 1);
        assert_eq!(
            queue
                .visible()
                .map(|n| n.notification_type.clone())
                .collect::<Vec<_>>(),
            [NotificationType::LevelUp { new_level: 2 }]
        );
    }

    #[test]
    fn test_notification_queue_push_and_count() {
        let mut queue = NotificationQueue::new();
        assert_eq!(queue.count(), 0);
        assert!(queue.is_empty());

        queue.push(Notification::new(NotificationType::LevelUp {
            new_level: 2,
        }));
        assert_eq!(queue.count(), 1);
        assert!(!queue.is_empty());

        queue.push(Notification::new(NotificationType::LevelUp {
            new_level: 3,
        }));
        assert_eq!(queue.count(), 2);
    }

    #[test]
    fn test_notification_queue_visible_limits() {
        let mut queue = NotificationQueue::new();

        // Add 5 notifications
        for i in 1..=5 {
            queue.push(Notification::new(NotificationType::LevelUp {
                new_level: i,
            }));
        }

        // Should only show last 3 (max_visible)
        let levels: Vec<_> = queue
            .visible()
            .map(|n| n.notification_type.clone())
            .collect();

        // Check that we get the most recent 3
        assert_eq!(
            levels,
            [3, 4, 5].map(|new_level| NotificationType::LevelUp { new_level })
        );
    }

    #[test]
    fn test_notification_queue_clear() {
        let mut queue = NotificationQueue::new();
        queue.push(Notification::new(NotificationType::LevelUp {
            new_level: 2,
        }));
        assert_eq!(queue.count(), 1);

        queue.clear();
        assert_eq!(queue.count(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_notification_types_title_and_message() {
        let level_up = Notification::new(NotificationType::LevelUp { new_level: 10 });
        assert_eq!(level_up.title(), "Level Up! Level 10");
        assert_eq!(level_up.message(), "You've reached level 10!");

        let achievement = Notification::new(NotificationType::Achievement {
            name: "Speed Demon".to_string(),
            description: "Complete 10 scenarios in under 30 seconds".to_string(),
        });
        assert_eq!(achievement.title(), "Achievement Unlocked: Speed Demon");
        assert_eq!(
            achievement.message(),
            "Complete 10 scenarios in under 30 seconds"
        );

        let quest = Notification::new(NotificationType::QuestComplete {
            description: "Delete 5 lines".to_string(),
            xp_reward: 50,
        });
        assert_eq!(quest.title(), "Quest Complete!");
        assert_eq!(quest.message(), "Delete 5 lines (+50 XP)");

        let streak = Notification::new(NotificationType::StreakMilestone { streak: 7 });
        assert_eq!(streak.title(), "7 Day Streak!");
        assert_eq!(streak.message(), "Keep it up! 7 days in a row");

        let info = Notification::new(NotificationType::Info {
            message: "Test info message".to_string(),
        });
        assert_eq!(info.title(), "Info");
        assert_eq!(info.message(), "Test info message");

        let freeze_granted = Notification::new(NotificationType::StreakFreezeGranted);
        assert_eq!(freeze_granted.title(), "Streak Freeze Earned!");
        assert_eq!(
            freeze_granted.message(),
            format!(
                "Away for up to {STREAK_FREEZE_MAX_GAP_DAYS} days without breaking your streak"
            )
        );

        let freeze_used = Notification::new(NotificationType::StreakFreezeUsed);
        assert_eq!(freeze_used.title(), "Streak Freeze Used");
        assert_eq!(
            freeze_used.message(),
            "Your streak was protected while you were away"
        );

        // Regression coverage for #345: the two `StreakBroken` reject reasons
        // must render distinct copy so a cap-exceeded break (freeze held but
        // insufficient) doesn't read like a plain missed-day break.
        let streak_broken_no_freeze = Notification::new(NotificationType::StreakBroken {
            was_streak: 5,
            freeze_could_not_cover_gap: false,
        });
        assert_eq!(streak_broken_no_freeze.title(), "Streak Broken");
        assert_eq!(
            streak_broken_no_freeze.message(),
            "Your 5 day streak was reset after a missed day"
        );

        let streak_broken_freeze_insufficient = Notification::new(NotificationType::StreakBroken {
            was_streak: 5,
            freeze_could_not_cover_gap: true,
        });
        assert_eq!(streak_broken_freeze_insufficient.title(), "Streak Broken");
        assert_eq!(
            streak_broken_freeze_insufficient.message(),
            format!(
                "Your 5 day streak was reset - you were away longer than your freeze covers ({STREAK_FREEZE_MAX_GAP_DAYS} days max)"
            )
        );

        let review_complete = Notification::new(NotificationType::ReviewSessionComplete {
            completed: 5,
            success_count: 4,
            xp_earned: 60,
        });
        assert_eq!(review_complete.title(), "Review Complete!");
        assert_eq!(review_complete.message(), "4/5 correct (+60 XP)");

        let mastery_up = Notification::new(NotificationType::MasteryLevelUp {
            command: "dd".to_string(),
            new_level: "Intermediate".to_string(),
        });
        assert_eq!(mastery_up.title(), "Mastery Up: dd");
        assert_eq!(mastery_up.message(), "Reached Intermediate level!");
    }

    #[test]
    fn test_notification_colors() {
        use ratatui::style::Color;

        let level_up = Notification::new(NotificationType::LevelUp { new_level: 2 });
        assert_eq!(level_up.color(), Color::Yellow);

        let achievement = Notification::new(NotificationType::Achievement {
            name: "Test".to_string(),
            description: "Test".to_string(),
        });
        assert_eq!(achievement.color(), Color::Magenta);

        let quest = Notification::new(NotificationType::QuestComplete {
            description: "Test".to_string(),
            xp_reward: 25,
        });
        assert_eq!(quest.color(), Color::Green);

        let streak = Notification::new(NotificationType::StreakMilestone { streak: 5 });
        assert_eq!(streak.color(), Color::Cyan);

        let freeze_granted = Notification::new(NotificationType::StreakFreezeGranted);
        assert_eq!(freeze_granted.color(), Color::Cyan);

        let info = Notification::new(NotificationType::Info {
            message: "Test".to_string(),
        });
        assert_eq!(info.color(), Color::Blue);

        let review_complete = Notification::new(NotificationType::ReviewSessionComplete {
            completed: 3,
            success_count: 2,
            xp_earned: 40,
        });
        assert_eq!(review_complete.color(), Color::Green);

        let mastery_up = Notification::new(NotificationType::MasteryLevelUp {
            command: "w".to_string(),
            new_level: "Advanced".to_string(),
        });
        assert_eq!(mastery_up.color(), Color::Yellow);
    }

    #[test]
    fn test_notification_queue_default() {
        let queue = NotificationQueue::default();
        assert_eq!(queue.count(), 0);
        assert!(queue.is_empty());
    }
}
