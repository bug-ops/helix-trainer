//! Scenario lifecycle message handlers
//!
//! Handles starting, completing, retrying, and abandoning scenarios

use crate::game::GameSession;
use crate::security::UserError;
use crate::ui::state::{
    AppState, MenuData, Message, ResultsData, TaskData, TypedScreen, XPBreakdown, update,
};

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
        // Create TaskData with the new session
        let task_data = TaskData::new(session);

        // Transition to Task screen with data
        state.screen = TypedScreen::Task(task_data);
        state.ui.show_key_history = false;
        state.ui.completion_time = None;

        // Clear old session from game state
        state.game.session = None;
    }
    Ok(())
}

/// Handle CompleteScenario message
///
/// Processes scenario completion: awards XP, updates quests, records FSRS data
pub fn handle_complete_scenario(state: &mut AppState) -> Result<(), UserError> {
    // Get feedback and completed session from temporary storage
    let (feedback, completed_session) = if let Some(ref feedback) = state.ui.last_feedback {
        // Extract completed session from pending storage
        let session = state.game.pending_completed_session.take();
        (feedback.clone(), session)
    } else {
        // No feedback available, transition to results anyway with placeholder
        let results_data = if let Some(session) = state.game.pending_completed_session.take() {
            let feedback = session.feedback().map_err(|_| UserError::OperationFailed)?;
            ResultsData::from_completed(session, feedback)
                .map_err(|_| UserError::OperationFailed)?
        } else {
            // No session either, just go to menu
            state.screen = TypedScreen::Menu(MenuData::default());
            return Ok(());
        };
        state.screen = TypedScreen::Results(results_data);
        return Ok(());
    };

    // Extract data from feedback
    let scenario_id = feedback.scenario_id.clone();
    let duration = feedback.duration;

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

    // Record commands in FSRS scheduler for spaced repetition (from feedback)
    let commands: Vec<String> = feedback
        .user_actions
        .iter()
        .map(|action| action.command.clone())
        .collect();

    state
        .progress
        .scheduler
        .record_scenario_commands(&commands, duration, feedback.score > 0);

    // Create ResultsData with completed session and transition to Results screen
    let Some(session) = completed_session else {
        // No completed session available - go back to menu
        // This shouldn't happen in normal flow, but handle gracefully
        state.screen = TypedScreen::Menu(MenuData::default());
        state.ui.clear_temp_results();
        return Ok(());
    };

    let mut results_data = ResultsData::from_completed(session, feedback.clone())
        .map_err(|_| UserError::OperationFailed)?;
    // Populate with XP breakdown and quest changes
    results_data.xp_breakdown = state.ui.xp_breakdown.clone();
    results_data.quest_changes = state.ui.quest_progress_changes.clone();
    results_data.scenario_mastery = state.ui.scenario_mastery;

    state.screen = TypedScreen::Results(results_data);
    state.ui.clear_temp_results();
    Ok(())
}

/// Handle AbandonScenario message
///
/// Marks the session as abandoned and shows results
pub fn handle_abandon_scenario(state: &mut AppState) -> Result<(), UserError> {
    // Extract session from Task screen
    if let TypedScreen::Task(task_data) = std::mem::replace(
        &mut state.screen,
        TypedScreen::Menu(MenuData::default()), // Temporary placeholder
    ) {
        // Take ownership and transition to abandoned state
        let abandoned = task_data.session.abandon();
        let feedback = abandoned.feedback();

        // Create ResultsData from abandoned session
        let results_data = ResultsData::from_abandoned(abandoned, feedback);

        // Transition to Results screen
        state.screen = TypedScreen::Results(results_data);
    }
    Ok(())
}

/// Handle RetryScenario message
///
/// Resets the current scenario to initial state
pub fn handle_retry_scenario(state: &mut AppState) -> Result<(), UserError> {
    // Get scenario from results screen and create new session
    if let TypedScreen::Results(results_data) =
        std::mem::replace(&mut state.screen, TypedScreen::Menu(MenuData::default()))
    {
        // Extract scenario from the session
        let scenario = match results_data.session {
            crate::ui::state::CompletedOrAbandoned::Completed(session) => {
                session.scenario().clone()
            }
            crate::ui::state::CompletedOrAbandoned::Abandoned(session) => {
                session.scenario().clone()
            }
        };

        // Create new session with same scenario
        let new_session = GameSession::new(scenario)?;
        let task_data = TaskData::new(new_session);

        state.screen = TypedScreen::Task(task_data);
        state.ui.show_key_history = false;
        state.ui.completion_time = None;
    }
    Ok(())
}

/// Handle NextScenario message
///
/// Completes the current scenario flow and returns to menu
pub fn handle_next_scenario(state: &mut AppState) -> Result<(), UserError> {
    // Preserve menu state if possible
    state.screen = TypedScreen::Menu(MenuData::default());
    state.game.session = None;
    Ok(())
}
