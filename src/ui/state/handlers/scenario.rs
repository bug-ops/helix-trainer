//! Scenario lifecycle message handlers
//!
//! Handles starting, completing, retrying, and abandoning scenarios

use crate::config::Difficulty;
use crate::constants::{FLASH_TIME_RATIO, SPEED_DEMON_TIME_RATIO};
use crate::game::GameSession;
use crate::game::services::ScenarioCompletionService;
use crate::gamification::{Achievement, AchievementEngine, speed_time_ratio};
use crate::security::UserError;
use crate::ui::notification::{Notification, NotificationType};
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
        // Create TaskData with the new session and scenario index for navigation
        let task_data = TaskData::with_index(session, index);

        // Save menu position before leaving (for later restoration)
        // Note: scroll_offset is not saved here because HandlerContext doesn't have
        // access to the current MenuData. The scroll is restored from last_menu_scroll
        // which is updated when the user navigates within the menu screen.
        ctx.ui.last_menu_selected = index;

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
    difficulty: Option<Difficulty>,
) -> Result<(), UserError> {
    let is_perfect = feedback.score == feedback.max_points;

    let leveled_up = ctx.progress.profile.add_xp(total_xp);
    ScenarioCompletionService::update_profile_counters(&mut ctx.progress.profile, is_perfect);

    if let Some(difficulty) = difficulty {
        ctx.progress
            .profile
            .difficulties_completed
            .insert(difficulty);
    }

    // Speed achievements (SpeedDemon/Speedrunner/Flash) previously only fired from
    // arcade play; Training mode has no `DifficultyController` to scale a time budget,
    // but the same base per-difficulty budget `speed_time_ratio` uses is enough to
    // judge a completion as a speed run here too.
    let time_ratio = speed_time_ratio(feedback.duration, difficulty);
    if time_ratio < SPEED_DEMON_TIME_RATIO {
        ctx.progress.profile.speed_run_count =
            ctx.progress.profile.speed_run_count.saturating_add(1);
    }
    if time_ratio < FLASH_TIME_RATIO {
        ctx.progress.profile.flash_run_count =
            ctx.progress.profile.flash_run_count.saturating_add(1);
    }

    ctx.progress.scenarios_completed_today += 1;

    let commands = ScenarioCompletionService::extract_commands(feedback);
    ctx.progress.profile.commands_executed = ctx
        .progress
        .profile
        .commands_executed
        .saturating_add(commands.len() as u32);
    let mastery_changes = ScenarioCompletionService::record_fsrs_data_with_mastery(
        &mut ctx.progress.scheduler,
        &mut ctx.progress.performance_tracker,
        &commands,
        feedback.duration,
        feedback.score > 0,
    );

    // Save after FSRS data has been recorded so `performance_data` reflects this
    // scenario's review history, not the state before it.
    // (non-fatal error - log but continue, matches handle_minigame_game_over's policy)
    if leveled_up {
        if let Err(e) = ctx.progress.save_immediate() {
            tracing::error!("Failed to save profile after scenario completion: {:?}", e);
        }
    } else if let Err(e) = ctx.progress.save_debounced() {
        tracing::error!("Failed to save profile after scenario completion: {:?}", e);
    }

    // Generate notifications for mastery level ups
    for (command, new_level) in mastery_changes {
        ctx.ui
            .notifications
            .push(Notification::new(NotificationType::MasteryLevelUp {
                command,
                new_level,
            }));
    }

    // Check and unlock any achievements newly satisfied by this completion
    // (perfect/scenario counters and command mastery all just changed above)
    let newly_unlocked = AchievementEngine::check_and_unlock(
        &mut ctx.progress.profile,
        &ctx.progress.performance_tracker,
    );
    if !newly_unlocked.is_empty() {
        for achievement_id in newly_unlocked {
            let achievement = Achievement::new(achievement_id);
            ctx.ui
                .notifications
                .push(Notification::new(NotificationType::Achievement {
                    name: achievement.name,
                    description: achievement.description,
                }));
        }
        // Persist through the shared save path (non-fatal, matches the policy
        // above) instead of a raw `storage.save`, which would skip the FSRS
        // sync and leave `profile.performance_data` stale.
        if let Err(e) = ctx.progress.save_immediate() {
            tracing::error!("Failed to save profile after achievement unlock: {:?}", e);
        }
    }

    // Generate a notification for an account-level level up (distinct from the
    // per-command mastery level ups above). Pushed last so it lands inside the
    // notification queue's fixed-size visible window even when mastery or
    // achievement notifications also fire during this completion.
    if leveled_up {
        ctx.ui
            .notifications
            .push(Notification::new(NotificationType::LevelUp {
                new_level: ctx.progress.profile.level,
            }));
    }

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
            // No session either, just go to menu with restored position
            return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
                MenuData {
                    selected_item: state.ui.last_menu_selected,
                    scroll_offset: state.ui.last_menu_scroll,
                    command_buffer: String::new(),
                },
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

    let (actual_xp, mastery_level, mastery_multiplier, mastery_factor, repeat_penalty) =
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
        mastery_factor,
        repeat_penalty,
        quest_bonuses,
        total_xp,
    });

    // Record completion and award XP
    let scenario_difficulty = completed_session
        .as_ref()
        .and_then(|s| s.scenario().metadata.as_ref())
        .and_then(|m| m.difficulty);
    record_scenario_completion(&mut ctx, &feedback, total_xp, scenario_difficulty)?;

    // Create ResultsData with completed session and transition to Results screen
    let Some(session) = completed_session else {
        // No completed session available - go back to menu with restored position
        // This shouldn't happen in normal flow, but handle gracefully
        let selected = ctx.ui.last_menu_selected;
        let scroll = ctx.ui.last_menu_scroll;
        ctx.ui.clear_temp_results();
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
            MenuData {
                selected_item: selected,
                scroll_offset: scroll,
                command_buffer: String::new(),
            },
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

    // Preserve scenario index for navigation
    let scenario_index = results_data.scenario_index;

    // Create new session with same scenario
    let new_session = GameSession::new(scenario)?;
    let mut task_data = TaskData::new(new_session);
    task_data.scenario_index = scenario_index;

    // Reset UI state
    ctx.ui.show_key_history = false;
    ctx.ui.completion_time = None;

    Ok(TypedScreen::Task(task_data))
}

