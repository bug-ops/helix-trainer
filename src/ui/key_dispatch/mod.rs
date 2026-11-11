//! Key-event dispatch
//!
//! Routes a crossterm [`KeyEvent`] to the screen handler of the current
//! [`TypedScreen`](crate::ui::state::TypedScreen) and turns it into a [`Message`]. The key-to-command
//! resolution itself lives in [`crate::input`]; this layer sits above it
//! because it reads [`AppState`] and produces UI messages.

mod ctrl_c;
pub mod handlers;

use crossterm::event::KeyEvent;

use crate::ui::{AppState, Message};

use self::handlers::{
    handle_category_filters_keys, handle_end_game_summary_keys, handle_menu_keys,
    handle_minigame_keys, handle_mode_selection_keys, handle_profile_stats_keys,
    handle_results_keys, handle_review_keys, handle_task_keys,
};

/// Handle keyboard events and convert them to messages
///
/// This function is the main entry point for input handling.
/// It dispatches to screen-specific handlers based on current screen state.
///
/// Note: Some handlers (like menu) need mutable access to state
/// for command buffer management.
pub fn handle_key_event(key: KeyEvent, state: &AppState) -> Option<Message> {
    use crate::ui::state::TypedScreen;
    use crossterm::event::{KeyCode, KeyModifiers};

    // Banner actions (#518, #531): offered only while the matching quarantine state holds
    if key.modifiers == KeyModifiers::CONTROL
        && let Some(quarantine) = state.progress.persistence().quarantine()
    {
        match key.code {
            KeyCode::Char('n') if quarantine.can_start_fresh() => {
                return Some(Message::StartFreshProfile);
            }
            // Ctrl-R is redo in gameplay, so the banner action leaves those screens alone
            KeyCode::Char('r')
                if quarantine.can_restore_previous()
                    && !matches!(
                        state.screen,
                        TypedScreen::Task(_) | TypedScreen::MiniGame(_)
                    ) =>
            {
                return Some(Message::RestoreProfile);
            }
            _ => {}
        }
    }

    if ctrl_c::is_ctrl_c(key) {
        return ctrl_c::message(key, state);
    }

    match &state.screen {
        TypedScreen::ModeSelection(_) => handle_mode_selection_keys(key),
        TypedScreen::Menu(_) => handle_menu_keys(key, state),
        TypedScreen::Task(_) => handle_task_keys(key, state),
        TypedScreen::Results(_) => handle_results_keys(key),
        TypedScreen::Profile(_) | TypedScreen::Statistics(_) | TypedScreen::Achievements(_) => {
            handle_profile_stats_keys(key, state)
        }
        TypedScreen::Review(_) => handle_review_keys(key),
        TypedScreen::MiniGame(_) => handle_minigame_keys(key, state),
        TypedScreen::CategoryFilters(_) => handle_category_filters_keys(key),
        TypedScreen::EndGameSummary(_) => handle_end_game_summary_keys(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gamification::{PreviousCopy, QuarantineOutcome};
    use crate::ui::state::Quarantine;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn ctrl_n() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)
    }

    /// #518: the start-fresh key only exists while a backed-up profile is quarantined
    #[test]
    fn test_ctrl_n_starts_a_fresh_profile_only_after_a_backup() {
        let mut state = crate::testing::empty_test_app_state();
        assert_ne!(
            handle_key_event(ctrl_n(), &state),
            Some(Message::StartFreshProfile)
        );

        state
            .progress
            .quarantine_profile(crate::testing::quarantine(crate::testing::failed_backup()));
        assert_ne!(
            handle_key_event(ctrl_n(), &state),
            Some(Message::StartFreshProfile)
        );

        state
            .progress
            .quarantine_profile(crate::testing::quarantine(QuarantineOutcome::BackedUp(
                "backup".into(),
            )));
        assert_eq!(
            handle_key_event(ctrl_n(), &state),
            Some(Message::StartFreshProfile)
        );
    }

    fn ctrl_r() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
    }

    fn quarantine_with(backup: QuarantineOutcome, previous: PreviousCopy) -> Quarantine {
        Quarantine { backup, previous }
    }

    /// #531: Ctrl-R restores only with a backup on disk and a loadable previous copy
    #[test]
    fn test_ctrl_r_restores_only_with_a_backup_and_an_available_previous_copy() {
        let mut state = crate::testing::empty_test_app_state();
        assert_ne!(
            handle_key_event(ctrl_r(), &state),
            Some(Message::RestoreProfile)
        );

        for (backup, previous, offered) in [
            (
                QuarantineOutcome::BackedUp("backup".into()),
                PreviousCopy::Unavailable,
                false,
            ),
            (
                crate::testing::failed_backup(),
                PreviousCopy::Available,
                false,
            ),
            (
                QuarantineOutcome::BackedUp("backup".into()),
                PreviousCopy::Available,
                true,
            ),
        ] {
            state
                .progress
                .quarantine_profile(quarantine_with(backup, previous));
            assert_eq!(
                handle_key_event(ctrl_r(), &state) == Some(Message::RestoreProfile),
                offered
            );
        }
    }

    /// Ctrl-R is redo on the task screen: the banner action must not steal it
    #[test]
    fn test_ctrl_r_stays_redo_on_the_task_screen_while_quarantined() {
        let mut state = crate::testing::empty_test_app_state();
        state.progress.quarantine_profile(quarantine_with(
            QuarantineOutcome::BackedUp("backup".into()),
            PreviousCopy::Available,
        ));
        state.screen = crate::ui::state::TypedScreen::Task(crate::ui::state::TaskData::new(
            crate::game::GameSession::new(crate::testing::default_test_scenario()).unwrap(),
        ));

        assert_ne!(
            handle_key_event(ctrl_r(), &state),
            Some(Message::RestoreProfile)
        );
    }
}
