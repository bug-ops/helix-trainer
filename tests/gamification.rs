//! Integration tests for gamification system

use helix_trainer::gamification::*;
use helix_trainer::learning::PerformanceTracker;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_complete_user_flow() {
    // Test a complete user journey
    let mut profile = UserProfile::new();
    let tracker = PerformanceTracker::new();

    // Day 1: Generate quests
    let mut quests = QuestGenerator::generate_quests(&profile, &tracker);
    assert_eq!(quests.len(), 3); // Beginner gets 3 quests

    // Complete a command quest
    QuestTracker::update_command_progress(&mut quests, "dd");
    QuestTracker::update_command_progress(&mut quests, "dd");
    QuestTracker::update_command_progress(&mut quests, "dd");

    // Check if any quests completed
    let completions = QuestTracker::check_completions(&quests);
    if let Some((quest_id, xp_reward)) = completions.first() {
        profile.add_xp(*xp_reward as u64);
        profile.complete_quest(quest_id.clone());
    }

    // Update streak
    let change = StreakManager::update_streak(&mut profile);
    assert!(matches!(
        change,
        StreakChange::Continued | StreakChange::Incremented { .. }
    ));

    // Check achievements
    profile.perfect_scenarios = 1;
    let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    assert!(unlocked.contains(&AchievementId::FirstPerfect));
}

#[test]
fn test_level_progression() {
    let mut profile = UserProfile::new();

    // Start at level 1
    assert_eq!(profile.level, 1);
    assert_eq!(profile.total_xp, 0);

    // Add XP to reach level 2 (100 XP required)
    let leveled_up = profile.add_xp(100);
    assert!(leveled_up);
    assert_eq!(profile.level, 2);

    // Add more XP to reach level 3 (~183 more XP)
    let leveled_up = profile.add_xp(183);
    assert!(leveled_up);
    assert_eq!(profile.level, 3);

    // Verify XP progress calculation
    let progress = profile.xp_progress();
    assert!((0.0..=1.0).contains(&progress));
}

#[test]
fn test_quest_generation_adapts_to_level() {
    let tracker = PerformanceTracker::new();

    // Level 1: Easy quests
    let mut profile = UserProfile::new();
    profile.level = 1;
    let quests = QuestGenerator::generate_quests(&profile, &tracker);
    assert_eq!(quests.len(), 3); // 2 easy + 1 medium

    // Level 10: Mixed difficulty
    profile.level = 10;
    let quests = QuestGenerator::generate_quests(&profile, &tracker);
    assert_eq!(quests.len(), 4); // 1 easy + 2 medium + 1 hard

    // Level 20: Hard quests
    profile.level = 20;
    let quests = QuestGenerator::generate_quests(&profile, &tracker);
    assert_eq!(quests.len(), 4); // 1 medium + 2 hard + 1 exploration
}

#[test]
fn test_streak_milestone_rewards() {
    let mut profile = UserProfile::new();

    // Reach 7-day streak milestone
    profile.current_streak = 7;
    let bonus = StreakManager::milestone_xp_bonus(7);
    assert_eq!(bonus, 50);

    profile.add_xp(bonus);

    // Check achievement unlocked
    let tracker = PerformanceTracker::new();
    let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    assert!(unlocked.contains(&AchievementId::Streak7Days));
}

#[test]
fn test_quest_type_variations() {
    let tracker = PerformanceTracker::new();
    let profile = UserProfile::new();

    // Generate multiple times and check variety
    let quests1 = QuestGenerator::generate_quests(&profile, &tracker);
    let quests2 = QuestGenerator::generate_quests(&profile, &tracker);

    // Since they use the same date seed, they should be identical
    assert_eq!(quests1.len(), quests2.len());
    for (q1, q2) in quests1.iter().zip(quests2.iter()) {
        assert_eq!(q1.id, q2.id);
        assert_eq!(q1.quest_type, q2.quest_type);
    }
}

