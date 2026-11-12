//! Key-history bookkeeping shared by the Task and Arcade screens
//!
//! Both screens record every physically-typed key before it is resolved, so
//! the popup shows what the player pressed, including keys that only extend a
//! pending prefix. The history is display-only: it never feeds a card, a score
//! or a quest.

use crate::game::format_key_for_display;
use crate::ui::state::{KeyHistory, UIState};

/// Record the physically-typed key `typed` and make the key-history popup visible.
///
/// Called before the key is resolved, whatever the resolution turns out to be.
pub(super) fn record_key(ui: &mut UIState, history: &mut KeyHistory, typed: &str) {
    history.push(format_key_for_display(typed));
    ui.show_key_history = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::empty_test_app_state;

    #[test]
    fn records_the_formatted_key_newest_first_and_shows_the_popup() {
        let mut state = empty_test_app_state();
        let mut history = KeyHistory::new();
        state.ui.show_key_history = false;

        record_key(&mut state.ui, &mut history, "g");
        record_key(&mut state.ui, &mut history, "Left");

        assert_eq!(*history.keys(), ["←", "g"]);
        assert!(state.ui.show_key_history);
    }
}