/// Handle NextScenario message
///
/// Completes the current scenario flow and returns to menu
/// Restores the previous menu position for consistent navigation experience
pub fn handle_next_scenario(ctx: &mut HandlerContext<'_>) -> Result<HandlerOutcome, UserError> {
    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData {
            selected_item: ctx.ui.last_menu_selected,
            scroll_offset: ctx.ui.last_menu_scroll,
            command_buffer: String::new(),
        },
    ))))
}

/// Handle NextLesson message
///
/// Navigates to the next scenario in the filtered list.
/// If at the end of the list, stays on Results screen.
/// If no scenario_index is available, returns to Menu screen with restored position.
pub fn handle_next_lesson(
    results_data: &ResultsData,
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Get current scenario index
    let Some(current_index) = results_data.scenario_index else {
        // No index context - navigate to menu with restored position
        return Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
            MenuData {
                selected_item: ctx.ui.last_menu_selected,
                scroll_offset: ctx.ui.last_menu_scroll,
                command_buffer: String::new(),
            },
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
///
/// Auto-advance behavior: If the scenario was completed successfully, the cursor
/// moves to the next scenario in the list. Otherwise, it stays at the current position.
pub fn handle_go_to_scenario_list(
    results_data: &ResultsData,
    ctx: &mut HandlerContext<'_>,
) -> Result<HandlerOutcome, UserError> {
    // Determine selected position with auto-advance for completed scenarios
    let selected = if results_data.session.is_completed() {
        // Auto-advance: move to next scenario after successful completion
        // Clamp to last valid index; saturating_sub handles empty collection case
        results_data
            .scenario_index
            .map(|i| (i + 1).min(ctx.game.scenario_collection.count().saturating_sub(1)))
            .unwrap_or(ctx.ui.last_menu_selected)
    } else {
        // Stay at current position for abandoned scenarios
        results_data
            .scenario_index
            .unwrap_or(ctx.ui.last_menu_selected)
    };

    Ok(HandlerOutcome::Transition(Box::new(TypedScreen::Menu(
        MenuData {
            selected_item: selected,
            scroll_offset: ctx.ui.last_menu_scroll,
            command_buffer: String::new(),
        },
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::game::GameSession;
    use crate::gamification::{ProfileStorage, Quest, QuestDifficulty, QuestType, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{
        AppState, ConfigState, GameState, ProgressState, TaskData, TypedScreen, UIState,
    };

    fn create_test_scenario() -> Scenario {
        create_test_scenario_with_difficulty("test_scenario", Difficulty::Beginner)
    }

    fn create_test_scenario_with_difficulty(id: &str, difficulty: Difficulty) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_cursor(1, 0)
            .target_content("line 1\nline 3\n")
            .target_cursor(1, 0)
            .hint("Hint 1")
            .tolerance(1)
            .difficulty(difficulty)
            .build()
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
                ProfileStorage::for_test(),
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
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let mut state = create_test_state();
        let temp_dir = TempDir::new().unwrap();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
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

    /// Regression test for #256: achievements must unlock through the live scenario
    /// completion path (`handle_complete_scenario`), not just via direct
    /// `AchievementEngine::check_achievements` calls.
    #[test]
    fn test_handle_complete_scenario_unlocks_achievement() {
        use crate::game::SessionAfterAction;
        use crate::gamification::AchievementId;

        let mut state = create_test_state();
        // One completion away from the Centurion milestone (100 scenarios completed),
        // regardless of this scenario's score.
        state.progress.profile.scenarios_completed = 99;

        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();

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

        handle_complete_scenario(&mut state).unwrap();

        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::Centurion)
        );
        assert!(state.ui.notifications.visible().iter().any(|n| matches!(
            n.notification_type,
            crate::ui::notification::NotificationType::Achievement { ref name, .. }
                if name == "Centurion"
        )));
    }

    /// Regression test for #290: completing a scenario must record its difficulty
    /// into `profile.difficulties_completed` via the `difficulty` param threaded
    /// through `record_scenario_completion`, not just update score-based counters.
    #[test]
    fn test_handle_complete_scenario_updates_difficulties_completed() {
        use crate::game::SessionAfterAction;

        let mut state = create_test_state();
        assert!(state.progress.profile.difficulties_completed.is_empty());

        let scenario = create_test_scenario(); // Difficulty::Beginner
        let session = GameSession::new(scenario).unwrap();

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

        handle_complete_scenario(&mut state).unwrap();

        assert!(
            state
                .progress
                .profile
                .difficulties_completed
                .contains(&Difficulty::Beginner)
        );
    }

    /// Regression test for #290: `Polyglot` must unlock once a scenario from every
    /// difficulty tier has been completed through the live completion path.
    #[test]
    fn test_handle_complete_scenario_unlocks_polyglot_after_all_difficulties() {
        use crate::game::SessionAfterAction;
        use crate::gamification::AchievementId;

        let mut state = create_test_state();

        for difficulty in [
            Difficulty::Beginner,
            Difficulty::Intermediate,
            Difficulty::Advanced,
        ] {
            let scenario = create_test_scenario_with_difficulty("multi_difficulty", difficulty);
            let session = GameSession::new(scenario).unwrap();

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

            handle_complete_scenario(&mut state).unwrap();
        }

        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::Polyglot)
        );
        assert!(state.ui.notifications.visible().iter().any(|n| matches!(
            n.notification_type,
            crate::ui::notification::NotificationType::Achievement { ref name, .. }
                if name == "Polyglot"
        )));
    }

    /// Regression test for S2 (#290 follow-up): speed achievements (`SpeedDemon`/
    /// `Flash`) previously only unlocked from arcade play, since Training mode had no
    /// concept of a per-scenario time budget. `record_scenario_completion` now derives
    /// one from the scenario's difficulty via `gamification::speed_time_ratio`, so an
    /// immediate (near-zero elapsed) Training-mode completion also counts as a speed run.
    #[test]
    fn test_handle_complete_scenario_unlocks_speed_achievements() {
        use crate::game::SessionAfterAction;
        use crate::gamification::AchievementId;

        let mut state = create_test_state();

        let scenario = create_test_scenario(); // Difficulty::Beginner
        let session = GameSession::new(scenario).unwrap();

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

        handle_complete_scenario(&mut state).unwrap();

        assert_eq!(state.progress.profile.speed_run_count, 1);
        assert_eq!(state.progress.profile.flash_run_count, 1);
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::SpeedDemon)
        );
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::Flash)
        );
    }

    /// Compares the fields that `save_immediate`/`save_debounced` sync from the
    /// tracker. `CommandPerformance` has no `PartialEq` impl.
    fn stats_match(
        a: &std::collections::HashMap<String, crate::learning::CommandPerformance>,
        b: &std::collections::HashMap<String, crate::learning::CommandPerformance>,
    ) -> bool {
        a.len() == b.len()
            && a.iter().all(|(k, v)| {
                b.get(k).is_some_and(|other| {
                    other.attempts == v.attempts
                        && other.successes == v.successes
                        && other.reps == v.reps
                        && other.command == v.command
                })
            })
    }

    fn completed_feedback_with_commands() -> crate::game::Feedback {
        use crate::game::SessionAfterAction;

        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();

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

        completed.feedback().unwrap()
    }

    /// Regression test for #273: the FSRS-recording step in `record_scenario_completion`
    /// must run *before* the save, otherwise the persisted profile captures stale
    /// (pre-completion) tracker state even though the in-memory tracker is up to date.
    #[test]
    fn test_record_scenario_completion_persists_fsrs_data_no_level_up() {
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut state = create_test_state();
        state.progress.storage = ProfileStorage::with_path(&profile_path);
        // Fresh profile: no prior save recorded, so save_debounced will fire.
        assert!(state.progress.should_save());

        let feedback = completed_feedback_with_commands();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Small XP amount that should not trigger a level-up, exercising the
        // save_debounced branch.
        record_scenario_completion(&mut ctx, &feedback, 1, None).unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(
            !persisted.performance_data.is_empty(),
            "persisted profile should contain FSRS data recorded during completion"
        );
        assert!(stats_match(
            &persisted.performance_data,
            &ctx.progress.performance_tracker.get_stats_clone()
        ));
    }

    /// Regression test for #296/S1/S2: an achievement unlock (independent of any
    /// level-up) must persist through `ProgressState::save_immediate` - not a raw
    /// `storage.save` - so `profile.performance_data` reflects the FSRS tracker even
    /// when the earlier `save_debounced` call in the same completion was skipped
    /// (debounce window not yet elapsed).
    #[test]
    fn test_record_scenario_completion_achievement_save_syncs_fsrs_data() {
        use crate::gamification::{AchievementId, ProfileStorage};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut state = create_test_state();
        state.progress.storage = ProfileStorage::with_path(&profile_path);
        // One completion away from the Centurion milestone, independent of level-up.
        state.progress.profile.scenarios_completed = 99;
        // Simulate a save that just happened, so the debounced save below is skipped.
        state.progress.mark_saved();
        assert!(!state.progress.should_save());

        let feedback = completed_feedback_with_commands();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Small XP amount that should not trigger a level-up, exercising the
        // skipped save_debounced branch alongside the achievement-unlock save.
        record_scenario_completion(&mut ctx, &feedback, 1, None).unwrap();

        assert!(
            ctx.progress
                .profile
                .has_achievement(&AchievementId::Centurion),
            "expected Centurion to unlock at 100 completed scenarios"
        );

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(
            !persisted.performance_data.is_empty(),
            "persisted profile should contain FSRS data even though save_debounced was skipped"
        );
        assert!(stats_match(
            &persisted.performance_data,
            &ctx.progress.performance_tracker.get_stats_clone()
        ));
    }

    /// Same as above but through the level-up (save_immediate) branch.
    #[test]
    fn test_record_scenario_completion_persists_fsrs_data_on_level_up() {
        use crate::gamification::{ProfileStorage, XPCalculator};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut state = create_test_state();
        state.progress.storage = ProfileStorage::with_path(&profile_path);
        let xp_for_level_2 = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_level_2 - 10;
        state.progress.profile.level = 1;

        let feedback = completed_feedback_with_commands();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        record_scenario_completion(&mut ctx, &feedback, 100, None).unwrap();
        assert!(ctx.progress.profile.level >= 2, "expected a level-up");

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(!persisted.performance_data.is_empty());
        assert!(stats_match(
            &persisted.performance_data,
            &ctx.progress.performance_tracker.get_stats_clone()
        ));

        // Regression test for #293/S3: this feedback also triggers mastery-level-up
        // and achievement-unlock notifications (>= 3 total alongside LevelUp), which
        // would evict LevelUp from the notification queue's fixed-size (3-slot)
        // `visible()` window if it were pushed first instead of last.
        assert!(
            ctx.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { .. }
            )),
            "LevelUp notification must remain visible even when mastery/achievement \
             notifications also fire in the same completion"
        );
    }

    /// Minimal feedback with no user actions, used to isolate the `LevelUp`
    /// notification from mastery/achievement notifications that a full
    /// completion (like `completed_feedback_with_commands`) would also trigger.
    fn minimal_feedback() -> crate::game::Feedback {
        use crate::game::PerformanceRating;
        crate::game::Feedback {
            scenario_id: "test_scenario".to_string(),
            success: true,
            score: 1,
            max_points: 2,
            rating: PerformanceRating::Good,
            actions_taken: 1,
            optimal_actions: 1,
            duration: std::time::Duration::from_secs(5),
            hint: None,
            is_optimal: false,
            user_actions: vec![],
        }
    }

    /// Regression test for #293: `handle_award_xp`'s dead `LevelUp` notification path
    /// was removed; the live scenario-completion path must notify on level up instead.
    #[test]
    fn test_record_scenario_completion_notifies_on_level_up() {
        use crate::gamification::{ProfileStorage, XPCalculator};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let mut state = create_test_state();
        state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        let xp_for_level_2 = XPCalculator::xp_for_level(2);
        state.progress.profile.total_xp = xp_for_level_2 - 10;
        state.progress.profile.level = 1;

        let feedback = minimal_feedback();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        record_scenario_completion(&mut ctx, &feedback, 100, None).unwrap();
        assert!(ctx.progress.profile.level >= 2, "expected a level-up");

        assert!(
            ctx.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { .. }
            )),
            "expected a LevelUp notification on account level up"
        );
    }

    /// Regression test for #293: a scenario completion that does NOT level up the
    /// account must not push a `LevelUp` notification.
    #[test]
    fn test_record_scenario_completion_no_level_up_notification_without_level_up() {
        let mut state = create_test_state();
        let feedback = minimal_feedback();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Small XP amount that should not trigger a level-up.
        record_scenario_completion(&mut ctx, &feedback, 1, None).unwrap();

        assert!(
            !ctx.ui.notifications.visible().iter().any(|n| matches!(
                n.notification_type,
                crate::ui::notification::NotificationType::LevelUp { .. }
            )),
            "no LevelUp notification should fire without an actual level up"
        );
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

    // ============================================================================
    // LESSON NAVIGATION TESTS
    // ============================================================================

    fn create_multi_scenario_state() -> AppState {
        let scenarios = vec![
            create_test_scenario_with_id("scenario_1"),
            create_test_scenario_with_id("scenario_2"),
            create_test_scenario_with_id("scenario_3"),
        ];
        AppState {
            screen: TypedScreen::Menu(MenuData::default()),
            ui: UIState::new(),
            game: GameState::new(scenarios),
            progress: ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            config: ConfigState::default(),
        }
    }

    fn create_test_scenario_with_id(id: &str) -> Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_cursor(1, 0)
            .target_content("line 1\nline 3\n")
            .target_cursor(1, 0)
            .hint("Hint 1")
            .tolerance(1)
            .difficulty(Difficulty::Beginner)
            .build()
    }

    fn create_results_data_with_index(scenario_index: Option<usize>) -> ResultsData {
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();
        let feedback = abandoned.feedback();
        ResultsData::from_abandoned(abandoned, feedback, scenario_index)
    }

    #[test]
    fn test_handle_next_lesson_navigates_to_next_scenario() {
        let mut state = create_multi_scenario_state();
        let results_data = create_results_data_with_index(Some(0));

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_lesson(&results_data, &mut ctx).unwrap();

        // Should transition to Task screen with next scenario (index 1)
        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Task(task_data) = *screen {
                assert_eq!(task_data.scenario_index, Some(1));
            } else {
                panic!("Expected Task screen, got {:?}", screen.screen_type());
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_handle_next_lesson_at_end_of_list_stays() {
        let mut state = create_multi_scenario_state();
        // Index 2 is the last scenario (0, 1, 2 for 3 scenarios)
        let results_data = create_results_data_with_index(Some(2));

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_lesson(&results_data, &mut ctx).unwrap();

        // Should stay on results screen
        assert!(
            matches!(outcome, HandlerOutcome::Stay),
            "Expected Stay outcome at end of list"
        );
    }

    #[test]
    fn test_handle_next_lesson_without_index_goes_to_menu() {
        let mut state = create_multi_scenario_state();
        let results_data = create_results_data_with_index(None);

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_lesson(&results_data, &mut ctx).unwrap();

        // Should transition to Menu screen
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(
                matches!(*screen, TypedScreen::Menu(_)),
                "Expected Menu screen"
            );
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_handle_go_to_scenario_list_transitions_to_menu() {
        let mut state = create_multi_scenario_state();
        let results_data = create_results_data_with_index(Some(1));

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_go_to_scenario_list(&results_data, &mut ctx).unwrap();

        // Should always transition to Menu screen
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(
                matches!(*screen, TypedScreen::Menu(_)),
                "Expected Menu screen"
            );
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_handle_go_to_scenario_list_without_index_transitions_to_menu() {
        let mut state = create_multi_scenario_state();
        let results_data = create_results_data_with_index(None);

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_go_to_scenario_list(&results_data, &mut ctx).unwrap();

        // Should always transition to Menu screen regardless of scenario_index
        if let HandlerOutcome::Transition(screen) = outcome {
            assert!(
                matches!(*screen, TypedScreen::Menu(_)),
                "Expected Menu screen"
            );
        } else {
            panic!("Expected Transition outcome");
        }
    }

    // ============================================================================
    // INDEX PROPAGATION TESTS
    // ============================================================================

    #[test]
    fn test_start_scenario_stores_index() {
        let mut state = create_multi_scenario_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_start_scenario(&mut ctx, 1).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Task(task_data) = *screen {
                assert_eq!(
                    task_data.scenario_index,
                    Some(1),
                    "TaskData should store scenario index"
                );
            } else {
                panic!("Expected Task screen");
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_retry_scenario_preserves_index() {
        let mut state = create_multi_scenario_state();

        // Create ResultsData with scenario_index = Some(1)
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();
        let feedback = abandoned.feedback();
        let results_data = ResultsData::from_abandoned(abandoned, feedback, Some(1));

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let screen = handle_retry_scenario(results_data, &mut ctx).unwrap();

        if let TypedScreen::Task(task_data) = screen {
            assert_eq!(
                task_data.scenario_index,
                Some(1),
                "Retry should preserve scenario_index"
            );
        } else {
            panic!("Expected Task screen after retry");
        }
    }

    #[test]
    fn test_abandon_scenario_preserves_index() {
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let mut task_data = TaskData::new(session);
        task_data.scenario_index = Some(2);

        let screen = handle_abandon_scenario(task_data).unwrap();

        if let TypedScreen::Results(results_data) = screen {
            assert_eq!(
                results_data.scenario_index,
                Some(2),
                "Abandon should preserve scenario_index in ResultsData"
            );
        } else {
            panic!("Expected Results screen after abandon");
        }
    }

    // ============================================================================
    // MENU POSITION PERSISTENCE TESTS
    // ============================================================================

    #[test]
    fn test_menu_position_saved_on_start_scenario() {
        let mut state = create_multi_scenario_state();
        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        // Start scenario at index 2
        let outcome = handle_start_scenario(&mut ctx, 2).unwrap();

        // Verify transition to Task screen
        assert!(matches!(
            outcome,
            HandlerOutcome::Transition(ref screen) if matches!(**screen, TypedScreen::Task(_))
        ));

        // Verify menu position was saved
        assert_eq!(
            ctx.ui.last_menu_selected, 2,
            "Menu position should be saved when starting scenario"
        );
    }

    #[test]
    fn test_menu_position_restored_on_return() {
        let mut state = create_multi_scenario_state();

        // Set saved menu position
        state.ui.last_menu_selected = 2;
        state.ui.last_menu_scroll = 1;

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_scenario(&mut ctx).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Menu(menu_data) = *screen {
                assert_eq!(
                    menu_data.selected_item, 2,
                    "Menu should restore selected_item"
                );
                assert_eq!(
                    menu_data.scroll_offset, 1,
                    "Menu should restore scroll_offset"
                );
            } else {
                panic!("Expected Menu screen");
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_auto_advance_after_completion() {
        use crate::game::SessionAfterAction;

        let mut state = create_multi_scenario_state();
        state.ui.last_menu_selected = 0;

        // Create a completed session at index 0
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
        let results_data = ResultsData::from_completed(completed, feedback, Some(0)).unwrap();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_go_to_scenario_list(&results_data, &mut ctx).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Menu(menu_data) = *screen {
                assert_eq!(
                    menu_data.selected_item, 1,
                    "Menu should auto-advance to next scenario after completion"
                );
            } else {
                panic!("Expected Menu screen");
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_no_advance_after_abandon() {
        let mut state = create_multi_scenario_state();
        state.ui.last_menu_selected = 0;

        // Create an abandoned session at index 1
        let scenario = create_test_scenario();
        let session = GameSession::new(scenario).unwrap();
        let abandoned = session.abandon();
        let feedback = abandoned.feedback();
        let results_data = ResultsData::from_abandoned(abandoned, feedback, Some(1));

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_go_to_scenario_list(&results_data, &mut ctx).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Menu(menu_data) = *screen {
                assert_eq!(
                    menu_data.selected_item, 1,
                    "Menu should NOT auto-advance after abandon - stay at current"
                );
            } else {
                panic!("Expected Menu screen");
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_auto_advance_clamps_to_last_scenario() {
        use crate::game::SessionAfterAction;

        let mut state = create_multi_scenario_state();
        // Index 2 is the last scenario (3 scenarios: 0, 1, 2)
        state.ui.last_menu_selected = 2;

        // Create a completed session at the last index
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
        let results_data = ResultsData::from_completed(completed, feedback, Some(2)).unwrap();

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_go_to_scenario_list(&results_data, &mut ctx).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Menu(menu_data) = *screen {
                assert_eq!(
                    menu_data.selected_item, 2,
                    "Menu should clamp to last scenario when at end of list"
                );
            } else {
                panic!("Expected Menu screen");
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }

    #[test]
    fn test_next_lesson_returns_menu_with_position_when_no_index() {
        let mut state = create_multi_scenario_state();
        state.ui.last_menu_selected = 2;
        state.ui.last_menu_scroll = 1;

        // ResultsData without scenario_index
        let results_data = create_results_data_with_index(None);

        let mut ctx = HandlerContext::new(
            &mut state.ui,
            &mut state.game,
            &mut state.progress,
            &state.config,
        );

        let outcome = handle_next_lesson(&results_data, &mut ctx).unwrap();

        if let HandlerOutcome::Transition(screen) = outcome {
            if let TypedScreen::Menu(menu_data) = *screen {
                assert_eq!(
                    menu_data.selected_item, 2,
                    "Menu should restore last_menu_selected when no scenario_index"
                );
                assert_eq!(
                    menu_data.scroll_offset, 1,
                    "Menu should restore last_menu_scroll when no scenario_index"
                );
            } else {
                panic!("Expected Menu screen");
            }
        } else {
            panic!("Expected Transition outcome");
        }
    }
}
