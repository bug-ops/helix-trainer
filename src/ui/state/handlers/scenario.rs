//! Scenario lifecycle message handlers
//!
//! Handles starting, completing, retrying, and abandoning scenarios

use crate::game::GameSession;
use crate::game::services::ScenarioCompletionService;
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
    // Extract scenario_index from current TaskData before any transitions
    let scenario_index = if let TypedScreen::Task(ref task_data) = state.screen {
        task_data.scenario_index
    } else {
        None
    };

    // Get feedback and completed session from state
    let (feedback, completed_session) = if let Some(ref feedback) = state.ui.last_feedback {
        // Extract completed session from pending storage
        let session = state.game.pending_completed_session.take();
        (feedback.clone(), session)
    } else {
        let results_data = if let Some(session) = state.game.pending_completed_session.take() {
            let feedback = session.feedback().map_err(UserError::from)?;
            ResultsData::from_completed(session, feedback, scenario_index)
                .map_err(UserError::from)?
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

    let mut results_data = ResultsData::from_completed(session, feedback.clone(), scenario_index)
        .map_err(UserError::from)?;
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

    // Create ResultsData from abandoned session, preserving scenario index
    let results_data = ResultsData::from_abandoned(abandoned, feedback, task_data.scenario_index);

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

/// Handle NextLesson message
///
/// Navigates to the next scenario in the filtered list.
/// If at the end of the list, stays on Results screen.
/// If no scenario_index is available, returns to Menu screen.
pub fn handle_next_lesson(
    results_data: &ResultsData,
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Get current scenario index
    let Some(current_index) = results_data.scenario_index else {
        // No index context - navigate to menu (scenario list)
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
            MenuData::default(),
        ))));
    };

    let next_index = current_index + 1;
    let scenario_count = ctx.game.scenario_collection.count();

    // Check if at end of list
    if next_index >= scenario_count {
        // Stay on results screen - notification will be handled in Phase 6
        return Ok(HandlerOutcome::Stay);
    }

    // Start next scenario using existing handler
    handle_start_scenario(ctx, next_index)
}

