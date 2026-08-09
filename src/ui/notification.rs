//! Notification system for displaying transient messages
//!
//! Provides a queue-based notification system that can show:
//! - Level-up notifications
//! - Achievement unlock notifications
//! - Quest completion notifications
//! - Streak milestone notifications
//!
//! Notifications auto-dismiss after a configurable duration (default 3 seconds).

use crate::constants::{DEFAULT_NOTIFICATION_DURATION, MAX_VISIBLE_NOTIFICATIONS};
use std::time::{Duration, Instant};

/// Type of notification to display
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    /// User leveled up
    LevelUp { new_level: u32 },

    /// Achievement unlocked
    Achievement { name: String, description: String },

    /// Quest completed
    QuestComplete { description: String, xp_reward: u32 },

    /// Streak milestone reached
    StreakMilestone { streak: u32 },

    /// Streak freeze earned (protects the streak if a day is missed)
    StreakFreezeGranted,

    /// Streak freeze consumed (a missed day was protected by an earned freeze)
    StreakFreezeUsed,

    /// Informational message
    Info { message: String },

    /// Review session completed
    ReviewSessionComplete {
        completed: usize,
        success_count: usize,
        xp_earned: u64,
    },

    /// Command mastery level increased
    MasteryLevelUp { command: String, new_level: String },
}

/// A notification to display in the UI
#[derive(Debug, Clone)]
pub struct Notification {
    /// Type of notification
    pub notification_type: NotificationType,

    /// When the notification was created
    /// Note: This field is excluded from PartialEq comparison
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) created_at: Instant,

    /// How long to display the notification (default 3 seconds)
    pub duration: Duration,
}

// Custom PartialEq that excludes created_at (Instant doesn't implement Eq)
impl PartialEq for Notification {
    fn eq(&self, other: &Self) -> bool {
        self.notification_type == other.notification_type && self.duration == other.duration
    }
}

// Implement Eq manually since we have a valid PartialEq
impl Eq for Notification {}

impl Notification {
    /// Create a new notification with default duration (3 seconds)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::ui::notification::{Notification, NotificationType};
    ///
    /// let notif = Notification::new(NotificationType::LevelUp { new_level: 5 });
    /// assert!(!notif.is_expired());
    /// ```
    pub fn new(notification_type: NotificationType) -> Self {
        Self {
            notification_type,
            created_at: Instant::now(),
            duration: DEFAULT_NOTIFICATION_DURATION,
        }
    }

    /// Create a notification with custom duration
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::ui::notification::{Notification, NotificationType};
    /// use std::time::Duration;
    ///
    /// let notif = Notification::with_duration(
    ///     NotificationType::LevelUp { new_level: 5 },
    ///     Duration::from_secs(5),
    /// );
    /// ```
    pub fn with_duration(notification_type: NotificationType, duration: Duration) -> Self {
        Self {
            notification_type,
            created_at: Instant::now(),
            duration,
        }
    }

    /// Check if the notification has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }

    /// Get remaining time before expiration
    pub fn remaining_time(&self) -> Duration {
        self.duration.saturating_sub(self.created_at.elapsed())
    }

    /// Get title text for this notification
    pub fn title(&self) -> String {
        match &self.notification_type {
            NotificationType::LevelUp { new_level } => format!("Level Up! Level {}", new_level),
            NotificationType::Achievement { name, .. } => format!("Achievement Unlocked: {}", name),
            NotificationType::QuestComplete { .. } => "Quest Complete!".to_string(),
            NotificationType::StreakMilestone { streak } => format!("{} Day Streak!", streak),
            NotificationType::StreakFreezeGranted => "Streak Freeze Earned!".to_string(),
            NotificationType::StreakFreezeUsed => "Streak Freeze Used".to_string(),
            NotificationType::Info { .. } => "Info".to_string(),
            NotificationType::ReviewSessionComplete { .. } => "Review Complete!".to_string(),
            NotificationType::MasteryLevelUp { command, .. } => {
                format!("Mastery Up: {}", command)
            }
        }
    }

