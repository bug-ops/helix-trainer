//! Scenario lifecycle message handlers
//!
//! Handles starting, completing, retrying, and abandoning scenarios

use crate::game::services::ScenarioCompletionService;
use crate::game::GameSession;
use crate::security::UserError;
use crate::ui::state::{
    AppState, HandlerContext, HandlerOutcome, MenuData, Message, ResultsData, TaskData,
    TypedScreen, XPBreakdown, update,
};

/// Handle StartScenario message
///
/// Initializes a new game session with the selected scenario
pub fn handle_start_scenario(
    ctx: &mut HandlerContext<'_>,
    index: usize,
) -> Result<HandlerOutcome, UserError> {
    if let Some(scenario) = ctx
        .game
        .scenario_collection
        .get_filtered_by_index(index)
        .cloned()
    {
        let session = GameSession::new(scenario)?;
        // Create TaskData with the new session
        let task_data = TaskData::new(session);

        // Update UI state
        ctx.ui.show_key_history = false;
        ctx.ui.completion_time = None;

        // Transition to Task screen with data
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Task(
            task_data,
        ))));
    }
    Ok(HandlerOutcome::Stay)
}


/// Collect quest bonuses for newly completed quests
///
/// Returns list of (description, xp) pairs for quests completed during this scenario
fn collect_quest_bonuses(ctx: &mut HandlerContext<'_>) -> Vec<(String, u64)> {
    let newly_completed_quest_ids: Vec<String> = {
        let profile = &ctx.progress.profile;
        profile
            .daily_quests
            .iter()
            .filter(|q| q.completed && !ctx.progress.previously_completed_quests.contains(&q.id))
            .map(|q| q.id.clone())
            .collect()
    };

    let mut quest_bonuses = Vec::new();
    for quest_id in newly_completed_quest_ids {
        let profile = &ctx.progress.profile;
        if let Some(quest) = profile.daily_quests.iter().find(|q| q.id == quest_id) {
            let description = super::format_quest_description(&quest.quest_type);
            let xp = quest.xp_reward as u64;
            quest_bonuses.push((description, xp));
            ctx.progress.previously_completed_quests.insert(quest_id);
        }
    }
    quest_bonuses
}

fn record_scenario_completion(
    ctx: &mut HandlerContext<'_>,
    feedback: &crate::game::Feedback,
    total_xp: u64,
) -> Result<(), UserError> {
    let is_perfect = feedback.score == feedback.max_points;

    let leveled_up = ctx.progress.profile.add_xp(total_xp);
    ScenarioCompletionService::update_profile_counters(&mut ctx.progress.profile, is_perfect);

    if leveled_up {
        ctx.progress
            .storage
            .save(&ctx.progress.profile)
            .map_err(UserError::from)?;
        ctx.progress.mark_saved();
    }

    ctx.progress.scenarios_completed_today += 1;

    if ctx.progress.should_save() {
        ctx.progress
            .storage
            .save(&ctx.progress.profile)
            .map_err(UserError::from)?;
        ctx.progress.mark_saved();
    }

    let commands = ScenarioCompletionService::extract_commands(feedback);
    ScenarioCompletionService::record_fsrs_data(
        &mut ctx.progress.scheduler,
        &mut ctx.progress.performance_tracker,
        &commands,
        feedback.duration,
        feedback.score > 0,
    );

    Ok(())
}

