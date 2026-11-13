//! Integration tests for gamification system

use helix_trainer::gamification::*;
use helix_trainer::learning::PerformanceTracker;
use helix_trainer::ui::text;
use std::assert_matches;
use std::time::Duration;
use tempfile::TempDir;

fn ncmd(raw: &str) -> helix_trainer::ids::NormalizedCommand {
    helix_trainer::ids::NormalizedCommand::from_raw(raw).expect("valid command fixture")
}

fn qid(raw: &str) -> helix_trainer::ids::QuestId {
    helix_trainer::ids::QuestId::parse(raw).expect("valid quest id fixture")
}

fn sid(raw: &str) -> helix_trainer::ids::ScenarioId {
    helix_trainer::ids::ScenarioId::parse(raw).expect("valid scenario id fixture")
}

/// Push a quest for today and mark it completed and rewarded.
fn push_rewarded_quest(profile: &mut UserProfile, id: &str) {
    profile.daily_quests.push(Quest::new(
        qid(id),
        QuestType::ScenarioCompletion {
            target: 1,
            current: 1,
        },
        QuestDifficulty::Easy,
    ));
    assert!(profile.complete_quest(&qid(id)));
}

/// Helper to create a registry for tests
fn test_registry() -> QuestTemplateRegistry {
    QuestTemplateRegistry::from_embedded("en").expect("Failed to load quest templates for tests")
}

#[test]
fn test_complete_user_flow() {
    // Test a complete user journey
    let mut profile = UserProfile::new();
    let tracker = PerformanceTracker::new();
    let registry = test_registry();

    // Day 1: Generate quests
    let mut quests = QuestGenerator::generate_quests(
        &profile,
        &registry,
        helix_trainer::time::Moment::from(chrono::Utc::now()).day(),
    )
    .quests;
    assert_eq!(quests.len(), 3); // Beginner gets 3 quests

    // Complete a command quest
    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("x"),
    ));
    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("x"),
    ));
    let completed = QuestTracker::update_command_progress(&mut quests, &ncmd("x"));

    // Award the completed quest
    profile.daily_quests = quests;
    if let Some(id) = completed.first() {
        let reward = profile
            .daily_quests
            .iter()
            .find(|quest| quest.id == *id)
            .map(|quest| quest.xp_reward)
            .unwrap();
        profile.add_xp(u64::from(reward));
        assert!(profile.complete_quest(id));
    }

    // Update streak
    let change = StreakManager::update_streak(&mut profile, chrono::Utc::now());
    assert_matches!(
        change,
        StreakChange::Continued | StreakChange::Incremented { .. }
    );

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
    let registry = test_registry();

    // Level 1: Easy quests
    let mut profile = UserProfile::new();
    profile.level = 1;
    let quests = QuestGenerator::generate_quests(
        &profile,
        &registry,
        helix_trainer::time::Moment::from(chrono::Utc::now()).day(),
    )
    .quests;
    assert_eq!(quests.len(), 3); // 2 easy + 1 medium

    // Level 10: Mixed difficulty
    profile.level = 10;
    let quests = QuestGenerator::generate_quests(
        &profile,
        &registry,
        helix_trainer::time::Moment::from(chrono::Utc::now()).day(),
    )
    .quests;
    assert_eq!(quests.len(), 4); // 1 easy + 2 medium + 1 hard

    // Level 20: Hard quests
    profile.level = 20;
    let quests = QuestGenerator::generate_quests(
        &profile,
        &registry,
        helix_trainer::time::Moment::from(chrono::Utc::now()).day(),
    )
    .quests;
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
    let profile = UserProfile::new();
    let registry = test_registry();

    // Generate multiple times and check variety
    let today = helix_trainer::time::Moment::from(chrono::Utc::now()).day();
    let quests1 = QuestGenerator::generate_quests(&profile, &registry, today).quests;
    let quests2 = QuestGenerator::generate_quests(&profile, &registry, today).quests;

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
    assert_eq!(
        unlocked,
        [] as [helix_trainer::gamification::AchievementId; 0]
    );

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
    use helix_trainer::time::{Clock, FakeClock};

    let clock = FakeClock::at("2026-01-15T12:00:00Z");
    let mut profile = UserProfile::new_at(clock.now());
    profile.current_streak = 10;
    profile.streak_freeze_available = true;

    // Simulate missing a day
    clock.advance_days(2);

    let change = StreakManager::update_streak(&mut profile, clock.now());
    assert_eq!(change, StreakChange::Protected);
    assert_eq!(profile.current_streak, 10); // Preserved
    assert!(!profile.streak_freeze_available); // Used up
}

#[test]
fn test_quest_progress_tracking() {
    let mut quests = vec![Quest::new(
        qid("test"),
        QuestType::CommandPractice {
            command: ncmd("x"),
            target: 5,
            current: 0,
        },
        QuestDifficulty::Easy,
    )];

    // Progress through quest
    for i in 1..=5 {
        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd("x"),
        ));
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
        command: ncmd("x"),
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
    push_rewarded_quest(&mut profile, "quest_1");

    storage.save(&profile).unwrap();

    // Load and verify
    let loaded = storage.load().unwrap();
    assert_eq!(loaded.level, profile.level);
    assert_eq!(loaded.total_xp, profile.total_xp);
    assert_eq!(loaded.current_streak, profile.current_streak);
    assert_eq!(loaded.perfect_scenarios, profile.perfect_scenarios);
    assert!(loaded.has_achievement(&AchievementId::FirstPerfect));
    assert!(loaded.is_quest_completed(&qid("quest_1")));
}

