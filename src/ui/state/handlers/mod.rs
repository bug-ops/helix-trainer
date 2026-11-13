//! Message handlers for the Elm Architecture update function
//!
//! This module contains all message handlers, organized by functional domain.
//! Each handler is a pure function that modifies `AppState` based on a message.

// Re-export all handlers for use in the main update function
pub(super) use category_filters::{
    handle_category_filter_down, handle_category_filter_select_all, handle_category_filter_toggle,
    handle_category_filter_up, handle_show_category_filters,
};
pub(super) use gameplay::{handle_execute_command, handle_show_hint};
pub(super) use menu::{handle_menu_down, handle_menu_up};
pub(super) use minigame::{
    handle_minigame_back_to_menu, handle_minigame_command, handle_minigame_game_over,
    handle_minigame_next_scenario, handle_minigame_session_timeout, handle_minigame_tick,
    handle_minigame_timeout, handle_pause_minigame, handle_resume_minigame,
};
pub(super) use mode_selection::{
    handle_mode_selection_back, handle_mode_selection_down, handle_mode_selection_select,
    handle_mode_selection_up, handle_select_arcade_mode,
};
pub(super) use navigation::{handle_back_to_menu, handle_navigate_to, handle_quit_app};
pub(super) use notifications::handle_cleanup_notifications;
pub(super) use profile::{
    handle_scroll_achievements, handle_show_achievements, handle_show_profile,
    handle_show_statistics,
};
pub(super) use quests::{
    apply_quest_event, format_quest_description, handle_update_quest_progress,
};
pub(super) use review::{
    handle_abandon_review_session, handle_complete_review_command, handle_start_review_session,
};
pub(super) use scenario::{
    handle_abandon_scenario, handle_complete_scenario, handle_go_to_scenario_list,
    handle_next_lesson, handle_retry_scenario,
};

/// What a resolved key feeds the active session: typed Insert-mode text or a
/// parsed Normal-mode command.
#[derive(Clone, Copy)]
pub(super) enum SessionInput<'a> {
    /// Text typed (or an Insert-mode key name) in Insert mode
    Text(&'a str),
    /// A parsed Normal-mode command, count included
    Command(&'a crate::helix::normal_command::NormalCommand),
}

mod category_filters;
mod gameplay;
mod key_history;
pub(super) mod menu;
mod minigame;
mod mode_selection;
mod navigation;
mod notifications;
mod profile;
mod quests;
mod review;
mod scenario;