#[test]
fn test_achievement_progression() {
    let tracker = PerformanceTracker::new();
    let mut profile = UserProfile::new();

    // No achievements initially
    let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    assert!(unlocked.is_empty());

    // Unlock FirstPerfect
    profile.perfect_scenarios = 1;
    let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    assert!(unlocked.contains(&AchievementId::FirstPerfect));
    profile.unlock_achievement(AchievementId::FirstPerfect);

    // Unlock Perfect10
    profile.perfect_scenarios = 10;
    let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    assert!(unlocked.contains(&AchievementId::Perfect10));
    assert!(!unlocked.contains(&AchievementId::FirstPerfect)); // Already unlocked

    // Unlock scenario milestones
    profile.scenarios_completed = 100;
    let unlocked = AchievementEngine::check_achievements(&profile, &tracker);
    assert!(unlocked.contains(&AchievementId::Centurion));
}

#[test]
fn test_streak_freeze_mechanics() {
    let mut profile = UserProfile::new();
    profile.current_streak = 10;
    profile.streak_freeze_available = true;

    // Simulate missing a day
    profile.last_activity = chrono::Utc::now() - chrono::Duration::days(2);

    let change = StreakManager::update_streak(&mut profile);
    assert_eq!(change, StreakChange::Protected { used_freeze: true });
    assert_eq!(profile.current_streak, 10); // Preserved
    assert!(!profile.streak_freeze_available); // Used up
}

#[test]
fn test_quest_progress_tracking() {
    let mut quests = vec![Quest::new(
        "test".to_string(),
        QuestType::CommandPractice {
            command: "dd".to_string(),
            target: 5,
            current: 0,
        },
        "Practice dd 5 times".to_string(),
        QuestDifficulty::Easy,
    )];

    // Progress through quest
    for i in 1..=5 {
        QuestTracker::update_command_progress(&mut quests, "dd");
        if i < 5 {
            assert!(!quests[0].is_completed());
        }
    }

    assert!(quests[0].is_completed());
    assert_eq!(quests[0].progress(), 1.0);
}

#[test]
fn test_xp_calculator_formulas() {
    // Test level curve
    assert_eq!(XPCalculator::xp_for_level(1), 0);
    assert_eq!(XPCalculator::xp_for_level(2), 100);

    // Test roundtrip
    for level in 1..20 {
        let xp = XPCalculator::xp_for_level(level);
        let calculated_level = XPCalculator::level_from_xp(xp);
        assert_eq!(calculated_level, level);
    }

    // Test quest XP rewards
    let easy_command = QuestType::CommandPractice {
        command: "dd".to_string(),
        target: 5,
        current: 0,
    };
    assert_eq!(
        XPCalculator::quest_xp_reward(&easy_command, QuestDifficulty::Easy),
        25
    );

    let hard_scenario = QuestType::ScenarioCompletion {
        target: 3,
        current: 0,
    };
    assert_eq!(
        XPCalculator::quest_xp_reward(&hard_scenario, QuestDifficulty::Hard),
        150
    );
}

#[test]
fn test_storage_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("profile.json");
    let storage = ProfileStorage::with_path(&file_path);

    // Create and save profile
    let mut profile = UserProfile::new();
    profile.add_xp(500);
    profile.current_streak = 10;
    profile.perfect_scenarios = 5;
    profile.unlock_achievement(AchievementId::FirstPerfect);
    profile.complete_quest("quest_1".to_string());

    storage.save(&profile).unwrap();

    // Load and verify
    let loaded = storage.load().unwrap();
    assert_eq!(loaded.level, profile.level);
    assert_eq!(loaded.total_xp, profile.total_xp);
    assert_eq!(loaded.current_streak, profile.current_streak);
    assert_eq!(loaded.perfect_scenarios, profile.perfect_scenarios);
    assert!(loaded.has_achievement(&AchievementId::FirstPerfect));
    assert!(loaded.is_quest_completed("quest_1"));
}

#[test]
fn test_daily_quest_reset() {
    let mut profile = UserProfile::new();

    // Complete some quests
    profile.complete_quest("quest_1".to_string());
    profile.complete_quest("quest_2".to_string());
    assert_eq!(profile.completed_quests_today.len(), 2);

    // Reset daily quests
    profile.reset_daily_quests();
    assert_eq!(profile.completed_quests_today.len(), 0);
    assert_eq!(profile.daily_quests.len(), 0);
}

