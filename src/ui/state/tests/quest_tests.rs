//! Tests for quest tracking and XP rewards

use crate::testing::{ncmd, qid, sid};
use std::collections::HashSet;
use std::time::Duration;

use super::common::{create_test_app_state, create_test_scenario};
use crate::gamification::{Quest, QuestDifficulty, QuestType, StreakManager};
use crate::testing::update_executing_effects as update;
use crate::ui::notification::NotificationType;
use crate::ui::state::{Message, QuestEvent};

#[test]
fn test_quest_progress_command_practice() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    // Add a CommandPractice quest to profile
    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            qid("test_x"),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 3,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    // Execute "x" command twice
    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
    )
    .unwrap();

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
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
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
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
            qid("test_scenario"),
            QuestType::ScenarioCompletion {
                target: 2,
                current: 0,
            },
            QuestDifficulty::Medium,
        ));
    }

    // Start and "complete" a scenario
    crate::testing::start_scenario_from_menu(&mut state, 0).unwrap();

    // Simulate scenario completion
    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::ScenarioCompleted {
            id: sid("test_scenario"),
            duration: Duration::from_secs(5),
        }),
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
        Message::UpdateQuestProgress(QuestEvent::ScenarioCompleted {
            id: sid("test_scenario"),
            duration: Duration::from_secs(5),
        }),
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
            qid("test_quest"),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 1,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    // Complete the quest
    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
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
            qid("test_exploration"),
            QuestType::Exploration {
                target_commands: 3,
                commands_used: HashSet::new(),
            },
            QuestDifficulty::Hard,
        ));
    }

    // Execute different commands
    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
    )
    .unwrap();

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("yy"))),
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
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("p"))),
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
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
    )
    .unwrap();

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("yy"))),
    )
    .unwrap();

    // Same command again (should not duplicate)
    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
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
            qid("test_quest"),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 3,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    // Execute command to trigger progress
    let quest_update = crate::ui::state::handlers::apply_quest_event(
        &mut state,
        QuestEvent::CommandExecuted(ncmd("x")),
    );

    // Check that progress changes were returned
    assert_eq!(quest_update.changes.len(), 1);
    let change = &quest_update.changes[0];
    assert_eq!(change.old_progress, 0);
    assert_eq!(change.new_progress, 1);
    assert!(change.quest_description.contains('x'));
}

/// Regression test for #267: completing a quest through the live message-passing
/// flow must show in `completed_quests_today`, since `StreakManager::update_streak`
/// gates next-day streak increments on it.
#[test]
fn test_quest_completion_populates_completed_quests_today() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);

    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            qid("test_quest"),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 1,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    assert_eq!(state.progress.profile.completed_quests_today().count(), 0);

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
    )
    .unwrap();

    assert!(
        state
            .progress
            .profile
            .is_quest_completed(&qid("test_quest"))
    );
}

/// Regression test for #267: the streak can now increment on the next session because
/// `completed_quests_today` was populated by a live quest completion the day before.
#[test]
fn test_streak_increments_next_day_after_live_quest_completion() {
    use crate::gamification::UserProfile;
    use crate::testing::TestAppStateBuilder;
    use crate::time::{Clock, FakeClock};
    use std::sync::Arc;

    let scenario = create_test_scenario();
    let clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
    let mut state = TestAppStateBuilder::new()
        .scenario(scenario)
        .profile(UserProfile::new_at(clock.now()))
        .with_clock(clock.clone())
        .build();
    state.progress.profile.current_streak = 3;

    {
        let profile = &mut state.progress.profile;
        profile.daily_quests.push(Quest::new(
            qid("test_quest"),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 1,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
    )
    .unwrap();

    // Advance the injected clock to the next day, as the next session's load would see it,
    // instead of backdating `last_activity` directly.
    clock.advance_days(1);

    let now = state.progress.now();
    let change = StreakManager::update_streak(&mut state.progress.profile, now);

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
            qid(id),
            QuestType::CommandPractice {
                command: ncmd(id),
                target: 1,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    for (i, id) in quest_ids.iter().enumerate() {
        update(
            &mut state,
            Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd(id))),
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
        qid("quest_0"),
        QuestType::CommandPractice {
            command: ncmd("x"),
            target: 1,
            current: 0,
        },
        QuestDifficulty::Easy,
    ));

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::CommandExecuted(ncmd("x"))),
    )
    .unwrap();

    assert!(state.progress.profile.streak_freeze_available);
    assert!(
        !state
            .ui
            .notifications
            .visible()
            .any(|n| n.notification_type == NotificationType::StreakFreezeGranted)
    );
}

/// FR-031/FR-032: a completion's duration feeds time-invested quests.
#[test]
fn test_scenario_completed_event_feeds_time_invested() {
    let scenario = create_test_scenario();
    let mut state = create_test_app_state(vec![scenario]);
    state.progress.profile.daily_quests = vec![Quest::new(
        qid("time"),
        QuestType::TimeInvested {
            target_minutes: 5,
            current_minutes: 0,
        },
        QuestDifficulty::Easy,
    )];

    update(
        &mut state,
        Message::UpdateQuestProgress(QuestEvent::ScenarioCompleted {
            id: sid("test_scenario"),
            duration: Duration::from_secs(90),
        }),
    )
    .unwrap();

    assert!(matches!(
        state.progress.profile.daily_quests[0].quest_type,
        QuestType::TimeInvested {
            current_minutes: 1,
            ..
        }
    ));
}

/// FR-036: two quests completed by one event are both awarded, once each, whatever
/// their order in the list; a later event does not award them again.
#[test]
fn test_double_completion_is_awarded_once_each_regardless_of_order() {
    for reversed in [false, true] {
        let scenario = create_test_scenario();
        let mut state = create_test_app_state(vec![scenario]);
        let mut quests = vec![
            crate::testing::quest(
                qid("first"),
                QuestType::CommandPractice {
                    command: ncmd("x"),
                    target: 1,
                    current: 0,
                },
                40,
            ),
            crate::testing::quest(
                qid("second"),
                QuestType::Exploration {
                    target_commands: 1,
                    commands_used: HashSet::new(),
                },
                60,
            ),
        ];
        if reversed {
            quests.reverse();
        }
        state.progress.profile.daily_quests = quests;
        let xp_before = state.progress.profile.total_xp;

        let first = crate::ui::state::handlers::apply_quest_event(
            &mut state,
            QuestEvent::CommandExecuted(ncmd("x")),
        );
        assert_eq!(first.award.bonuses.len(), 2);
        assert_eq!(state.progress.profile.total_xp, xp_before + 100);

        let again = crate::ui::state::handlers::apply_quest_event(
            &mut state,
            QuestEvent::CommandExecuted(ncmd("x")),
        );
        assert_eq!(again.award.bonuses, vec![]);
        assert_eq!(state.progress.profile.total_xp, xp_before + 100);
    }
}
