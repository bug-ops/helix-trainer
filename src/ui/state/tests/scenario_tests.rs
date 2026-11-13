//! Tests for scenario lifecycle (start, complete, abandon, retry)

use crate::testing::{ncmd, qid};
use std::assert_matches;

use super::common::{create_test_app_state, create_test_scenario, exec};
use crate::grammar::commands::{CMD_DELETE_SELECTION, CMD_SELECT_LINE};
use crate::testing::update_executing_effects as update;
use crate::ui::state::{Message, TypedScreen};

#[test]
#[expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads better than the machine fix"
)]
fn test_start_scenario() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();

    // After typestate refactoring, session is in TypedScreen::Task, not game.session
    assert_matches!(state.screen, TypedScreen::Task(_));
    if let TypedScreen::Task(task_data) = &state.screen {
        // Verify session exists in task data
        assert!(!task_data.session.playable().current_content().is_empty());
    }
}

#[test]
fn test_start_invalid_scenario_index() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    crate::testing::start_scenario_from_menu(&mut state, 999).unwrap();

    // Should stay on the menu (not transition to Task)
    assert_matches!(state.screen, TypedScreen::Menu(_));
}

#[test]
fn test_complete_scenario_navigates_to_results() {
    use crate::gamification::ProfileStorage;
    use tempfile::TempDir;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    let temp_dir = TempDir::new().unwrap();
    state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();
    assert_matches!(state.screen, TypedScreen::Task(_));

    // Execute the solution command to reach target state
    // In Helix, 'xd' = select line + delete selection (or legacy 'x')
    update(&mut state, exec(CMD_SELECT_LINE)).unwrap();
    update(&mut state, exec(CMD_DELETE_SELECTION)).unwrap();

    // After completing the scenario the task is completed (success animation starts)
    // Screen stays on Task until CompleteScenario message is sent after delay
    assert!(state.screen.completed_task().is_some());
    assert_matches!(state.screen, TypedScreen::Task(_));

    // Simulate the delayed transition (event loop sends CompleteScenario after 1.5s)
    update(&mut state, Message::CompleteScenario).unwrap();

    // Now should be on Results screen
    assert_matches!(state.screen, TypedScreen::Results(_));
}

/// Complete the test scenario and stay on the task screen for the success animation
fn completed_task_state() -> crate::ui::state::AppState {
    let mut state = create_test_app_state(vec![create_test_scenario()]);
    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();
    update(&mut state, exec(CMD_SELECT_LINE)).unwrap();
    update(&mut state, exec(CMD_DELETE_SELECTION)).unwrap();
    assert!(state.screen.completed_task().is_some());
    state
}

#[test]
fn test_completed_task_carries_its_feedback() {
    let state = completed_task_state();

    let completed = state.screen.completed_task().unwrap();
    assert!(completed.feedback.success());
    assert_eq!(
        completed.feedback.scenario_id,
        completed.session.scenario().id
    );
}

#[test]
fn test_commands_are_ignored_while_the_task_is_completed() {
    let mut state = completed_task_state();
    let content_before = match &state.screen {
        TypedScreen::Task(task) => task.session.playable().current_content(),
        _ => panic!("Should stay on Task screen"),
    };

    update(&mut state, exec("j")).unwrap();

    let TypedScreen::Task(task) = &state.screen else {
        panic!("Should stay on Task screen");
    };
    assert!(task.session.completed().is_some());
    assert_eq!(task.session.playable().current_content(), content_before);
}

#[test]
fn test_abandon_during_success_animation_keeps_completed_task() {
    let mut state = completed_task_state();

    update(&mut state, Message::AbandonScenario).unwrap();

    assert!(state.screen.completed_task().is_some());
}

#[test]
fn test_complete_scenario_without_completed_task_goes_to_menu() {
    let mut state = create_test_app_state(vec![create_test_scenario()]);
    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();

    update(&mut state, Message::CompleteScenario).unwrap();

    assert_matches!(state.screen, TypedScreen::Menu(_));
}

#[test]
fn test_abandon_scenario_navigates_to_results() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();
    // After TypedScreen refactoring, verify we're on Task screen
    assert_matches!(state.screen, TypedScreen::Task(_));

    update(&mut state, Message::AbandonScenario).unwrap();
    // Should transition to Results screen
    if let TypedScreen::Results(results_data) = &state.screen {
        assert!(!results_data.feedback.success());
        assert_eq!(results_data.feedback.score, 0);
    } else {
        panic!("Should be on Results screen after abandon");
    }
}

#[test]
fn test_show_hint() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(!task_data.hint.is_shown());
    } else {
        panic!("Should be on Task screen");
    }

    update(&mut state, Message::ShowHint).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.hint.is_shown());
        assert!(task_data.hint.text().is_some());
    } else {
        panic!("Should be on Task screen");
    }
}

#[test]
fn test_retry_scenario_resets_state() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();

    // Execute an action to increase action count
    update(&mut state, exec("l")).unwrap();

    // Verify we have 1 action recorded
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.playable().action_count(), 1);
    }

    // Abandon to go to Results screen
    update(&mut state, Message::AbandonScenario).unwrap();
    assert_matches!(state.screen, TypedScreen::Results(_));

    // Now retry - this should create a fresh session with action count = 0
    update(&mut state, Message::RetryScenario).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.playable().action_count(), 0);
    } else {
        panic!("Should be on Task screen after retry");
    }
}

#[test]
fn test_previously_completed_quests_tracking() {
    use crate::gamification::{ProfileStorage, Quest, QuestDifficulty, QuestType};
    use tempfile::TempDir;

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    let temp_dir = TempDir::new().unwrap();
    state.progress.storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

    // Add a quest that will be completed on first scenario
    // In Helix, use 'x' (select_line) for line-based quests
    {
        let profile = &mut state.progress.profile;
        profile.push_daily_quest(Quest::new(
            qid("test_quest"),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 1,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    // First scenario completion
    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();

    // Execute command through message to trigger quest progress tracking and complete scenario
    // In Helix, 'xd' = select line + delete selection
    update(&mut state, exec(CMD_SELECT_LINE)).unwrap();
    update(&mut state, exec(CMD_DELETE_SELECTION)).unwrap();

    // After executing the solution the task is completed (success animation)
    // Screen stays on Task until CompleteScenario message is sent after delay
    assert!(
        state.screen.completed_task().is_some(),
        "the task should be completed after finishing the scenario"
    );
    assert_matches!(
        state.screen,
        TypedScreen::Task(_),
        "Should stay on Task screen during success animation"
    );

    // Quest should be completed (command was tracked during gameplay)
    {
        let profile = &state.progress.profile;
        let quest = &profile.daily_quests()[0];
        assert!(
            quest.is_completed(),
            "Quest should be completed after executing 'x'. Quest state: {quest:?}"
        );
    }

    // Simulate the delayed transition (event loop sends CompleteScenario after 1.5s)
    update(&mut state, Message::CompleteScenario).unwrap();

    // Now should be on Results screen
    assert_matches!(
        state.screen,
        TypedScreen::Results(_),
        "Should be on Results screen after CompleteScenario"
    );
}
