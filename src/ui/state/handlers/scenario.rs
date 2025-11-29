//! Scenario lifecycle message handlers
//!
//! Handles starting, completing, retrying, and abandoning scenarios

use crate::game::GameSession;
use crate::security::UserError;
use crate::ui::state::{AppState, Message, Screen, XPBreakdown, update};

/// Handle StartScenario message
///
/// Initializes a new game session with the selected scenario
pub fn handle_start_scenario(state: &mut AppState, index: usize) -> Result<(), UserError> {
    if let Some(scenario) = state
        .game
        .scenario_collection
        .get_filtered_by_index(index)
        .cloned()
    {
        let session = GameSession::new(scenario)?;
        state.game.session = Some(session);
        state.ui.screen = Screen::Task;
        state.ui.show_hint_panel = false;
        state.ui.show_key_history = false;
        state.ui.current_hint = None;
        state.ui.last_command = None;
        state.ui.completion_time = None;
        state.clear_key_history();
        state.ui.command_buffer.clear();
    }
    Ok(())
}

/// Handle CompleteScenario message
///
/// Processes scenario completion: awards XP, updates quests, records FSRS data
pub fn handle_complete_scenario(state: &mut AppState) -> Result<(), UserError> {
    // Update quest progress BEFORE awarding XP
    // Extract data we need first to avoid borrow issues
    let (duration, feedback, scenario_id) = if let Some(session) = &state.game.session {
        let duration = session.elapsed();
        let feedback = session
            .get_feedback()
            .map_err(|_| UserError::OperationFailed)?;
        let scenario_id = session.scenario().id.clone();
        (duration, feedback, scenario_id)
    } else {
        state.ui.screen = Screen::Results;
        return Ok(());
    };

    // Update quest progress
    update(
        state,
        Message::UpdateQuestProgress {
            command: None,
            scenario_completed: true,
            duration,
        },
    )?;

    // Calculate base XP (before mastery scaling)
    let score = feedback.score;
    let is_perfect = feedback.score == feedback.max_points;
    let is_first_today = state.progress.scenarios_completed_today == 0;

    // Base XP from score (50 XP per 100 points)
    let base_xp = (score as u64 * 50) / 100;

    // Perfect bonus (+20%)
    let perfect_bonus = if is_perfect { base_xp / 5 } else { 0 };

    // First today bonus (+10 XP)
    let first_today_bonus = if is_first_today { 10 } else { 0 };

    let total_base_xp = base_xp + perfect_bonus + first_today_bonus;

    // Apply mastery scaling and record completion
    let (actual_xp, mastery_level, mastery_multiplier) = {
        let mut profile = state.progress.profile.borrow_mut();
        let actual_xp =
            profile
                .scenario_history
                .record_completion(&scenario_id, score, total_base_xp);

        // Get mastery info for UI display
        let completion = profile.scenario_history.get(&scenario_id).unwrap();
        let mastery_level = completion.mastery_level;
        let mastery_multiplier = completion.xp_multiplier();

        (actual_xp, mastery_level, mastery_multiplier)
    };

    // Store mastery info for results display
    state.ui.scenario_mastery = Some((mastery_level, mastery_multiplier));

    // Quest bonuses (collect newly completed quests)
    let mut quest_bonuses = Vec::new();
    let newly_completed_quest_ids: Vec<String> = {
        let profile = state.progress.profile.borrow();
        profile
            .daily_quests
            .iter()
            .filter(|q| q.completed && !state.progress.previously_completed_quests.contains(&q.id))
            .map(|q| q.id.clone())
            .collect()
    };

    // Collect bonuses and mark as processed
    for quest_id in newly_completed_quest_ids {
        let profile = state.progress.profile.borrow();
        if let Some(quest) = profile.daily_quests.iter().find(|q| q.id == quest_id) {
            let description = super::format_quest_description(&quest.quest_type);
            let xp = quest.xp_reward as u64;
            drop(profile);
            quest_bonuses.push((description, xp));
            state.progress.previously_completed_quests.insert(quest_id);
        }
    }

    let quest_xp = quest_bonuses.iter().map(|(_, xp)| xp).sum::<u64>();
    let total_xp = actual_xp + quest_xp;

    // Store breakdown for results display
    state.ui.xp_breakdown = Some(XPBreakdown {
        base_xp,
        perfect_bonus,
        first_today_bonus,
        mastery_multiplier,
        quest_bonuses,
        total_xp,
    });

    // Award XP to profile
    {
        let mut profile = state.progress.profile.borrow_mut();
        let leveled_up = profile.add_xp(total_xp);

        // Update counters
        profile.scenarios_completed += 1;
        if is_perfect {
            profile.perfect_scenarios += 1;
        }

        if leveled_up {
            drop(profile);
            state
                .save_profile_immediate()
                .map_err(|_| UserError::OperationFailed)?;
        }
    }

    state.progress.scenarios_completed_today += 1;
    state
        .save_profile_debounced()
        .map_err(|_| UserError::OperationFailed)?;

    // Record commands in FSRS scheduler for spaced repetition
    if let Some(session) = &state.game.session {
        let commands: Vec<String> = session
            .actions()
            .iter()
            .map(|action| action.command.clone())
            .collect();

        state
            .progress
            .scheduler
            .record_scenario_commands(&commands, duration, score > 0);
    }

    state.ui.screen = Screen::Results;
    Ok(())
}

/// Handle AbandonScenario message
///
/// Marks the session as abandoned and shows results
pub fn handle_abandon_scenario(state: &mut AppState) -> Result<(), UserError> {
    if let Some(session) = &mut state.game.session {
        session.abandon();
    }
    state.ui.screen = Screen::Results;
    Ok(())
}

/// Handle RetryScenario message
///
/// Resets the current scenario to initial state
pub fn handle_retry_scenario(state: &mut AppState) -> Result<(), UserError> {
    if let Some(session) = &mut state.game.session {
        session.reset()?;
        state.ui.screen = Screen::Task;
        state.ui.show_hint_panel = false;
        state.ui.show_key_history = false;
        state.ui.current_hint = None;
        state.ui.last_command = None;
        state.ui.completion_time = None;
        state.clear_key_history();
        state.ui.command_buffer.clear();
    }
    Ok(())
}

/// Handle NextScenario message
///
/// Completes the current scenario flow and returns to menu
pub fn handle_next_scenario(state: &mut AppState) -> Result<(), UserError> {
    state.ui.screen = Screen::MainMenu;
    state.game.session = None;
    state.ui.show_hint_panel = false;
    state.ui.current_hint = None;
    Ok(())
}