/// Handle GoToScenarioList message
///
/// Navigates directly to the Menu screen (scenario list).
/// Unlike BackToMenu which goes to ModeSelection, this goes directly to Menu.
pub fn handle_go_to_scenario_list(
    _results_data: &ResultsData,
    _ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData::default(),
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Difficulty, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
    };
    use crate::game::GameSession;
    use crate::gamification::{ProfileStorage, Quest, QuestDifficulty, QuestType, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::ui::state::{
        AppState, ConfigState, GameState, ProgressState, TaskData, TypedScreen, UIState,
    };

    fn create_test_scenario() -> Scenario {
        Scenario {
            id: "test_scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: "Test description".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\nline 3\n".to_string(),
                cursor_position: (1, 0),
            },
            target: TargetState {
                file_content: "line 1\nline 3\n".to_string(),
                cursor_position: (1, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Delete line".to_string(),
            },
            alternatives: vec![],
            hints: vec!["Hint 1".to_string()],
            scoring: ScoringConfig {
                optimal_count: 2,
                max_points: 100,
                tolerance: 1,
            },
            metadata: Some(ScenarioMetadata {
                difficulty: Some(Difficulty::Beginner),
                ..Default::default()
            }),
        }
    }

    fn create_test_state() -> AppState {
        let scenarios = vec![create_test_scenario()];
        AppState {
            screen: TypedScreen::Menu(MenuData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::new(),
            ),
            config: ConfigState::default(),
        }
    }

    #[test]
    fn test_handle_start_scenario_valid_index() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_start_scenario(&mut ctx, 0).unwrap();

        assert!(matches!(
            outcome,
            HandlerOutcome::Transition(ref screen) if matches!(**screen, TypedScreen::Task(_))
        ));
        assert!(!ctx.ui.show_key_history);
        assert!(ctx.ui.completion_time.is_none());
    }

    #[test]
    fn test_handle_start_scenario_invalid_index() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_start_scenario(&mut ctx, 999).unwrap();

        assert!(matches!(outcome, HandlerOutcome::Stay));
    }

    #[test]
    fn test_handle_abandon_scenario() {
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let task_data = TaskData::new(session);

        let screen = handle_abandon_scenario(task_data).unwrap();

        assert!(matches!(screen, TypedScreen::Results(_)));
        if let TypedScreen::Results(results) = screen {
            assert!(matches!(
                results.session,
                crate::ui::state::CompletedOrAbandoned::Abandoned(_)
            ));
        }
    }

    #[test]
    fn test_handle_retry_scenario_from_completed() {
        use crate::game::SessionAfterAction;

        let mut state = create_test_state();
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();

        // Complete the scenario by executing the solution commands
        let result = session.record_action("x".to_string()).unwrap();
        let session = match result {
            SessionAfterAction::StillActive(s) => s,
            SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
        };
        let result = session.record_action("d".to_string()).unwrap();
        let completed = match result {
            SessionAfterAction::Completed(c) => c,
            SessionAfterAction::StillActive(_) => panic!("Should complete after 'x' + 'd'"),
        };

        let feedback = completed.feedback().unwrap();
        let results_data = ResultsData::from_completed(completed, feedback, None).unwrap();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let screen = handle_retry_scenario(results_data, &mut ctx).unwrap();

        assert!(matches!(screen, TypedScreen::Task(_)));
        assert!(!ctx.ui.show_key_history);
        assert!(ctx.ui.completion_time.is_none());
    }

    #[test]
    fn test_handle_retry_scenario_from_abandoned() {
        let mut state = create_test_state();
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();
        let feedback = abandoned.feedback();

        let results_data = ResultsData::from_abandoned(abandoned, feedback, None);

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let screen = handle_retry_scenario(results_data, &mut ctx).unwrap();

        assert!(matches!(screen, TypedScreen::Task(_)));
    }

    #[test]
    fn test_handle_next_scenario() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_scenario(&mut ctx).unwrap();

        assert!(matches!(
            outcome,
            HandlerOutcome::Transition(ref screen) if matches!(**screen, TypedScreen::Menu(_))
        ));
    }

    #[test]
    fn test_collect_quest_bonuses_no_quests() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let bonuses = collect_quest_bonuses(&mut ctx);

        assert!(bonuses.is_empty());
    }

    #[test]
    fn test_collect_quest_bonuses_with_completed_quest() {
        let mut state = create_test_state();

        state.progress.profile.daily_quests = vec![Quest {
            id: "test_quest".to_string(),
            quest_type: QuestType::ScenarioCompletion {
                target: 5,
                current: 5,
            },
            description: "Complete scenarios".to_string(),
            difficulty: QuestDifficulty::Easy,
            xp_reward: 100,
            completed: true,
        }];

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let bonuses = collect_quest_bonuses(&mut ctx);

        assert_eq!(bonuses.len(), 1);
        assert_eq!(bonuses[0].1, 100);
        assert!(
            ctx.progress
                .previously_completed_quests
                .contains("test_quest")
        );
    }

    #[test]
    fn test_collect_quest_bonuses_skips_already_tracked() {
        let mut state = create_test_state();

        state.progress.profile.daily_quests = vec![Quest {
            id: "test_quest".to_string(),
            quest_type: QuestType::ScenarioCompletion {
                target: 5,
                current: 5,
            },
            description: "Complete scenarios".to_string(),
            difficulty: QuestDifficulty::Easy,
            xp_reward: 100,
            completed: true,
        }];

        state
            .progress
            .previously_completed_quests
            .insert("test_quest".to_string());

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let bonuses = collect_quest_bonuses(&mut ctx);

        assert!(bonuses.is_empty());
    }

    #[test]
    fn test_handle_complete_scenario_no_feedback() {
        let mut state = create_test_state();

        state.ui.last_feedback = None;
        state.game.pending_completed_session = None;

        let outcome = handle_complete_scenario(&mut state).unwrap();

        assert!(matches!(
            outcome,
            HandlerOutcome::Transition(ref screen) if matches!(**screen, TypedScreen::Menu(_))
        ));
    }

    #[test]
    fn test_handle_complete_scenario_with_session_no_feedback() {
        use crate::game::SessionAfterAction;

        let mut state = create_test_state();
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();

        // Complete the scenario
        let result = session.record_action("x".to_string()).unwrap();
        let session = match result {
            SessionAfterAction::StillActive(s) => s,
            SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
        };
        let result = session.record_action("d".to_string()).unwrap();
        let completed = match result {
            SessionAfterAction::Completed(c) => c,
            SessionAfterAction::StillActive(_) => panic!("Should complete after 'x' + 'd'"),
        };

        state.ui.last_feedback = None;
        state.game.pending_completed_session = Some(completed);

        let outcome = handle_complete_scenario(&mut state).unwrap();

        assert!(matches!(
            outcome,
            HandlerOutcome::Transition(ref screen) if matches!(**screen, TypedScreen::Results(_))
        ));
    }

    #[test]
    fn test_handle_complete_scenario_full_flow() {
        use crate::game::SessionAfterAction;

        let mut state = create_test_state();
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();

        // Complete the scenario
        let result = session.record_action("x".to_string()).unwrap();
        let session = match result {
            SessionAfterAction::StillActive(s) => s,
            SessionAfterAction::Completed(_) => panic!("Should not complete after just 'x'"),
        };
        let result = session.record_action("d".to_string()).unwrap();
        let completed = match result {
            SessionAfterAction::Completed(c) => c,
            SessionAfterAction::StillActive(_) => panic!("Should complete after 'x' + 'd'"),
        };

        let feedback = completed.feedback().unwrap();
        state.ui.last_feedback = Some(feedback);
        state.game.pending_completed_session = Some(completed);

        let initial_xp = state.progress.profile.total_xp;

        let outcome = handle_complete_scenario(&mut state).unwrap();

        assert!(matches!(
            outcome,
            HandlerOutcome::Transition(ref screen) if matches!(**screen, TypedScreen::Results(_))
        ));

        assert!(state.progress.profile.total_xp > initial_xp);
        assert_eq!(state.progress.scenarios_completed_today, 1);
    }

    #[test]
    fn test_handle_start_scenario_creates_task_screen() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_start_scenario(&mut ctx, 0).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Task(_)));
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_handle_abandon_creates_results_screen() {
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let task_data = TaskData::new(session);

        let screen = handle_abandon_scenario(task_data).unwrap();

        if let TypedScreen::Results(results) = screen {
            assert!(matches!(
                results.session,
                crate::ui::state::CompletedOrAbandoned::Abandoned(_)
            ));
        } else {
            panic!("Expected Results screen");
        }
    }

    #[test]
    fn test_handle_next_scenario_returns_to_menu() {
        let mut state = create_test_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_scenario(&mut ctx).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(matches!(*screen, TypedScreen::Menu(_)));
        } else {
            panic!("Expected Transition outcome");
        }
    }
}
