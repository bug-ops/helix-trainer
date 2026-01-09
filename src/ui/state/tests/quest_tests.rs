//! Tests for quest tracking and XP rewards

use std::collections::HashSet;
use std::time::Duration;

use super::common::{create_test_app_state, create_test_scenario};
use crate::gamification::{Quest, QuestDifficulty, QuestType};
use crate::ui::state::{Message, update};

#[test]
fn test_quest_progress_command_practice() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Add a CommandPractice quest to profile
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            "test_x".to_string(),
            QuestType::CommandPractice {
                command: "x".to_string(),
                target: 3,
                current: 0,
            },
            "Delete 3 lines".to_string(),
            QuestDifficulty::Easy,
        ));
    }

    // Execute "x" command twice
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Quest should not be completed yet (2/3)
    {
        let profile = &state.progress.profile;
        assert!(!profile.daily_quests[0].is_completed());
    }

    // Execute once more
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Quest should now be completed and bonus XP awarded
    {
        let profile = &state.progress.profile;
        assert!(profile.daily_quests[0].is_completed());
        // XP should be at least the quest reward
        assert!(profile.total_xp >= 25); // Easy CommandPractice = 25 XP
    }
}

#[test]
fn test_quest_progress_scenario_completion() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Add a ScenarioCompletion quest to profile
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            "test_scenario".to_string(),
            QuestType::ScenarioCompletion {
                target: 2,
                current: 0,
            },
            "Complete 2 scenarios".to_string(),
            QuestDifficulty::Medium,
        ));
    }

    // Start and "complete" a scenario
    update(&mut state, Message::StartScenario(0)).unwrap();

    // Simulate scenario completion
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: None,
            scenario_completed: true,
            scenario_id: Some("test_scenario".to_string()),
            duration: Duration::from_secs(5),
        },
    )
    .unwrap();

    // Quest should not be completed yet (1/2)
    {
        let profile = &state.progress.profile;
        assert!(!profile.daily_quests[0].is_completed());
    }

    // Complete another scenario
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: None,
            scenario_completed: true,
            scenario_id: Some("test_scenario".to_string()),
            duration: Duration::from_secs(5),
        },
    )
    .unwrap();

    // Quest should now be completed
    {
        let profile = &state.progress.profile;
        assert!(profile.daily_quests[0].is_completed());
        // XP should include quest reward
        assert!(profile.total_xp >= 75); // Medium ScenarioCompletion = 75 XP
    }
}

#[test]
fn test_quest_completion_awards_bonus_xp() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    let initial_xp = {
        let profile = &state.progress.profile;
        profile.total_xp
    };

    // Add a quest
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            "test_quest".to_string(),
            QuestType::CommandPractice {
                command: "x".to_string(),
                target: 1,
                current: 0,
            },
            "Delete 1 character".to_string(),
            QuestDifficulty::Easy,
        ));
    }

    // Complete the quest
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Check that XP was awarded
    {
        let profile = &state.progress.profile;
        assert_eq!(profile.total_xp, initial_xp + 25); // Easy CommandPractice = 25 XP
        assert!(profile.daily_quests[0].is_completed());
    }
}

#[test]
fn test_exploration_quest_tracking() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    let initial_xp = {
        let profile = &state.progress.profile;
        profile.total_xp
    };

    // Add an Exploration quest
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            "test_exploration".to_string(),
            QuestType::Exploration {
                target_commands: 3,
                commands_used: HashSet::new(),
            },
            "Use 3 different commands".to_string(),
            QuestDifficulty::Hard,
        ));
    }

    // Execute different commands
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("yy".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Not completed yet (2/3)
    {
        let profile = &state.progress.profile;
        assert!(!profile.daily_quests[0].is_completed());
    }

    // Execute third unique command
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("p".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Should be completed now and bonus XP awarded
    {
        let profile = &state.progress.profile;
        assert!(profile.daily_quests[0].is_completed());
        assert_eq!(profile.total_xp, initial_xp + 160); // Hard Exploration = 160 XP
    }
}

#[test]
fn test_commands_used_today_tracking() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    assert_eq!(state.progress.commands_used_today.len(), 0);

    // Execute some commands
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("yy".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Same command again (should not duplicate)
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Should have 2 unique commands
    assert_eq!(state.progress.commands_used_today.len(), 2);
    assert!(state.progress.commands_used_today.contains("x"));
    assert!(state.progress.commands_used_today.contains("yy"));
}

#[test]
fn test_quest_progress_changes_tracking() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Add a quest
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            "test_quest".to_string(),
            QuestType::CommandPractice {
                command: "x".to_string(),
                target: 3,
                current: 0,
            },
            "Delete 3 lines".to_string(),
            QuestDifficulty::Easy,
        ));
    }

    // Execute command to trigger progress
    update(
        &mut state,
        Message::UpdateQuestProgress {
            command: Some("x".to_string()),
            scenario_completed: false,
            scenario_id: None,
            duration: Duration::from_secs(0),
        },
    )
    .unwrap();

    // Check that progress changes were recorded
    assert_eq!(state.ui.quest_progress_changes.len(), 1);
    let change = &state.ui.quest_progress_changes[0];
    assert_eq!(change.old_progress, 0);
    assert_eq!(change.new_progress, 1);
    assert!(change.quest_description.contains("x"));
}