/// Handle CompleteScenario message
///
/// Processes scenario completion: awards XP, updates quests, records FSRS data
///
/// Note: This handler needs full AppState access to call update() for quest progress
pub fn handle_complete_scenario(state: &mut AppState) -> Result<HandlerOutcome, UserError> {
    // Get feedback and completed session from state
    let (feedback, completed_session) = if let Some(ref feedback) = state.ui.last_feedback {
        // Extract completed session from pending storage
        let session = state.game.pending_completed_session.take();
        (feedback.clone(), session)
    } else {
        let results_data = if let Some(session) = state.game.pending_completed_session.take() {
            let feedback = session.feedback().map_err(UserError::from)?;
            ResultsData::from_completed(session, feedback).map_err(UserError::from)?
        } else {
            // No session either, just go to menu
            return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
                MenuData::default(),
            ))));
        };
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Results(
            results_data,
        ))));
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
            scenario_id: Some(scenario_id.clone()),
            duration,
        },
    )?;

    // Create context for helper functions
    let mut ctx = HandlerContext::new(
        &mut state.ui,
        &mut state.game,
        &mut state.progress,
        &state.config,
    );

    let is_first_today = ctx.progress.scenarios_completed_today == 0;
    let xp = ScenarioCompletionService::calculate_xp_components(&feedback, is_first_today);

    let (actual_xp, mastery_level, mastery_multiplier) =
        ScenarioCompletionService::record_and_scale_xp(
            &mut ctx.progress.profile,
            &scenario_id,
            feedback.score,
            xp.total_base_xp,
        );

    // Store mastery info for results display
    ctx.ui.scenario_mastery = Some((mastery_level, mastery_multiplier));

    // Collect quest bonuses
    let quest_bonuses = collect_quest_bonuses(&mut ctx);
    let quest_xp = quest_bonuses.iter().map(|(_, xp)| xp).sum::<u64>();
    let total_xp = actual_xp + quest_xp;

    ctx.ui.xp_breakdown = Some(XPBreakdown {
        base_xp: xp.base_xp,
        perfect_bonus: xp.perfect_bonus,
        first_today_bonus: xp.first_today_bonus,
        mastery_multiplier,
        quest_bonuses,
        total_xp,
    });

    // Record completion and award XP
    record_scenario_completion(&mut ctx, &feedback, total_xp)?;

    // Create ResultsData with completed session and transition to Results screen
    let Some(session) = completed_session else {
        // No completed session available - go back to menu
        // This shouldn't happen in normal flow, but handle gracefully
        ctx.ui.clear_temp_results();
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
            MenuData::default(),
        ))));
    };

    let mut results_data =
        ResultsData::from_completed(session, feedback.clone()).map_err(UserError::from)?;
    // Populate with XP breakdown and quest changes
    results_data.xp_breakdown = ctx.ui.xp_breakdown.clone();
    results_data.quest_changes = ctx.ui.quest_progress_changes.clone();
    results_data.scenario_mastery = ctx.ui.scenario_mastery;

    ctx.ui.clear_temp_results();
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Results(
        results_data,
    ))))
}

/// Handle AbandonScenario message
///
/// Marks the session as abandoned and shows results
///
/// Takes ownership of TaskData to consume the session
pub fn handle_abandon_scenario(task_data: TaskData) -> Result<TypedScreen, UserError> {
    // Take ownership and transition to abandoned state
    let abandoned = task_data.session.abandon();
    let feedback = abandoned.feedback();

    // Create ResultsData from abandoned session
    let results_data = ResultsData::from_abandoned(abandoned, feedback);

    // Return new screen directly
    Ok(TypedScreen::Results(results_data))
}

/// Handle RetryScenario message
///
/// Resets the current scenario to initial state
///
/// Takes ownership of ResultsData to extract scenario
pub fn handle_retry_scenario(
    results_data: ResultsData,
    ctx: &mut HandlerContext<'_>,
) -> Result<TypedScreen, UserError> {
    // Extract scenario from the session
    let scenario = match results_data.session {
        crate::ui::state::CompletedOrAbandoned::Completed(session) => session.scenario().clone(),
        crate::ui::state::CompletedOrAbandoned::Abandoned(session) => session.scenario().clone(),
    };

    // Create new session with same scenario
    let new_session = GameSession::new(scenario)?;
    let task_data = TaskData::new(new_session);

    // Reset UI state
    ctx.ui.show_key_history = false;
    ctx.ui.completion_time = None;

    Ok(TypedScreen::Task(task_data))
}

/// Handle NextScenario message
///
/// Completes the current scenario flow and returns to menu
pub fn handle_next_scenario(_ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    // Preserve menu state if possible
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData::default(),
    ))))
}

