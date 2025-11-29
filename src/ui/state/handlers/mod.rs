//! Message handlers for the Elm Architecture update function
//!
//! This module contains all message handlers, organized by functional domain.
//! Each handler is a pure function that modifies AppState based on a message.

// Re-export all handlers for use in the main update function
pub(super) use filters::{
    handle_reset_filters, handle_set_sort_mode, handle_toggle_category_filter,
    handle_toggle_completed_filter, handle_toggle_difficulty_filter,
};
pub(super) use gameplay::{handle_execute_command, handle_show_hint};
pub(super) use menu::{handle_menu_down, handle_menu_select, handle_menu_up};
pub(super) use navigation::{handle_back_to_menu, handle_navigate_to, handle_quit_app};
pub(super) use profile::{handle_award_xp, handle_show_profile, handle_show_statistics};
pub(super) use quests::{format_quest_description, handle_update_quest_progress};
pub(super) use review::{
    handle_abandon_review_session, handle_complete_review_command, handle_next_review_command,
    handle_start_review_session,
};
pub(super) use scenario::{
    handle_abandon_scenario, handle_complete_scenario, handle_next_scenario, handle_retry_scenario,
    handle_start_scenario,
};

mod filters;
mod gameplay;
mod menu;
mod navigation;
mod profile;
mod quests;
mod review;
mod scenario;
