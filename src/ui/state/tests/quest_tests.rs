//! Tests for quest tracking and XP rewards

use std::collections::HashSet;
use std::time::Duration;

use super::common::{create_test_app_state, create_test_scenario};
use crate::gamification::{Quest, QuestDifficulty, QuestType, StreakManager};
use crate::ui::notification::NotificationType;
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

/// Regression test for #267: completing a quest through the live message-passing
/// flow must populate `completed_quests_today`, since `StreakManager::update_streak`
/// gates next-day streak increments on that set being non-empty.
#[test]
fn test_quest_completion_populates_completed_quests_today() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

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

    assert!(state.progress.profile.completed_quests_today.is_empty());

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

    assert!(
        state
            .progress
            .profile
            .completed_quests_today
            .contains("test_quest")
    );
}

/// Regression test for #267: the streak can now increment on the next session because
/// `completed_quests_today` was populated by a live quest completion the day before.
#[test]
fn test_streak_increments_next_day_after_live_quest_completion() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    state.progress.profile.current_streak = 3;

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

    // Simulate that this activity happened yesterday, as `StreakManager` would see it
    // at the start of the next session.
    state.progress.profile.last_activity = chrono::Utc::now() - chrono::Duration::days(1);

    let change = StreakManager::update_streak(&mut state.progress.profile);

    assert_eq!(
        change,
        crate::gamification::StreakChange::Incremented { new_streak: 4 }
    );
    assert_eq!(state.progress.profile.current_streak, 4);
}

/// Regression test for #257: completing *all* of today's daily quests through the live
/// flow grants a streak freeze. Eligibility is based on completing every quest generated
/// for the day, not a fixed count — the quest generator produces at most 4 quests/day, so
/// the old fixed threshold of 5 was unreachable in production.
#[test]
fn test_streak_freeze_granted_after_all_daily_quests_completed_live() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    let quest_ids = ["quest_0", "quest_1", "quest_2"];
    for id in quest_ids {
        state.progress.profile.daily_quests.push(Quest::new(
            id.to_string(),
            QuestType::CommandPractice {
                command: id.to_string(),
                target: 1,
                current: 0,
            },
            format!("Use {}", id),
            QuestDifficulty::Easy,
        ));
    }

    for (i, id) in quest_ids.iter().enumerate() {
        update(
            &mut state,
            Message::UpdateQuestProgress {
                command: Some(id.to_string()),
                scenario_completed: false,
                scenario_id: None,
                duration: Duration::from_secs(0),
            },
        )
        .unwrap();

        // Freeze must not be granted until every daily quest is completed
        let is_last = i == quest_ids.len() - 1;
        assert_eq!(state.progress.profile.streak_freeze_available, is_last);
    }

    assert!(
        state
            .ui
            .notifications
            .visible()
            .iter()
            .any(|n| n.notification_type == NotificationType::StreakFreezeGranted)
    );
}

/// A freeze already held must not be granted (or notified) again, even if the
/// "all daily quests completed" condition is met again.
#[test]
fn test_streak_freeze_not_granted_twice() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    state.progress.profile.streak_freeze_available = true;
    state.progress.profile.daily_quests.push(Quest::new(
        "quest_0".to_string(),
        QuestType::CommandPractice {
            command: "x".to_string(),
            target: 1,
            current: 0,
        },
        "Use x".to_string(),
        QuestDifficulty::Easy,
    ));

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

    assert!(state.progress.profile.streak_freeze_available);
    assert!(
        !state
            .ui
            .notifications
            .visible()
            .iter()
            .any(|n| n.notification_type == NotificationType::StreakFreezeGranted)
    );
}
