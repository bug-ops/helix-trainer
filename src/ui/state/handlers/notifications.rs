//! Notification message handlers

use crate::ui::state::handler_context::HandlerOutcome;

/// Handle `CleanupNotifications` message
///
/// Removes expired notifications from the queue
pub(in crate::ui::state) fn handle_cleanup_notifications(
    ui: &mut crate::ui::state::UIState,
) -> HandlerOutcome {
    ui.notifications.remove_expired();
    HandlerOutcome::Stay
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::notification::Notification;
    use crate::ui::notification::NotificationType;
    use crate::ui::state::UIState;

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
        handle_cleanup_notifications(&mut ui);
        assert_eq!(ui.notifications.count(), 1);
    }
}
