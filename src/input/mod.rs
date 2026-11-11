//! User input handling
//!
//! Converts keyboard events into application messages based on current screen state.
//!
//! # Architecture
//!
//! Input handling is split into two layers:
//!
//! 1. **Screen handlers** ([`handlers`]) - Route input to the correct screen
//! 2. **Typestate handlers** ([`typestate`]) - Handle key-to-command mapping with state machine
//!
//! # Typestate-Based Input Handling
//!
//! The [`typestate`] module provides compile-time guarantees about input state
//! transitions. It encodes the current input context (base state, waiting for
//! character argument, building count prefix, etc.) at the type level.
//!
//! See [`typestate`] for the state machine implementation.

pub mod handlers;
pub mod keymap;
pub mod typestate;

use crossterm::event::KeyEvent;

use crate::ui::{AppState, Message};

use handlers::*;

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

    // Banner action (#518): offered only while a backed-up profile is quarantined
    if key.code == KeyCode::Char('n')
        && key.modifiers == KeyModifiers::CONTROL
        && matches!(
            state.progress.persistence(),
            crate::ui::state::ProfilePersistence::Quarantined(crate::ui::state::Quarantine {
                backup: crate::gamification::QuarantineOutcome::BackedUp(_),
                ..
            })
        )
    {
        return Some(Message::StartFreshProfile);
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
    use crate::gamification::QuarantineOutcome;
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
}
