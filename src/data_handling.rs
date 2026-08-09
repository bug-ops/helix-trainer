//! Data loading message handling
//!
//! Processes messages from background data loaders and updates app state.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::time::Duration;

use helix_trainer::{
    async_state::DataLoadMessage,
    config::{ScenarioCollection, keymap::keymap_fingerprint_mismatch_message},
    gamification::{
        Achievement, AchievementEngine, LockStatus, QuestGenerator, QuestTemplateRegistry,
        StreakChange, StreakManager, UserProfile,
    },
    learning::PerformanceTracker,
    ui::{AppState, Notification, NotificationType},
};

/// How long a data-loss-risk warning notification stays visible.
///
/// Longer than the default notification duration (3s) since these warn
/// about possible silent data loss, not a routine gameplay event, and the
/// user needs time to notice and act on it.
const WARNING_NOTIFICATION_DURATION: Duration = Duration::from_secs(8);

/// Check if daily quests should be refreshed
///
/// Returns true if the current date differs from the last quest refresh date.
pub fn should_refresh_quests(profile: &UserProfile, now: DateTime<Utc>) -> bool {
    now.date_naive() != profile.last_quest_refresh.date_naive()
}

/// Warn the user if another live instance already appears to be using this
/// profile — two instances writing the same file independently means
/// "last save wins" silently discards one side's progress.
///
/// Called for every path that establishes `state.progress.profile`
/// (`ProfileReady` and `ProfileError`'s fallback), not just the happy
/// path: the fallback case is the highest-stakes one, since a fresh
/// fallback profile silently clobbering a genuinely live instance's real
/// profile on the next save is exactly the scenario this warning exists
/// to prevent.
fn warn_if_other_instance_running(state: &mut AppState) {
    if let LockStatus::OtherInstanceRunning(pid) = state.progress.storage.check_and_refresh_lock() {
        tracing::warn!(
            other_pid = pid,
            "Another instance of helix-trainer appears to be running"
        );
        state.ui.notifications.push(Notification::with_duration(
            NotificationType::Info {
                message: format!(
                    "Another instance (pid {pid}) appears to be running. \
                     Progress may be overwritten if both instances save."
                ),
            },
            WARNING_NOTIFICATION_DURATION,
        ));
    }
}

/// Compare the resolved gameplay keymap's fingerprint against the one
/// stored on the just-loaded profile, and notify if FSRS review history
/// was recorded under a different mapping.
///
/// Always converges `profile.keymap_fingerprint` to the current overlay's
/// fingerprint afterward, so this only fires once per keymap change, not
/// on every subsequent launch with the same (still-mismatched-from-some-
/// older-history) mapping.
fn warn_keymap_fingerprint_mismatch(state: &mut AppState) {
    if state.config.keymap.is_empty() {
        // Stock keymap - fingerprinting only matters once a custom mapping
        // is active, and clearing a previously-set fingerprint here would
        // spuriously "mismatch" the next time a keymap is re-enabled.
        return;
    }

    let current = state.config.keymap.fingerprint();
    if let Some(stored) = state.progress.profile.keymap_fingerprint
        && stored != current
    {
        state.ui.notifications.push(Notification::with_duration(
            NotificationType::Info {
                message: keymap_fingerprint_mismatch_message(),
            },
            WARNING_NOTIFICATION_DURATION,
        ));
    }
    state.progress.profile.keymap_fingerprint = Some(current);
}