    /// Get message text for this notification
    pub fn message(&self) -> String {
        match &self.notification_type {
            NotificationType::LevelUp { new_level } => {
                format!("You've reached level {}!", new_level)
            }
            NotificationType::Achievement { description, .. } => description.clone(),
            NotificationType::QuestComplete {
                description,
                xp_reward,
            } => {
                format!("{} (+{} XP)", description, xp_reward)
            }
            NotificationType::StreakMilestone { streak } => {
                format!("Keep it up! {} days in a row", streak)
            }
            NotificationType::StreakFreezeGranted => {
                "Miss a day without breaking your streak".to_string()
            }
            NotificationType::StreakFreezeUsed => {
                "Your streak was protected after a missed day".to_string()
            }
            NotificationType::Info { message } => message.clone(),
            NotificationType::ReviewSessionComplete {
                completed,
                success_count,
                xp_earned,
            } => {
                format!(
                    "{}/{} correct (+{} XP)",
                    success_count, completed, xp_earned
                )
            }
            NotificationType::MasteryLevelUp { new_level, .. } => {
                format!("Reached {} level!", new_level)
            }
        }
    }

    /// Get color for this notification type
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match &self.notification_type {
            NotificationType::LevelUp { .. } => Color::Yellow,
            NotificationType::Achievement { .. } => Color::Magenta,
            NotificationType::QuestComplete { .. } => Color::Green,
            NotificationType::StreakMilestone { .. } => Color::Cyan,
            NotificationType::StreakFreezeGranted => Color::Cyan,
            NotificationType::StreakFreezeUsed => Color::Cyan,
            NotificationType::Info { .. } => Color::Blue,
            NotificationType::ReviewSessionComplete { .. } => Color::Green,
            NotificationType::MasteryLevelUp { .. } => Color::Yellow,
        }
    }
}

/// Queue for managing notifications
#[derive(Debug, Clone)]
pub struct NotificationQueue {
    /// Active notifications
    notifications: Vec<Notification>,

    /// Maximum number of visible notifications
    max_visible: usize,
}

impl NotificationQueue {
    /// Create a new notification queue
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
        Self {
            notifications: Vec::new(),
            max_visible: MAX_VISIBLE_NOTIFICATIONS,
        }
    }

    /// Add a notification to the queue
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
        self.notifications.push(notification);
    }

    /// Remove expired notifications from the queue
    ///
    /// Returns the number of notifications removed.
    pub fn remove_expired(&mut self) -> usize {
        let before = self.notifications.len();
        self.notifications.retain(|n| !n.is_expired());
        before - self.notifications.len()
    }

    /// Get visible notifications (up to max_visible, oldest first)
    ///
    /// Returns a slice of the most recent notifications that should be displayed.
    pub fn visible(&self) -> &[Notification] {
        let start = self.notifications.len().saturating_sub(self.max_visible);
        &self.notifications[start..]
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

    #[test]
    fn test_notification_not_expired_immediately() {
        let notif = Notification::new(NotificationType::LevelUp { new_level: 5 });
        assert!(!notif.is_expired());
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
        let visible = queue.visible();
        assert_eq!(visible.len(), 3);

        // Check that we get the most recent 3
        if let NotificationType::LevelUp { new_level } = visible[0].notification_type {
            assert_eq!(new_level, 3);
        }
        if let NotificationType::LevelUp { new_level } = visible[2].notification_type {
            assert_eq!(new_level, 5);
        }
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
            "Miss a day without breaking your streak"
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
    fn test_notification_remaining_time() {
        let notif = Notification::new(NotificationType::LevelUp { new_level: 2 });
        let remaining = notif.remaining_time();
        // Should be close to 3 seconds (within 100ms)
        assert!(remaining >= Duration::from_millis(2900));
        assert!(remaining <= Duration::from_secs(3));
    }

    #[test]
    fn test_notification_queue_default() {
        let queue = NotificationQueue::default();
        assert_eq!(queue.count(), 0);
        assert!(queue.is_empty());
    }
}