#[test]
fn test_daily_quest_reset() {
    let mut profile = UserProfile::new();

    // Complete some quests
    push_rewarded_quest(&mut profile, "quest_1");
    push_rewarded_quest(&mut profile, "quest_2");
    assert_eq!(profile.completed_quests_today().count(), 2);

    // Reset daily quests
    profile.reset_daily_quests(chrono::Utc::now());
    assert_eq!(profile.completed_quests_today().count(), 0);
    assert_eq!(profile.daily_quests.len(), 0);
}

#[test]
fn test_exploration_quest_tracking() {
    let mut quests = vec![Quest::new(
        qid("exploration"),
        QuestType::Exploration {
            target_commands: 5,
            commands_used: std::collections::HashSet::new(),
        },
        QuestDifficulty::Hard,
    )];

    // Use different commands
    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("x"),
    ));
    assert!(!quests[0].is_completed());

    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("yy"),
    ));
    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("p"),
    ));
    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("w"),
    ));
    assert!(!quests[0].is_completed());

    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("i"),
    ));
    assert!(quests[0].is_completed());

    // Duplicate commands don't count
    drop(QuestTracker::update_command_progress(
        &mut quests,
        &ncmd("x"),
    ));
    if let QuestType::Exploration { commands_used, .. } = &quests[0].quest_type {
        assert_eq!(commands_used.len(), 5);
    }
}

#[test]
fn test_speed_run_quest() {
    let mut quests = vec![Quest::new(
        qid("speed"),
        QuestType::SpeedRun {
            scenario_id: sid("delete_line_001"),
            time_limit: Duration::from_secs(5),
        },
        QuestDifficulty::Hard,
    )];

    // Too slow
    drop(QuestTracker::update_scenario_progress(
        &mut quests,
        &sid("delete_line_001"),
        Duration::from_secs(10),
    ));
    assert!(!quests[0].is_completed());

    // Fast enough
    drop(QuestTracker::update_scenario_progress(
        &mut quests,
        &sid("delete_line_001"),
        Duration::from_secs(3),
    ));
    assert!(quests[0].is_completed());
}

#[test]
fn test_time_invested_quest() {
    let mut quests = vec![Quest::new(
        qid("time"),
        QuestType::TimeInvested {
            target_minutes: 10,
            current_minutes: 0,
        },
        QuestDifficulty::Medium,
    )];

    // Partial progress
    drop(QuestTracker::update_time_progress(&mut quests, 5));
    assert!(!quests[0].is_completed());
    assert_eq!(quests[0].progress(), 0.5);

    // Complete
    drop(QuestTracker::update_time_progress(&mut quests, 5));
    assert!(quests[0].is_completed());
    assert_eq!(quests[0].progress(), 1.0);
}

#[test]
fn test_all_achievement_metadata() {
    let achievements = AchievementEngine::all_achievements();

    // Verify all achievements have metadata
    for achievement in achievements {
        assert!(!text::achievement_name(achievement.id).starts_with("achievements."));
        assert!(!text::achievement_description(achievement.id).starts_with("achievements."));
        assert!(!achievement.is_unlocked());
    }
}

#[test]
fn test_training_base_xp_calculation() {
    let base = |score| XPCalculator::training_xp_components(score, false, false).base_xp;
    assert_eq!(base(100), 50);
    assert_eq!(base(85), 42);
    assert_eq!(base(50), 25);
    assert_eq!(base(0), 0);
}

#[test]
fn test_freeze_eligibility() {
    let mut profile = UserProfile::new();

    // Not eligible initially - no quests generated for today
    assert!(!StreakManager::check_freeze_eligibility(&profile));

    // Generate today's quests
    for i in 0..3 {
        profile.daily_quests.push(Quest::new(
            qid(&format!("quest_{i}")),
            QuestType::CommandPractice {
                command: ncmd("x"),
                target: 1,
                current: 0,
            },
            QuestDifficulty::Easy,
        ));
    }

    // Complete all of today's quests
    for i in 0..3 {
        assert!(profile.complete_quest(&qid(&format!("quest_{i}"))));
    }

    // Now eligible - all of today's quests completed
    assert!(StreakManager::check_freeze_eligibility(&profile));

    // Grant freeze
    StreakManager::grant_freeze(&mut profile);

    // No longer eligible (already has freeze)
    assert!(!StreakManager::check_freeze_eligibility(&profile));
}

#[test]
fn test_longest_streak_tracking() {
    use helix_trainer::time::{Clock, FakeClock};

    let clock = FakeClock::at("2026-01-15T12:00:00Z");
    let mut profile = UserProfile::new_at(clock.now());
    profile.current_streak = 5;
    profile.longest_streak = 5;

    // Increment streak
    push_rewarded_quest(&mut profile, "test");
    clock.advance_days(1);
    StreakManager::update_streak(&mut profile, clock.now());

    assert_eq!(profile.current_streak, 6);
    assert_eq!(profile.longest_streak, 6);

    // Break streak
    clock.advance_days(2);
    StreakManager::update_streak(&mut profile, clock.now());

    assert_eq!(profile.current_streak, 0);
    assert_eq!(profile.longest_streak, 6); // Preserved
}