/// Handle messages from background data loaders
///
/// Processes async loading results and updates app state accordingly.
/// Handles scenarios, profiles, quest registry, and save confirmations.
pub fn handle_data_message(state: &mut AppState, msg: DataLoadMessage) -> Result<()> {
    match msg {
        DataLoadMessage::ScenariosReady(scenarios) => {
            let count = scenarios.len();
            state.game.scenario_collection = ScenarioCollection::new(scenarios);
            tracing::info!(count, "Scenarios loaded");
        }

        DataLoadMessage::ScenariosError(err) => {
            tracing::error!("Failed to load scenarios: {}", err);
        }

        DataLoadMessage::ProfileReady(profile) => {
            // Single `now` read shared by every day-boundary decision below (streak,
            // quest refresh, daily reset). Previously each check called Utc::now()
            // independently, which could straddle a midnight boundary; this also fixes
            // a latent race where the streak and quest-refresh checks could disagree
            // about what day it is.
            let now = state.progress.now();

            // Update streak and refresh quests if needed
            let mut updated_profile = profile;
            let streak_change = StreakManager::update_streak(&mut updated_profile, now);
            tracing::debug!("Streak status: {:?}", streak_change);
            match streak_change {
                StreakChange::Protected => {
                    state
                        .ui
                        .notifications
                        .push(Notification::new(NotificationType::StreakFreezeUsed));
                }
                StreakChange::Broken {
                    was_streak,
                    freeze_could_not_cover_gap,
                } if was_streak > 0 => {
                    state.ui.notifications.push(Notification::new(
                        NotificationType::StreakBroken {
                            was_streak,
                            freeze_could_not_cover_gap,
                        },
                    ));
                }
                _ => {}
            }

            // Check if we need to refresh daily quests
            if should_refresh_quests(&updated_profile, now)
                || updated_profile.daily_quests.is_empty()
            {
                tracing::info!("Refreshing daily quests for new day");
                let tracker = PerformanceTracker::new();
                updated_profile.reset_daily_quests(now);

                // Load quest registry synchronously
                let quest_registry = QuestTemplateRegistry::load_from_default_path("en")
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "Failed to load quest templates: {}, using empty registry",
                            e
                        );
                        QuestTemplateRegistry::new()
                    });

                updated_profile.daily_quests = QuestGenerator::generate_quests(
                    &updated_profile,
                    &tracker,
                    &quest_registry,
                    now.date_naive(),
                );
            }

            state.progress.performance_tracker = PerformanceTracker::from_stats_with_clock(
                updated_profile.performance_data.clone(),
                state.progress.clock(),
            );
            state.progress.profile = updated_profile;

            warn_keymap_fingerprint_mismatch(state);

            warn_if_other_instance_running(state);

            // Streak (and possibly quest history) may have just changed above;
            // surface any achievements that are now satisfied.
            let newly_unlocked = AchievementEngine::check_and_unlock(
                &mut state.progress.profile,
                &state.progress.performance_tracker,
            );
            if !newly_unlocked.is_empty() {
                for achievement_id in newly_unlocked {
                    let achievement = Achievement::new(achievement_id);
                    state
                        .ui
                        .notifications
                        .push(Notification::new(NotificationType::Achievement {
                            name: achievement.name,
                            description: achievement.description,
                        }));
                }
                state.progress.storage.save(&state.progress.profile)?;
                state.progress.mark_saved();
            }

            tracing::info!("Profile loaded");
        }

        DataLoadMessage::ProfileError { error, fallback } => {
            state.progress.profile = fallback;
            tracing::warn!("Profile load failed, using default: {}", error);
            warn_if_other_instance_running(state);
        }

        DataLoadMessage::QuestRegistryReady(_registry) => {
            tracing::debug!("Quest registry loaded");
        }

        DataLoadMessage::QuestRegistryError(err) => {
            tracing::error!("Failed to load quest registry: {}", err);
        }

        DataLoadMessage::ProfileSaved => {
            state.progress.mark_saved();
            tracing::debug!("Profile saved");
        }

        DataLoadMessage::ProfileSaveError(err) => {
            tracing::error!("Failed to save profile: {}", err);
            // The optimistic `mark_saved()` made when the save was
            // dispatched was wrong: re-dirty so the next debounced or
            // immediate save retries, instead of the debounce gate
            // treating this failure as a successful save indefinitely.
            state.progress.mark_unsaved();
            state.ui.notifications.push(Notification::with_duration(
                NotificationType::Info {
                    message: format!("Failed to save progress: {err}. Will retry automatically."),
                },
                WARNING_NOTIFICATION_DURATION,
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_trainer::{
        config::{CursorSpec, Scenario, ScoringConfig, Setup, Solution, TargetState},
        constants::STREAK_FREEZE_MAX_GAP_DAYS,
        gamification::ProfileStorage,
    };

    // Binary crate tests cannot access the library's #[cfg(test)]-gated
    // `ProfileStorage::for_test()`, so we mirror it locally here: a unique file
    // under a process-lifetime temp dir, never the real user profile path.
    fn test_profile_storage() -> ProfileStorage {
        use std::sync::OnceLock;
        use std::sync::atomic::{AtomicU64, Ordering};

        static TEST_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
        static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

        let dir = TEST_DIR.get_or_init(|| tempfile::TempDir::new().expect("create test temp dir"));
        let id = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        ProfileStorage::with_path(dir.path().join(format!("profile-{id}.json")))
    }

    fn empty_test_app_state() -> AppState {
        let profile = UserProfile::new();
        let storage = test_profile_storage();
        let tracker = PerformanceTracker::new();
        AppState::new(vec![], profile, storage, tracker)
    }

    fn default_test_scenario() -> Scenario {
        Scenario {
            id: "test_scenario".to_string(),
            name: "Test Scenario".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\nline 3\n".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            target: TargetState {
                file_content: "line 2\nline 3\n".to_string(),
                cursor: CursorSpec {
                    cursor_position: Some((0, 0)),
                    selection: None,
                    cursors: None,
                    selections: None,
                },
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Delete first line".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(2).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: None,
        }
    }

    #[test]
    fn test_handle_scenarios_ready() {
        let mut state = empty_test_app_state();
        let scenarios = vec![default_test_scenario()];

        let result = handle_data_message(&mut state, DataLoadMessage::ScenariosReady(scenarios));

        assert!(result.is_ok());
        assert_eq!(state.game.scenario_collection.count(), 1);
    }

    #[test]
    fn test_handle_scenarios_ready_empty() {
        let mut state = empty_test_app_state();

        let result = handle_data_message(&mut state, DataLoadMessage::ScenariosReady(vec![]));

        assert!(result.is_ok());
        assert_eq!(state.game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_handle_scenarios_error() {
        let mut state = empty_test_app_state();

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::ScenariosError("File not found".to_string()),
        );

        // Should not panic, just log error
        assert!(result.is_ok());
        // Scenarios should remain empty
        assert_eq!(state.game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_handle_profile_ready() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.total_xp = 500;
        profile.level = 3;

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        let loaded_profile = &state.progress.profile;
        assert_eq!(loaded_profile.total_xp, 500);
        assert_eq!(loaded_profile.level, 3);
    }

    fn app_state_with_keymap() -> AppState {
        use helix_trainer::config::keymap::resolve_str;
        use helix_trainer::ui::state::ConfigState;

        let (keymap, _) = resolve_str(
            r#"
            [keys.normal]
            j = "move_char_left"
            "#,
        )
        .unwrap();
        assert!(!keymap.is_empty());

        AppState::with_config(
            vec![],
            UserProfile::new(),
            test_profile_storage(),
            PerformanceTracker::new(),
            ConfigState {
                keymap,
                ..ConfigState::default()
            },
        )
    }

    /// A stale `keymap_fingerprint` (recorded under a different mapping)
    /// must notify when the currently active keymap doesn't match.
    #[test]
    fn test_handle_profile_ready_notifies_on_keymap_fingerprint_mismatch() {
        let mut state = app_state_with_keymap();
        let current_fingerprint = state.config.keymap.fingerprint();
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = Some(current_fingerprint.wrapping_add(1));

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile)).unwrap();

        assert!(
            state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| matches!(&n.notification_type, NotificationType::Info { message }
                    if message == &helix_trainer::config::keymap::keymap_fingerprint_mismatch_message())),
            "expected a keymap fingerprint mismatch notification"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(current_fingerprint),
            "fingerprint must converge to the current keymap after notifying"
        );
    }

    /// First-ever activation of a keymap (`keymap_fingerprint` is `None`)
    /// must not be treated as a mismatch - there's no prior mapping to
    /// have diverged from.
    #[test]
    fn test_handle_profile_ready_no_notification_on_first_keymap_activation() {
        let mut state = app_state_with_keymap();
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = None;

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile)).unwrap();

        assert!(
            !state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| matches!(n.notification_type, NotificationType::Info { .. })),
            "first-time keymap activation must not notify a mismatch"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(state.config.keymap.fingerprint())
        );
    }

    /// With the stock keymap (no overlay), a stale `keymap_fingerprint` on
    /// the loaded profile must be left untouched and never notified -
    /// fingerprinting only matters once a custom mapping is active.
    #[test]
    fn test_handle_profile_ready_no_keymap_notification_when_disabled() {
        let mut state = empty_test_app_state();
        assert!(state.config.keymap.is_empty());
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = Some(42);

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile)).unwrap();

        assert!(
            !state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| matches!(n.notification_type, NotificationType::Info { .. })),
            "stock keymap must never produce a fingerprint mismatch notification"
        );
        assert_eq!(state.progress.profile.keymap_fingerprint, Some(42));
    }

    /// Regression test for #256: a streak that crosses a milestone at load time (the only
    /// place `StreakManager::update_streak` runs) must unlock the corresponding achievement
    /// and notify the user, not just update `current_streak`.
    #[test]
    fn test_handle_profile_ready_unlocks_streak_achievement() {
        use helix_trainer::gamification::AchievementId;
        use helix_trainer::time::{Clock, FakeClock};
        use helix_trainer::ui::state::{AppState, ConfigState};
        use std::sync::Arc;

        // Construct AppState via with_clock directly (rather than building it with the
        // default SystemClock and reassigning `progress.clock` afterward) so the injected
        // clock is shared by every clock-consuming field from the start, not just read back.
        let clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
        let mut state = AppState::with_clock(
            vec![],
            UserProfile::new(),
            test_profile_storage(),
            PerformanceTracker::new(),
            ConfigState::default(),
            clock.clone(),
        );

        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 6;
        profile.completed_quests_today.insert("quest_0".to_string());
        clock.advance_days(1);

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        // Streak should have incremented from 6 to 7, crossing the Streak7Days threshold
        assert_eq!(state.progress.profile.current_streak, 7);
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::Streak7Days)
        );
        assert!(state.ui.notifications.visible().iter().any(|n| matches!(
            n.notification_type,
            NotificationType::Achievement { ref name, .. } if name == "7-Day Warrior"
        )));
    }

    /// Regression test for #298: if another live process already holds the
    /// PID lock for this profile, loading the profile must warn the user
    /// instead of silently proceeding as if only one instance were running.
    #[cfg(unix)]
    #[test]
    fn test_handle_profile_ready_warns_on_other_live_instance() {
        let storage = test_profile_storage();
        let lock_path = storage.path().with_file_name(format!(
            "{}.lock",
            storage.path().file_name().unwrap().to_string_lossy()
        ));

        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let other_pid = child.id();
        std::fs::write(&lock_path, other_pid.to_string()).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            storage,
            PerformanceTracker::new(),
        );

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );

        let _ = child.kill();
        let _ = child.wait();

        assert!(result.is_ok());
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                &n.notification_type,
                NotificationType::Info { message } if message.contains(&other_pid.to_string())
            )),
            "expected a notification warning about the other running instance"
        );
    }

    /// Regression test for #292 finding 2: consuming a streak freeze to protect a missed
    /// day was previously silent (logged only). It must now push a
    /// `NotificationType::StreakFreezeUsed` notification, mirroring the existing
    /// `StreakFreezeGranted` notification pushed when a freeze is earned.
    #[test]
    fn test_handle_profile_ready_notifies_on_streak_freeze_used() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.current_streak = 5;
        profile.streak_freeze_available = true;
        // Missed more than one day, so `update_streak` takes the "missed day(s)" branch.
        profile.last_activity = Utc::now() - chrono::Duration::days(2);

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        assert!(!state.progress.profile.streak_freeze_available);
        assert_eq!(state.progress.profile.current_streak, 5);
        assert!(
            state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| n.notification_type == NotificationType::StreakFreezeUsed),
            "expected a StreakFreezeUsed notification when a freeze protects a missed day"
        );
    }

    #[test]
    fn test_handle_profile_ready_notifies_on_streak_broken() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.current_streak = 5;
        profile.streak_freeze_available = false;
        // Missed more than one day with no freeze available, so `update_streak` breaks
        // the streak instead of protecting it.
        profile.last_activity = Utc::now() - chrono::Duration::days(2);

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| n.notification_type
                    == NotificationType::StreakBroken {
                        was_streak: 5,
                        freeze_could_not_cover_gap: false
                    }),
            "expected a StreakBroken notification when a streak breaks with no freeze available"
        );
    }

    /// Regression test for #345: when a gap exceeds the freeze's coverage cap
    /// even though a freeze was held, the freeze stays held (per #325/#344)
    /// and the resulting `StreakBroken` notification must flag that the
    /// freeze couldn't cover this specific gap, rather than reading like a
    /// plain missed-day break.
    #[test]
    fn test_handle_profile_ready_notifies_on_streak_broken_beyond_freeze_cap() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.current_streak = 5;
        profile.streak_freeze_available = true;
        // Gap exceeds the freeze coverage cap, so the freeze can't protect it.
        profile.last_activity = Utc::now() - chrono::Duration::days(STREAK_FREEZE_MAX_GAP_DAYS + 1);

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            state.progress.profile.streak_freeze_available,
            "freeze must remain held since it couldn't cover this gap"
        );
        assert!(
            state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| n.notification_type
                    == NotificationType::StreakBroken {
                        was_streak: 5,
                        freeze_could_not_cover_gap: true
                    }),
            "expected a StreakBroken notification flagging that a freeze was held but insufficient"
        );
    }

    #[test]
    fn test_handle_profile_ready_no_notification_for_never_started_streak() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        // current_streak is already 0 - nothing to lose, so breaking it should stay silent.
        profile.last_activity = Utc::now() - chrono::Duration::days(2);

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            !state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| matches!(n.notification_type, NotificationType::StreakBroken { .. })),
            "no streak was ever active, so no StreakBroken notification should fire"
        );
    }

    /// Regression test for #319: a freeze must not be consumed (and no
    /// `StreakFreezeUsed` notification pushed) when `current_streak` is already 0 -
    /// there is nothing to protect, even though a freeze is held.
    #[test]
    fn test_handle_profile_ready_no_freeze_used_for_never_started_streak() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.streak_freeze_available = true;
        // current_streak is already 0 - nothing to protect.
        profile.last_activity = Utc::now() - chrono::Duration::days(2);

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(result.is_ok());
        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            state.progress.profile.streak_freeze_available,
            "freeze must not be consumed when there was no streak to protect"
        );
        assert!(
            !state
                .ui
                .notifications
                .visible()
                .iter()
                .any(|n| n.notification_type == NotificationType::StreakFreezeUsed),
            "no streak was ever active, so no StreakFreezeUsed notification should fire"
        );
    }

    #[test]
    fn test_handle_profile_error_uses_fallback() {
        let mut state = empty_test_app_state();
        let mut fallback = UserProfile::new();
        fallback.total_xp = 100; // Mark fallback with some XP

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::ProfileError {
                error: "Corrupted file".to_string(),
                fallback,
            },
        );

        assert!(result.is_ok());
        let loaded_profile = &state.progress.profile;
        assert_eq!(loaded_profile.total_xp, 100); // Fallback was used
    }

    /// Regression test for a non-blocking finding in the #298 critique: the
    /// `ProfileError` fallback arm is the highest-stakes multi-instance
    /// case (a fresh fallback profile can clobber another live instance's
    /// real profile on the next save), so it must warn too, not just the
    /// `ProfileReady` happy path.
    #[cfg(unix)]
    #[test]
    fn test_handle_profile_error_warns_on_other_live_instance() {
        let storage = test_profile_storage();
        let lock_path = storage.path().with_file_name(format!(
            "{}.lock",
            storage.path().file_name().unwrap().to_string_lossy()
        ));

        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let other_pid = child.id();
        std::fs::write(&lock_path, other_pid.to_string()).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            storage,
            PerformanceTracker::new(),
        );

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::ProfileError {
                error: "Corrupted file".to_string(),
                fallback: UserProfile::new(),
            },
        );

        let _ = child.kill();
        let _ = child.wait();

        assert!(result.is_ok());
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                &n.notification_type,
                NotificationType::Info { message } if message.contains(&other_pid.to_string())
            )),
            "expected a notification warning about the other running instance"
        );
    }

    #[test]
    fn test_handle_profile_saved() {
        let mut state = empty_test_app_state();

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileSaved);

        assert!(result.is_ok());
    }

    /// Regression test for critique finding B2: a failed async save must
    /// not be silently swallowed. Before this fix, `ProfileSaveError` was
    /// only logged — the optimistic `mark_saved()` made at dispatch time
    /// stood uncorrected, so the debounce gate treated the failed write as
    /// a successful one, and the user had no way to know their progress
    /// wasn't actually persisted.
    #[test]
    fn test_handle_profile_save_error() {
        let mut state = empty_test_app_state();
        state.progress.mark_saved();
        assert!(!state.progress.should_save());

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::ProfileSaveError("Disk full".to_string()),
        );

        assert!(result.is_ok());
        assert!(
            state.progress.should_save(),
            "a failed save must re-dirty the profile so the next save retries"
        );
        assert!(
            state.ui.notifications.visible().iter().any(|n| matches!(
                &n.notification_type,
                NotificationType::Info { message } if message.contains("Disk full")
            )),
            "a failed save must surface a user-facing notification"
        );
    }

    #[test]
    fn test_handle_quest_registry_ready() {
        let mut state = empty_test_app_state();
        let registry = QuestTemplateRegistry::new();

        let result = handle_data_message(&mut state, DataLoadMessage::QuestRegistryReady(registry));

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_quest_registry_error() {
        let mut state = empty_test_app_state();

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryError("Invalid TOML".to_string()),
        );

        // Should not panic, just log error
        assert!(result.is_ok());
    }
}
