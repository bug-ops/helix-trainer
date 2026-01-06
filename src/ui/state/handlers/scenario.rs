//! Scenario lifecycle message handlers
//!
//! Handles starting, completing, retrying, and abandoning scenarios

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

        // Clear old session from game state
        ctx.game.session = None;

        // Transition to Task screen with data
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Task(
            task_data,
        ))));
    }
    Ok(HandlerOutcome::Stay)
}

/// Calculate XP breakdown from scenario feedback
///
/// Pure function that computes base XP, bonuses, and total XP
fn calculate_xp_breakdown(
    feedback: &crate::game::Feedback,
    is_first_today: bool,
) -> (u64, u64, u64, u64) {
    let score = feedback.score;
    let is_perfect = feedback.score == feedback.max_points;

    // Base XP from score (50 XP per 100 points)
    let base_xp = (score as u64 * 50) / 100;

    // Perfect bonus (+20%)
    let perfect_bonus = if is_perfect { base_xp / 5 } else { 0 };

    // First today bonus (+10 XP)
    let first_today_bonus = if is_first_today { 10 } else { 0 };

    let total_base_xp = base_xp + perfect_bonus + first_today_bonus;

    (base_xp, perfect_bonus, first_today_bonus, total_base_xp)
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

/// Record scenario completion and award XP
///
/// Updates profile with XP, counters, and saves to disk
fn record_scenario_completion(
    ctx: &mut HandlerContext<'_>,
    feedback: &crate::game::Feedback,
    total_xp: u64,
) -> Result<(), UserError> {
    let is_perfect = feedback.score == feedback.max_points;

    // Award XP to profile
    let profile = &mut ctx.progress.profile;
    let leveled_up = profile.add_xp(total_xp);

    // Update counters
    profile.scenarios_completed += 1;
    if is_perfect {
        profile.perfect_scenarios += 1;
    }

    // Save profile if leveled up
    if leveled_up {
        ctx.progress
            .storage
            .save(&ctx.progress.profile)
            .map_err(|_| UserError::OperationFailed)?;
        ctx.progress.mark_saved();
    }

    ctx.progress.scenarios_completed_today += 1;

    // Debounced save
    if ctx.progress.should_save() {
        ctx.progress
            .storage
            .save(&ctx.progress.profile)
            .map_err(|_| UserError::OperationFailed)?;
        ctx.progress.mark_saved();
    }

    // Record commands in FSRS scheduler for spaced repetition
    let commands: Vec<String> = feedback
        .user_actions
        .iter()
        .map(|action| action.command.clone())
        .collect();

    ctx.progress.scheduler.record_scenario_commands(
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
        // No feedback available, transition to results anyway with placeholder
        let results_data = if let Some(session) = state.game.pending_completed_session.take() {
            let feedback = session.feedback().map_err(|_| UserError::OperationFailed)?;
            ResultsData::from_completed(session, feedback)
                .map_err(|_| UserError::OperationFailed)?
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

    // Calculate base XP components
    let is_first_today = ctx.progress.scenarios_completed_today == 0;
    let (base_xp, perfect_bonus, first_today_bonus, total_base_xp) =
        calculate_xp_breakdown(&feedback, is_first_today);

    // Apply mastery scaling and record completion
    let profile = &mut ctx.progress.profile;
    let actual_xp =
        profile
            .scenario_history
            .record_completion(&scenario_id, feedback.score, total_base_xp);

    // Get mastery info for UI display
    let completion = profile.scenario_history.get(&scenario_id).unwrap();
    let mastery_level = completion.mastery_level;
    let mastery_multiplier = completion.xp_multiplier();

    // Store mastery info for results display
    ctx.ui.scenario_mastery = Some((mastery_level, mastery_multiplier));

    // Collect quest bonuses
    let quest_bonuses = collect_quest_bonuses(&mut ctx);
    let quest_xp = quest_bonuses.iter().map(|(_, xp)| xp).sum::<u64>();
    let total_xp = actual_xp + quest_xp;

    // Store breakdown for results display
    ctx.ui.xp_breakdown = Some(XPBreakdown {
        base_xp,
        perfect_bonus,
        first_today_bonus,
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

    let mut results_data = ResultsData::from_completed(session, feedback.clone())
        .map_err(|_| UserError::OperationFailed)?;
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
pub fn handle_next_scenario(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    // Clear session
    ctx.game.session = None;

    // Preserve menu state if possible
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData::default(),
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Feedback;

    // Helper to create a minimal feedback struct for testing
    fn create_test_feedback(score: u32, max_points: u32) -> Feedback {
        Feedback {
            scenario_id: "test".to_string(),
            success: true,
            score,
            max_points,
            rating: crate::game::PerformanceRating::Perfect,
            actions_taken: 1,
            optimal_actions: 1,
            duration: std::time::Duration::from_secs(5),
            hint: None,
            is_optimal: true,
            user_actions: vec![],
        }
    }

    // Unit tests for calculate_xp_breakdown()
    mod calculate_xp_breakdown_tests {
        use super::*;

        #[test]
        fn test_calculate_xp_breakdown_base_only() {
            // Score 100, not first today, not perfect
            let feedback = create_test_feedback(100, 150);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, false);

            assert_eq!(base_xp, 50); // 100 * 50 / 100 = 50
            assert_eq!(perfect_bonus, 0); // Not perfect
            assert_eq!(first_today_bonus, 0); // Not first today
            assert_eq!(total, 50);
        }

        #[test]
        fn test_calculate_xp_breakdown_perfect_bonus() {
            // Perfect score (100/100)
            let feedback = create_test_feedback(100, 100);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, false);

            assert_eq!(base_xp, 50); // 100 * 50 / 100 = 50
            assert_eq!(perfect_bonus, 10); // 50 / 5 = 10 (20% bonus)
            assert_eq!(first_today_bonus, 0);
            assert_eq!(total, 60); // 50 + 10
        }

        #[test]
        fn test_calculate_xp_breakdown_first_today_bonus() {
            // First scenario today
            let feedback = create_test_feedback(100, 150);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, true);

            assert_eq!(base_xp, 50);
            assert_eq!(perfect_bonus, 0);
            assert_eq!(first_today_bonus, 10); // Fixed +10 XP
            assert_eq!(total, 60); // 50 + 10
        }

        #[test]
        fn test_calculate_xp_breakdown_all_bonuses() {
            // Perfect score AND first today
            let feedback = create_test_feedback(100, 100);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, true);

            assert_eq!(base_xp, 50);
            assert_eq!(perfect_bonus, 10); // 20% bonus
            assert_eq!(first_today_bonus, 10); // +10 XP
            assert_eq!(total, 70); // 50 + 10 + 10
        }

        #[test]
        fn test_calculate_xp_breakdown_zero_score() {
            // Zero score edge case
            let feedback = create_test_feedback(0, 100);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, false);

            assert_eq!(base_xp, 0);
            assert_eq!(perfect_bonus, 0);
            assert_eq!(first_today_bonus, 0);
            assert_eq!(total, 0);
        }

        #[test]
        fn test_calculate_xp_breakdown_partial_score() {
            // 75/100 score
            let feedback = create_test_feedback(75, 100);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, false);

            assert_eq!(base_xp, 37); // 75 * 50 / 100 = 37
            assert_eq!(perfect_bonus, 0); // Not perfect
            assert_eq!(first_today_bonus, 0);
            assert_eq!(total, 37);
        }

        #[test]
        fn test_calculate_xp_breakdown_high_score() {
            // High score scenario (200 points)
            let feedback = create_test_feedback(200, 200);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, true);

            assert_eq!(base_xp, 100); // 200 * 50 / 100 = 100
            assert_eq!(perfect_bonus, 20); // 100 / 5 = 20
            assert_eq!(first_today_bonus, 10);
            assert_eq!(total, 130); // 100 + 20 + 10
        }

        #[test]
        fn test_calculate_xp_breakdown_low_score() {
            // Very low score (10/100)
            let feedback = create_test_feedback(10, 100);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, false);

            assert_eq!(base_xp, 5); // 10 * 50 / 100 = 5
            assert_eq!(perfect_bonus, 0);
            assert_eq!(first_today_bonus, 0);
            assert_eq!(total, 5);
        }

        #[test]
        fn test_calculate_xp_breakdown_rounding() {
            // Test integer division rounding (e.g., 51 * 50 / 100 = 25)
            let feedback = create_test_feedback(51, 100);
            let (base_xp, perfect_bonus, first_today_bonus, total) =
                calculate_xp_breakdown(&feedback, false);

            assert_eq!(base_xp, 25); // 51 * 50 / 100 = 25 (truncated)
            assert_eq!(perfect_bonus, 0);
            assert_eq!(first_today_bonus, 0);
            assert_eq!(total, 25);
        }

        #[test]
        fn test_calculate_xp_breakdown_perfect_bonus_rounding() {
            // Perfect bonus should also be rounded down
            let feedback = create_test_feedback(100, 100);
            let (_, perfect_bonus, _, _) = calculate_xp_breakdown(&feedback, false);

            // base_xp = 50, perfect_bonus = 50 / 5 = 10
            assert_eq!(perfect_bonus, 10);
        }
    }
}
