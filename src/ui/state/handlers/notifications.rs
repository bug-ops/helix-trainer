//! Notification message handlers

use crate::security::UserError;
use crate::ui::notification::Notification;
use crate::ui::state::handler_context::HandlerOutcome;

/// Handle ShowNotification message
///
/// Adds a new notification to the queue
pub(in crate::ui::state) fn handle_show_notification(
    ui: &mut crate::ui::state::UIState,
    notification: Notification,
) -> Result<HandlerOutcome, UserError> {
    ui.notifications.push(notification);
    Ok(HandlerOutcome::Stay)
}

/// Handle CleanupNotifications message
///
/// Removes expired notifications from the queue
pub(in crate::ui::state) fn handle_cleanup_notifications(
    ui: &mut crate::ui::state::UIState,
) -> Result<HandlerOutcome, UserError> {
    ui.notifications.remove_expired();
    Ok(HandlerOutcome::Stay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::notification::NotificationType;
    use crate::ui::state::UIState;

    #[test]
    fn test_handle_show_notification() {
        let mut ui = UIState::new();
        assert_eq!(ui.notifications.count(), 0);

        let notification = Notification::new(NotificationType::LevelUp { new_level: 2 });
        handle_show_notification(&mut ui, notification).unwrap();

        assert_eq!(ui.notifications.count(), 1);
    }

    #[test]
    fn test_handle_cleanup_notifications() {
        let mut ui = UIState::new();

        // Add some notifications
        ui.notifications
            .push(Notification::new(NotificationType::LevelUp {
                new_level: 2,
            }));
        assert_eq!(ui.notifications.count(), 1);

        // Cleanup (should not remove fresh notifications)
        handle_cleanup_notifications(&mut ui).unwrap();
        assert_eq!(ui.notifications.count(), 1);
    }
}
