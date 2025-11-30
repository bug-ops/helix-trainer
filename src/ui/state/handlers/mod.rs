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
pub(super) use minigame::{
    handle_minigame_back_to_menu, handle_minigame_command, handle_minigame_next_scenario,
    handle_minigame_scenario_complete, handle_minigame_tick, handle_minigame_timeout,
    handle_pause_minigame, handle_resume_minigame, handle_start_minigame,
};
pub(super) use mode_selection::{
    handle_mode_selection_down, handle_mode_selection_select, handle_mode_selection_up,
    handle_select_arcade_mode, handle_select_training_mode,
};
pub(super) use navigation::{handle_back_to_menu, handle_navigate_to, handle_quit_app};
pub(super) use profile::{handle_award_xp, handle_show_profile, handle_show_statistics};
pub(super) use quests::{format_quest_description, handle_update_quest_progress};
// Shared quest tracking functions for both training and arcade modes
pub(super) use quests::{
    award_quest_completion_xp, snapshot_quest_completion, track_command_for_quests,
    track_scenario_completion_for_quests,
};
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
mod minigame;
mod mode_selection;
mod navigation;
mod profile;
mod quests;
mod review;
mod scenario;
