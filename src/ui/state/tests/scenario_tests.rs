//! Tests for scenario lifecycle (start, complete, abandon, retry)

use std::borrow::Cow;

use super::common::{create_test_app_state, create_test_scenario};
use crate::helix::commands::{CMD_DELETE_SELECTION, CMD_SELECT_LINE};
use crate::ui::state::{Message, TypedScreen, update};

#[test]
fn test_start_scenario() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();

    // After typestate refactoring, session is in TypedScreen::Task, not game.session
    assert!(matches!(state.screen, TypedScreen::Task(_)));
    if let TypedScreen::Task(task_data) = &state.screen {
        // Verify session exists in task data
        assert!(!task_data.session.current_state().content().is_empty());
    }
}

#[test]
fn test_start_invalid_scenario_index() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(999)).unwrap();

    // Should stay on current screen (not transition to Task)
    assert!(matches!(state.screen, TypedScreen::ModeSelection(_)));
}

#[test]
fn test_complete_scenario_navigates_to_results() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();
    assert!(matches!(state.screen, TypedScreen::Task(_)));

    // Execute the solution command to reach target state
    // In Helix, 'xd' = select line + delete selection (or legacy 'x')
    update(
        &mut state,
        Message::ExecuteCommand(Cow::Borrowed(CMD_SELECT_LINE)),
    )
    .unwrap();
    update(
        &mut state,
        Message::ExecuteCommand(Cow::Borrowed(CMD_DELETE_SELECTION)),
    )
    .unwrap();

    // After completing the scenario, completion_time is set (success animation starts)
    // Screen stays on Task until CompleteScenario message is sent after delay
    assert!(state.ui.completion_time.is_some());
    assert!(matches!(state.screen, TypedScreen::Task(_)));

    // Simulate the delayed transition (event loop sends CompleteScenario after 1.5s)
    update(&mut state, Message::CompleteScenario).unwrap();

    // Now should be on Results screen
    assert!(matches!(state.screen, TypedScreen::Results(_)));
}

#[test]
fn test_abandon_scenario_navigates_to_results() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();
    // After TypedScreen refactoring, verify we're on Task screen
    assert!(matches!(state.screen, TypedScreen::Task(_)));

    update(&mut state, Message::AbandonScenario).unwrap();
    // Should transition to Results screen
    if let TypedScreen::Results(results_data) = &state.screen {
        assert!(!results_data.feedback.success);
        assert_eq!(results_data.feedback.score, 0);
    } else {
        panic!("Should be on Results screen after abandon");
    }
}

#[test]
fn test_show_hint() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(!task_data.show_hint_panel);
    } else {
        panic!("Should be on Task screen");
    }

    update(&mut state, Message::ShowHint).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert!(task_data.show_hint_panel);
        assert!(task_data.current_hint.is_some());
    } else {
        panic!("Should be on Task screen");
    }
}

#[test]
fn test_retry_scenario_resets_state() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();

    // Execute an action to increase action count
    update(&mut state, Message::ExecuteCommand(Cow::Borrowed("l"))).unwrap();

    // Verify we have 1 action recorded
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.action_count(), 1);
    }

    // Abandon to go to Results screen
    update(&mut state, Message::AbandonScenario).unwrap();
    assert!(matches!(state.screen, TypedScreen::Results(_)));

    // Now retry - this should create a fresh session with action count = 0
    update(&mut state, Message::RetryScenario).unwrap();
    if let TypedScreen::Task(task_data) = &state.screen {
        assert_eq!(task_data.session.action_count(), 0);
    } else {
        panic!("Should be on Task screen after retry");
    }
}

#[test]
fn test_next_scenario_clears_session() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    update(&mut state, Message::StartScenario(0)).unwrap();
    // Verify we're on Task screen with active session
    assert!(matches!(state.screen, TypedScreen::Task(_)));

    update(&mut state, Message::NextScenario).unwrap();
    // Should return to menu
    assert!(matches!(state.screen, TypedScreen::Menu(_)));
}

#[test]
fn test_previously_completed_quests_tracking() {
    use crate::gamification::{Quest, QuestDifficulty, QuestType};

    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario.clone()]);

    // Add a quest that will be completed on first scenario
    // In Helix, use 'x' (select_line) for line-based quests
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            "test_quest".to_string(),
            QuestType::CommandPractice {
                command: "x".to_string(),
                target: 1,
                current: 0,
            },
            "Select 1 line".to_string(),
            QuestDifficulty::Easy,
        ));
    }

    // First scenario completion
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Execute command through message to trigger quest progress tracking and complete scenario
    // In Helix, 'xd' = select line + delete selection
    update(
        &mut state,
        Message::ExecuteCommand(Cow::Borrowed(CMD_SELECT_LINE)),
    )
    .unwrap();
    update(
        &mut state,
        Message::ExecuteCommand(Cow::Borrowed(CMD_DELETE_SELECTION)),
    )
    .unwrap();

    // After executing the solution, completion_time is set (success animation)
    // Screen stays on Task until CompleteScenario message is sent after delay
    assert!(
        state.ui.completion_time.is_some(),
        "completion_time should be set after completing scenario"
    );
    assert!(
        matches!(state.screen, TypedScreen::Task(_)),
        "Should stay on Task screen during success animation"
    );

    // Quest should be completed (command was tracked during gameplay)
    {
        let profile = &state.progress.profile;
        let quest = &profile.daily_quests[0];
        assert!(
            quest.is_completed(),
            "Quest should be completed after executing 'x'. Quest state: {:?}",
            quest
        );
    }

    // Simulate the delayed transition (event loop sends CompleteScenario after 1.5s)
    update(&mut state, Message::CompleteScenario).unwrap();

    // Now should be on Results screen
    assert!(
        matches!(state.screen, TypedScreen::Results(_)),
        "Should be on Results screen after CompleteScenario"
    );
}