#[test]
fn test_exploration_quest_tracking() {
    let mut quests = vec![Quest::new(
        "exploration".to_string(),
        QuestType::Exploration {
            target_commands: 5,
            commands_used: std::collections::HashSet::new(),
        },
        "Use 5 different commands".to_string(),
        QuestDifficulty::Hard,
    )];

    // Use different commands
    QuestTracker::update_command_progress(&mut quests, "dd");
    assert!(!quests[0].is_completed());

    QuestTracker::update_command_progress(&mut quests, "yy");
    QuestTracker::update_command_progress(&mut quests, "p");
    QuestTracker::update_command_progress(&mut quests, "x");
    assert!(!quests[0].is_completed());

    QuestTracker::update_command_progress(&mut quests, "i");
    assert!(quests[0].is_completed());

    // Duplicate commands don't count
    QuestTracker::update_command_progress(&mut quests, "dd");
    if let QuestType::Exploration { commands_used, .. } = &quests[0].quest_type {
        assert_eq!(commands_used.len(), 5);
    }
}

#[test]
fn test_speed_run_quest() {
    let mut quests = vec![Quest::new(
        "speed".to_string(),
        QuestType::SpeedRun {
            scenario_id: "delete_line_001".to_string(),
            time_limit: Duration::from_secs(5),
        },
        "Complete scenario in under 5 seconds".to_string(),
        QuestDifficulty::Hard,
    )];

    // Too slow
    QuestTracker::update_scenario_progress(&mut quests, "delete_line_001", Duration::from_secs(10));
    assert!(!quests[0].is_completed());

    // Fast enough
    QuestTracker::update_scenario_progress(&mut quests, "delete_line_001", Duration::from_secs(3));
    assert!(quests[0].is_completed());
}

#[test]
fn test_time_invested_quest() {
    let mut quests = vec![Quest::new(
        "time".to_string(),
        QuestType::TimeInvested {
            target_minutes: 10,
            current_minutes: 0,
        },
        "Practice for 10 minutes".to_string(),
        QuestDifficulty::Medium,
    )];

    // Partial progress
    QuestTracker::update_time_progress(&mut quests, 5);
    assert!(!quests[0].is_completed());
    assert_eq!(quests[0].progress(), 0.5);

    // Complete
    QuestTracker::update_time_progress(&mut quests, 5);
    assert!(quests[0].is_completed());
    assert_eq!(quests[0].progress(), 1.0);
}

#[test]
fn test_all_achievement_metadata() {
    let achievements = AchievementEngine::all_achievements();

    // Verify all achievements have metadata
    for achievement in achievements {
        assert!(!achievement.name.is_empty());
        assert!(!achievement.description.is_empty());
        assert!(!achievement.is_unlocked());
    }
}

#[test]
fn test_scenario_xp_calculation() {
    // Perfect score, first today
    let xp = XPCalculator::scenario_xp(100, true, true);
    assert_eq!(xp, 34); // (20 * 1.0 * 1.2) + 10 = 34

    // Perfect score, not first
    let xp = XPCalculator::scenario_xp(100, true, false);
    assert_eq!(xp, 24); // 20 * 1.0 * 1.2 = 24

    // 50% score
    let xp = XPCalculator::scenario_xp(50, false, false);
    assert_eq!(xp, 10); // 20 * 0.5 * 1.0 = 10
}

#[test]
fn test_freeze_eligibility() {
    let mut profile = UserProfile::new();

    // Not eligible initially
    assert!(!StreakManager::check_freeze_eligibility(&profile));

    // Complete 5 quests
    for i in 0..5 {
        profile.complete_quest(format!("quest_{}", i));
    }

    // Now eligible
    assert!(StreakManager::check_freeze_eligibility(&profile));

    // Grant freeze
    StreakManager::grant_freeze(&mut profile);

    // No longer eligible (already has freeze)
    assert!(!StreakManager::check_freeze_eligibility(&profile));
}

#[test]
fn test_longest_streak_tracking() {
    let mut profile = UserProfile::new();
    profile.current_streak = 5;
    profile.longest_streak = 5;

    // Increment streak
    profile.last_activity = chrono::Utc::now() - chrono::Duration::days(1);
    profile.complete_quest("test".to_string());
    StreakManager::update_streak(&mut profile);

    assert_eq!(profile.current_streak, 6);
    assert_eq!(profile.longest_streak, 6);

    // Break streak
    profile.last_activity = chrono::Utc::now() - chrono::Duration::days(2);
    StreakManager::update_streak(&mut profile);

    assert_eq!(profile.current_streak, 0);
    assert_eq!(profile.longest_streak, 6); // Preserved
}
