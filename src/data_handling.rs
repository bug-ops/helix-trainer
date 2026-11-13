//! Data loading message handling
//!
//! Processes messages from background data loaders and updates app state.

use std::sync::Arc;
use std::time::Duration;

use helix_trainer::{
    async_state::DataLoadMessage,
    game::ScenarioCollection,
    gamification::{
        AchievementEngine, LockStatus, QuarantineOutcome, QuestGenerator, QuestRegistrySlot,
        StreakChange, StreakManager, UserProfile,
    },
    learning::PerformanceTracker,
    security::error_chain,
    time::Moment,
    ui::{AppState, Notification, NotificationType, ProfileBackup, state::Quarantine},
};

/// How long a data-loss-risk warning notification stays visible.
///
/// Longer than the default notification duration (3s) since these warn
/// about possible silent data loss, not a routine gameplay event, and the
/// user needs time to notice and act on it.
const WARNING_NOTIFICATION_DURATION: Duration = Duration::from_secs(8);

/// Short popup text for a failed profile load; the persistent banner (see
/// `ui::render`) carries the backup location.
fn profile_backup(backup: &QuarantineOutcome) -> ProfileBackup {
    match backup {
        QuarantineOutcome::BackedUp(_) => ProfileBackup::Kept,
        QuarantineOutcome::Failed(_) => ProfileBackup::Missing,
    }
}

/// Warning shown when a profile save fails; the next save retries.
///
/// Carries no error detail: paths and OS messages go to the log, not the screen.
fn save_failed_notification() -> Notification {
    Notification::with_duration(NotificationType::SaveFailed, WARNING_NOTIFICATION_DURATION)
}

/// Check if daily quests should be refreshed
///
/// Returns true if the player's current day is strictly after the day of the last quest
/// refresh; a refresh stamped on a later day (a clock or zone moved backward) is still today.
pub fn should_refresh_quests(profile: &UserProfile, now: &Moment) -> bool {
    now.day() > now.day_of(profile.last_quest_refresh)
}

/// Regenerate the daily quests when a new day began or none exist.
///
/// Waits (does nothing) while the profile is not saveable or the quest
/// registry is still loading; whichever of the profile and the registry
/// arrives last triggers the refresh. A registry that is unavailable or lacks
/// templates leaves quest slots empty and raises one warning, never a panic.
fn refresh_daily_quests_if_due(state: &mut AppState) {
    if !state.progress.persistence().is_writable() {
        return;
    }
    let now = state.progress.moment();
    if !should_refresh_quests(&state.progress.profile, &now)
        && !state.progress.profile.daily_quests.is_empty()
    {
        return;
    }
    let Some(registry) = state.progress.quest_registry.resolved() else {
        tracing::debug!("Daily quest refresh deferred until the quest registry loads");
        return;
    };

    tracing::info!("Refreshing daily quests for new day");
    state.progress.profile.reset_daily_quests(now.instant());
    let generation = QuestGenerator::generate_quests(&state.progress.profile, &registry, now.day());
    state.progress.profile.daily_quests = generation.quests;
    if generation.unfilled_slots > 0 {
        tracing::warn!(
            unfilled = generation.unfilled_slots,
            "Quest templates missing; daily quests incomplete"
        );
        state.ui.notifications.push(Notification::with_duration(
            NotificationType::QuestTemplatesMissing,
            WARNING_NOTIFICATION_DURATION,
        ));
    }
}

/// Installs a loaded profile as the session profile.
///
/// The single place that assigns `state.progress.profile`: it settles an
/// unreadable challenge day to today so the attempt limit is never refilled.
fn install_profile(state: &mut AppState, mut profile: UserProfile) {
    let today = state.progress.today();
    profile.challenge_progress.settle_unreadable_day(today);
    state.progress.profile = profile;
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
            NotificationType::OtherInstanceRunning { pid },
            WARNING_NOTIFICATION_DURATION,
        ));
    }
}

/// Compare the resolved gameplay keymap's fingerprint against the one
/// stored on the just-loaded profile, and notify if FSRS review history
/// was recorded under a different mapping.
///
/// The stock keymap fingerprints identically to `KeymapOverlay::identity()`
/// (an overlay with no entries) regardless of *why* it's active - disabled,
/// never configured, or a malformed config falling back - so it acts as its
/// own distinct "mapping" here rather than a no-tracking gap. That's
/// intentional (#348): a disable -> stock-play -> re-enable cycle must not
/// let a stale fingerprint silently match the re-enabled mapping and hide
/// that history was recorded partly under the stock keymap in between, and
/// treating stock as a real mapping catches that uniformly, the same way
/// switching between two different custom mappings would be caught.
///
/// Always converges `profile.keymap_fingerprint` to the current
/// fingerprint afterward, so this only fires once per mapping change, not
/// on every subsequent launch with the same (still-mismatched-from-some-
/// older-history) mapping.
fn warn_keymap_fingerprint_mismatch(state: &mut AppState) {
    let current = state.config.keymap.fingerprint();
    if let Some(stored) = state.progress.profile.keymap_fingerprint
        && stored.contradicts(current)
    {
        state.ui.notifications.push(Notification::with_duration(
            NotificationType::KeymapFingerprintMismatch,
            WARNING_NOTIFICATION_DURATION,
        ));
    }
    state.progress.profile.keymap_fingerprint = Some(current.into());
}

/// Handle messages from background data loaders
///
/// Processes async loading results and updates app state accordingly.
/// Handles scenarios, profiles, quest registry, and save confirmations.
#[expect(
    clippy::too_many_lines,
    reason = "one flat sequence of steps; splitting would only add indirection"
)]
pub fn handle_data_message(state: &mut AppState, msg: DataLoadMessage) {
    match msg {
        DataLoadMessage::ScenariosReady(scenarios) => {
            let count = scenarios.len();
            state.game.scenario_collection = ScenarioCollection::new(scenarios);
            tracing::info!(count, "Scenarios loaded");
        }

        DataLoadMessage::ScenariosError(err) => {
            tracing::error!(error = %error_chain(&err), "Failed to load scenarios");
        }

        DataLoadMessage::ProfileReady(profile) => {
            // Single `now` read shared by every day-boundary decision below (streak,
            // quest refresh, daily reset). Previously each check called Utc::now()
            // independently, which could straddle a midnight boundary; this also fixes
            // a latent race where the streak and quest-refresh checks could disagree
            // about what day it is.
            let now = state.progress.moment();

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

            state.progress.performance_tracker = PerformanceTracker::from_stats_with_clock(
                updated_profile.performance_data.clone(),
                state.progress.clock(),
            );
            install_profile(state, updated_profile);
            state.progress.mark_profile_loaded();

            refresh_daily_quests_if_due(state);

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
                    state
                        .ui
                        .notifications
                        .push(Notification::new(NotificationType::Achievement(
                            achievement_id,
                        )));
                }
                if let Err(e) = state.progress.save_immediate() {
                    tracing::error!(
                        error = %error_chain(&e),
                        "Failed to save profile after startup unlocks"
                    );
                    state.ui.notifications.push(save_failed_notification());
                }
            }

            tracing::info!("Profile loaded");
        }

        DataLoadMessage::ProfileError {
            error,
            fallback,
            backup,
            previous,
        } => {
            install_profile(state, fallback);
            tracing::error!(
                backup = ?backup,
                error = %error_chain(&error),
                "Profile load failed, saving disabled"
            );
            state.ui.notifications.push(Notification::with_duration(
                NotificationType::ProfileLoadFailed(profile_backup(&backup)),
                WARNING_NOTIFICATION_DURATION,
            ));
            state
                .progress
                .quarantine_profile(Quarantine { backup, previous });
            warn_if_other_instance_running(state);
        }

        DataLoadMessage::QuestRegistryReady(registry) => {
            tracing::debug!(templates = registry.len(), "Quest registry loaded");
            state.progress.quest_registry = QuestRegistrySlot::Ready(Arc::new(registry));
            refresh_daily_quests_if_due(state);
        }

        DataLoadMessage::QuestRegistryError(err) => {
            tracing::error!(error = %error_chain(&err), "Failed to load quest registry");
            state.progress.quest_registry = QuestRegistrySlot::Unavailable;
            refresh_daily_quests_if_due(state);
        }

        DataLoadMessage::ProfileSaved => {
            state.progress.mark_saved();
            tracing::debug!("Profile saved");
        }

        DataLoadMessage::ProfileSaveError(err) => {
            tracing::error!(error = %error_chain(&*err), "Failed to save profile");
            // The optimistic `mark_saved()` made when the save was
            // dispatched was wrong: re-dirty so the next debounced or
            // immediate save retries, instead of the debounce gate
            // treating this failure as a successful save indefinitely.
            state.progress.mark_unsaved();
            state.ui.notifications.push(save_failed_notification());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use helix_trainer::async_state::{LoaderError, SaveFailure};
    use helix_trainer::config::json_store::QuarantineError;
    use helix_trainer::config::json_store::{Document, StorageError};
    use helix_trainer::config::keymap::fingerprint::{KeymapFingerprint, StoredKeymapFingerprint};
    use helix_trainer::error::UserError;
    use helix_trainer::gamification::{PreviousCopy, Quest, QuestDifficulty, QuestType};
    use helix_trainer::ids::{QuestId, ScenarioId};
    use helix_trainer::{
        config::{
            CursorSpec, Pos, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution,
            TargetState,
        },
        constants::STREAK_FREEZE_MAX_GAP_DAYS,
        gamification::{ProfileStorage, QuestTemplateRegistry},
    };
    use std::assert_matches;

    // Binary crate tests cannot access the library's #[cfg(test)]-gated
    // `ProfileStorage::for_test()`, so we mirror it locally here: a unique file
    // under a process-lifetime temp dir, never the real user profile path.
    fn profile_with_fingerprint(fingerprint: KeymapFingerprint) -> UserProfile {
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = Some(StoredKeymapFingerprint::Current(fingerprint));
        profile
    }

    fn test_profile_storage() -> ProfileStorage {
        use std::sync::OnceLock;
        use std::sync::atomic::{AtomicU64, Ordering};

        static TEST_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
        static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

        let dir = TEST_DIR.get_or_init(|| tempfile::TempDir::new().expect("create test temp dir"));
        let id = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        ProfileStorage::with_path(dir.path().join(format!("profile-{id}.json")))
    }

    fn corrupt_profile_error() -> StorageError {
        StorageError::Corrupt {
            doc: Document::Profile,
            path: "profile.json".into(),
            source: serde_json::from_str::<u8>("x").unwrap_err(),
        }
    }

    fn no_backup() -> QuarantineOutcome {
        QuarantineOutcome::Failed(Arc::new(QuarantineError::Missing))
    }

    fn empty_test_app_state() -> AppState {
        let profile = UserProfile::new();
        let storage = test_profile_storage();
        let tracker = PerformanceTracker::new();
        AppState::new(vec![], profile, storage, tracker)
    }

    fn default_test_scenario() -> Scenario {
        Scenario {
            id: ScenarioId::parse("test_scenario").unwrap(),
            name: "Test Scenario".to_string(),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\nline 3\n".to_string(),
                language: None,
                cursor: CursorSpec::Point(Pos::try_from([0, 0]).unwrap()),
            },
            target: TargetState {
                file_content: "line 2\nline 3\n".to_string(),
                cursor: CursorSpec::Point(Pos::try_from([0, 0]).unwrap()),
            },
            solution: Solution {
                commands: vec!["x".to_string(), "d".to_string()],
                description: "Delete first line".to_string(),
            },
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: std::num::NonZeroUsize::new(2).unwrap(),
                max_points: 100,
                tolerance: 0,
            },
            metadata: ScenarioMetadata::default(),
        }
    }

    /// #660: quests refresh at the player's local midnight (UTC+9: not at 09:00 local).
    #[test]
    fn quests_refresh_at_local_midnight_east_of_utc() {
        use helix_trainer::time::{Clock, FakeClock};

        let clock = FakeClock::at_offset("2026-01-14T23:00:00Z", 9); // 08:00 local the 15th
        let mut profile = UserProfile::new_at(clock.now());
        profile.last_quest_refresh = clock.now();

        clock.advance(chrono::Duration::minutes(90)); // 09:30 local, the next UTC date
        assert!(!should_refresh_quests(&profile, &clock.moment()));

        clock.advance(chrono::Duration::hours(15)); // 00:30 local the 16th
        assert!(should_refresh_quests(&profile, &clock.moment()));
    }

    /// FR-007: east, west, east on one real day refreshes at most once.
    #[test]
    fn quests_do_not_refresh_again_when_the_zone_moves_west() {
        use helix_trainer::time::{Clock, FakeClock};

        let clock = FakeClock::at_offset("2026-01-15T20:00:00Z", 9); // the 16th, local
        let mut profile = UserProfile::new_at(clock.now());
        profile.last_quest_refresh = clock.now();
        clock.set_utc_offset_hours(-8); // the 15th, local

        assert!(!should_refresh_quests(&profile, &clock.moment()));
    }

    #[test]
    fn test_handle_scenarios_ready() {
        let mut state = empty_test_app_state();
        let scenarios = vec![default_test_scenario()];

        handle_data_message(&mut state, DataLoadMessage::ScenariosReady(scenarios));

        assert_eq!(state.game.scenario_collection.count(), 1);
    }

    #[test]
    fn test_handle_scenarios_ready_empty() {
        let mut state = empty_test_app_state();

        handle_data_message(&mut state, DataLoadMessage::ScenariosReady(vec![]));

        assert_eq!(state.game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_handle_scenarios_error() {
        let mut state = empty_test_app_state();

        handle_data_message(
            &mut state,
            DataLoadMessage::ScenariosError(LoaderError::Scenarios(
                UserError::scenario_load_error(),
            )),
        );

        // Should not panic, just log error
        // Scenarios should remain empty
        assert_eq!(state.game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_handle_profile_ready() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.total_xp = 500;
        profile.level = 3;

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        let loaded_profile = &state.progress.profile;
        assert_eq!(loaded_profile.total_xp, 500);
        assert_eq!(loaded_profile.level, 3);
    }

    fn app_state_with_keymap_toml(toml: &str) -> AppState {
        use helix_trainer::input::keymap::resolve_str;
        use helix_trainer::ui::state::ConfigState;

        let (keymap, _) = resolve_str(toml).unwrap();
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

    fn app_state_with_keymap() -> AppState {
        app_state_with_keymap_toml(
            r#"
            [keys.normal]
            j = "move_char_left"
            "#,
        )
    }

    fn different_from(fingerprint: KeymapFingerprint) -> KeymapFingerprint {
        let KeymapFingerprint::Fnv1a64V1(value) = fingerprint;
        KeymapFingerprint::Fnv1a64V1(value.wrapping_add(1))
    }

    /// A legacy untagged fingerprint (written by the pre-tag hasher) cannot be
    /// compared: it must not notify and is replaced by the current value.
    #[test]
    fn test_handle_profile_ready_replaces_a_legacy_fingerprint_without_notifying() {
        let mut state = app_state_with_keymap();
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = Some(StoredKeymapFingerprint::Legacy(12345));

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(
            !state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "a legacy fingerprint must not raise the mismatch notification"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(
                state.config.keymap.fingerprint()
            ))
        );
    }

    /// A stale `keymap_fingerprint` (recorded under a different mapping)
    /// must notify when the currently active keymap doesn't match.
    #[test]
    fn test_handle_profile_ready_notifies_on_keymap_fingerprint_mismatch() {
        let mut state = app_state_with_keymap();
        let current_fingerprint = state.config.keymap.fingerprint();
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = Some(StoredKeymapFingerprint::Current(different_from(
            current_fingerprint,
        )));

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(
            state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "expected a keymap fingerprint mismatch notification"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(current_fingerprint)),
            "fingerprint must converge to the current keymap after notifying"
        );
    }

    /// Steady state: an ordinary restart with the same custom keymap still
    /// enabled and no intervening change must stay silent - the stored
    /// fingerprint already matches the currently active one, so there is no
    /// mapping transition to report.
    #[test]
    fn test_handle_profile_ready_no_notification_when_keymap_unchanged() {
        let mut state = app_state_with_keymap();
        let current_fingerprint = state.config.keymap.fingerprint();
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = Some(StoredKeymapFingerprint::Current(current_fingerprint));

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(
            !state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "an unchanged custom keymap must not renotify on every launch"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(current_fingerprint))
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

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(
            !state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "first-time keymap activation must not notify a mismatch"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(
                state.config.keymap.fingerprint()
            ))
        );
    }

    /// A brand-new profile that has never played under any keymap
    /// (`keymap_fingerprint` is `None`) must not be treated as a mismatch
    /// when the stock keymap is active - the stock keymap's fingerprint is
    /// a real "first activation" case too, not a special no-tracking gap.
    #[test]
    fn test_handle_profile_ready_no_notification_on_first_stock_activation() {
        let mut state = empty_test_app_state();
        assert!(state.config.keymap.is_empty());
        let mut profile = UserProfile::new();
        profile.keymap_fingerprint = None;

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(
            !state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "first-ever activation (even of the stock keymap) must not notify a mismatch"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(
                state.config.keymap.fingerprint()
            ))
        );
    }

    /// With the stock keymap active, a `keymap_fingerprint` recorded while
    /// a custom mapping was previously active must still be flagged as a
    /// mismatch (#348): the stock keymap fingerprints as its own distinct
    /// "mapping" (see `warn_keymap_fingerprint_mismatch`'s doc comment), so
    /// disabling a custom keymap is itself a genuine mapping change worth
    /// surfacing, not something to silently swallow.
    #[test]
    fn test_handle_profile_ready_notifies_when_keymap_disabled_after_custom_mapping() {
        let custom_fingerprint = app_state_with_keymap().config.keymap.fingerprint();
        let mut state = empty_test_app_state();
        assert!(state.config.keymap.is_empty());
        let stock_fingerprint = state.config.keymap.fingerprint();
        assert_ne!(custom_fingerprint, stock_fingerprint);

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(profile_with_fingerprint(custom_fingerprint)),
        );

        assert!(
            state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "disabling a custom keymap must notify that history now spans a different mapping"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(stock_fingerprint))
        );
    }

    /// Regression test for #348: disabling a custom keymap, playing under
    /// stock, then re-enabling the *same* custom keymap must notify at
    /// *each* mapping transition. The stock keymap fingerprints as its own
    /// distinct mapping, so re-enabling can no longer silently look like an
    /// unbroken continuation of the original custom mapping just because
    /// the fingerprint value happens to match again.
    #[test]
    fn test_keymap_disable_reenable_cycle_detects_mismatch_at_each_transition() {
        let mut state = app_state_with_keymap();
        let custom_fingerprint = state.config.keymap.fingerprint();

        // Step 1: disable the custom keymap (stock is now active) and reload
        // a profile carrying the fingerprint recorded while it was active -
        // stock is a different mapping, so this must notify.
        let mut disabled_state = empty_test_app_state();
        let stock_fingerprint = disabled_state.config.keymap.fingerprint();
        handle_data_message(
            &mut disabled_state,
            DataLoadMessage::ProfileReady(profile_with_fingerprint(custom_fingerprint)),
        );
        assert!(
            disabled_state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "disabling a custom keymap must notify that history now spans a different mapping"
        );
        assert_eq!(
            disabled_state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(stock_fingerprint))
        );

        // Step 2: re-enable the same custom keymap. The intervening stock
        // play recorded `stock_fingerprint`, which again differs from
        // `custom_fingerprint` - so this must notify too, correctly
        // surfacing that history spans two mappings, not silently matching
        // a stale value.
        let carried_over_profile = disabled_state.progress.profile;
        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(carried_over_profile),
        );

        assert!(
            state.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "re-enabling after a stock-keymap gap must notify, not silently match a stale fingerprint"
        );
        assert_eq!(
            state.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(custom_fingerprint))
        );
    }

    /// Regression test for the impl-critic's S2 finding: re-enabling a
    /// *different* custom keymap after an intervening stock-keymap gap must
    /// still be flagged as a mismatch - clearing/sentinel-izing the
    /// fingerprint during the stock gap must not accidentally make this
    /// look like a fresh first activation.
    #[test]
    fn test_keymap_disable_then_different_keymap_reenable_still_mismatches() {
        let fingerprint_a = app_state_with_keymap().config.keymap.fingerprint();
        let mut state_b = app_state_with_keymap_toml(
            r#"
            [keys.normal]
            k = "move_char_left"
            "#,
        );
        let fingerprint_b = state_b.config.keymap.fingerprint();
        assert_ne!(fingerprint_a, fingerprint_b);

        // Disable keymap A (stock active), reload a profile carrying A's fingerprint.
        let mut disabled_state = empty_test_app_state();
        handle_data_message(
            &mut disabled_state,
            DataLoadMessage::ProfileReady(profile_with_fingerprint(fingerprint_a)),
        );
        let carried_over_profile = disabled_state.progress.profile;

        // Re-enable a *different* custom keymap B.
        handle_data_message(
            &mut state_b,
            DataLoadMessage::ProfileReady(carried_over_profile),
        );

        assert!(
            state_b.ui.notifications.visible().any(|n| matches!(
                n.notification_type,
                NotificationType::KeymapFingerprintMismatch
            )),
            "switching to a different custom keymap after a stock gap must still notify"
        );
        assert_eq!(
            state_b.progress.profile.keymap_fingerprint,
            Some(StoredKeymapFingerprint::Current(fingerprint_b))
        );
    }

    /// Regression test for #256: a streak that crosses a milestone at load time (the only
    /// place `StreakManager::update_streak` runs) must unlock the corresponding achievement
    /// and notify the user, not just update `current_streak`.
    #[test]
    fn test_handle_profile_ready_unlocks_streak_achievement() {
        use helix_trainer::gamification::AchievementId;
        use helix_trainer::time::{Clock, FakeClock, SystemMonotonicClock};
        use helix_trainer::ui::state::{AppState, ConfigState};
        use std::sync::Arc;

        // Construct AppState via with_clocks directly (rather than building it with the
        // default SystemClock and reassigning `progress.clock` afterward) so the injected
        // clock is shared by every clock-consuming field from the start, not just read back.
        let clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
        let mut state = AppState::with_clocks(
            vec![],
            UserProfile::new(),
            test_profile_storage(),
            PerformanceTracker::new(),
            ConfigState::default(),
            clock.clone(),
            Arc::new(SystemMonotonicClock),
        );

        let mut profile = UserProfile::new_at(clock.now());
        profile.current_streak = 6;
        profile.daily_quests.push(Quest::new(
            QuestId::parse("quest_0").unwrap(),
            QuestType::ScenarioCompletion {
                target: 1,
                current: 1,
            },
            QuestDifficulty::Easy,
        ));
        assert!(profile.complete_quest(&QuestId::parse("quest_0").unwrap()));
        clock.advance_days(1);

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        // Streak should have incremented from 6 to 7, crossing the Streak7Days threshold
        assert_eq!(state.progress.profile.current_streak, 7);
        assert!(
            state
                .progress
                .profile
                .has_achievement(&AchievementId::Streak7Days)
        );
        assert!(state.ui.notifications.visible().any(|n| matches!(
            n.notification_type,
            NotificationType::Achievement(AchievementId::Streak7Days)
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

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );

        let _ = child.kill();
        let _ = child.wait();

        assert!(
            state.ui.notifications.visible().any(|n| matches!(
                &n.notification_type,
                NotificationType::OtherInstanceRunning { pid } if *pid == other_pid
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

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(!state.progress.profile.streak_freeze_available);
        assert_eq!(state.progress.profile.current_streak, 5);
        assert!(
            state
                .ui
                .notifications
                .visible()
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

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            state.ui.notifications.visible().any(|n| n.notification_type
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

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            state.progress.profile.streak_freeze_available,
            "freeze must remain held since it couldn't cover this gap"
        );
        assert!(
            state.ui.notifications.visible().any(|n| n.notification_type
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

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert_eq!(state.progress.profile.current_streak, 0);
        assert!(
            !state
                .ui
                .notifications
                .visible()
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

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

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
                .any(|n| n.notification_type == NotificationType::StreakFreezeUsed),
            "no streak was ever active, so no StreakFreezeUsed notification should fire"
        );
    }

    #[test]
    fn test_handle_profile_error_uses_fallback() {
        let mut state = empty_test_app_state();
        let mut fallback = UserProfile::new();
        fallback.total_xp = 100; // Mark fallback with some XP

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileError {
                error: corrupt_profile_error(),
                fallback,
                backup: no_backup(),
                previous: PreviousCopy::Unavailable,
            },
        );

        let loaded_profile = &state.progress.profile;
        assert_eq!(loaded_profile.total_xp, 100); // Fallback was used
    }

    /// Regression test for #440: after a failed load the unparsable file is
    /// never replaced by the fallback profile, on the exit path or the
    /// debounced/immediate save paths, and the user is told.
    #[test]
    fn test_profile_error_never_overwrites_unparsable_file() {
        let storage = test_profile_storage();
        std::fs::write(storage.path(), "{broken").unwrap();
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            storage.clone(),
            PerformanceTracker::new(),
        );
        state.progress.begin_profile_load();

        let backup = storage.quarantine();
        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileError {
                error: corrupt_profile_error(),
                fallback: UserProfile::new(),
                backup: backup.clone(),
                previous: PreviousCopy::Unavailable,
            },
        );

        assert!(state.progress.prepare_final_save_request().is_none());
        state.progress.save_immediate().unwrap();
        state.progress.save_debounced().unwrap();
        assert_eq!(std::fs::read_to_string(storage.path()).unwrap(), "{broken");
        let QuarantineOutcome::BackedUp(backup_path) = backup else {
            panic!("expected a backup copy");
        };
        assert_eq!(std::fs::read_to_string(backup_path).unwrap(), "{broken");
        assert!(
            state
                .ui
                .notifications
                .visible()
                .any(|n| matches!(&n.notification_type, NotificationType::ProfileLoadFailed(_))),
            "a failed profile load must be announced"
        );
    }

    /// Regression test for the #440 follow-up trigger: quitting before
    /// `ProfileReady` arrives must not overwrite the real file with the
    /// placeholder profile.
    #[test]
    fn test_exit_before_profile_ready_does_not_save_placeholder() {
        let mut state = empty_test_app_state();
        state.progress.begin_profile_load();

        assert!(state.progress.prepare_final_save_request().is_none());

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );
        assert!(state.progress.prepare_final_save_request().is_some());
    }

    /// Regression test for #459: the saved sound settings are applied to the
    /// sound manager when the profile arrives (volume clamped).
    #[test]
    fn test_profile_ready_applies_saved_sound_config() {
        let mut state = empty_test_app_state();
        let mut profile = UserProfile::new();
        profile.sound_config =
            serde_json::from_str(r#"{"volume": 12.0, "enabled": false}"#).unwrap();

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        let config = state.progress.sound_manager.config();
        assert!(!config.enabled());
        assert!((config.volume().get() - 1.0).abs() < f32::EPSILON);
    }

    /// Regression test for #466: a failed startup save is reported, not
    /// propagated as a fatal error.
    #[test]
    fn test_profile_ready_startup_save_failure_is_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        // A directory in place of the profile file makes the save's rename fail.
        let path = dir.path().join("profile.json");
        std::fs::create_dir(&path).unwrap();
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::with_path(&path),
            PerformanceTracker::new(),
        );
        let mut profile = UserProfile::new();
        // A long streak guarantees a startup achievement unlock (and so a save).
        profile.current_streak = 400;
        profile.longest_streak = 400;
        profile.last_activity = state.progress.now();

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        assert!(
            state
                .ui
                .notifications
                .visible()
                .any(|n| matches!(&n.notification_type, NotificationType::SaveFailed)),
            "the failed startup save must be surfaced"
        );
    }

    /// Regression test for #467: load-time achievement saves go through the
    /// serialized save writer, not a direct `ProfileStorage::save` call.
    #[test]
    fn test_profile_ready_startup_save_uses_serialized_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.json");
        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::with_path(&path),
            PerformanceTracker::new(),
        );
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        state.set_save_channel(tx);
        let mut profile = UserProfile::new();
        profile.current_streak = 400;
        profile.longest_streak = 400;
        profile.last_activity = state.progress.now();

        handle_data_message(&mut state, DataLoadMessage::ProfileReady(profile));

        let request = rx.try_recv().expect("save must be queued on the writer");
        assert_eq!(request.storage.path(), path);
        assert!(!path.exists(), "no synchronous write on the event loop");
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

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileError {
                error: corrupt_profile_error(),
                fallback: UserProfile::new(),
                backup: no_backup(),
                previous: PreviousCopy::Unavailable,
            },
        );

        let _ = child.kill();
        let _ = child.wait();

        assert!(
            state.ui.notifications.visible().any(|n| matches!(
                &n.notification_type,
                NotificationType::OtherInstanceRunning { pid } if *pid == other_pid
            )),
            "expected a notification warning about the other running instance"
        );
    }

    #[test]
    fn test_handle_profile_saved() {
        let mut state = empty_test_app_state();

        handle_data_message(&mut state, DataLoadMessage::ProfileSaved);
    }

    /// #615: an unreadable challenge day is settled to today on both install paths,
    /// so the used attempts survive instead of the allowance being refilled.
    #[test]
    fn test_install_profile_settles_unreadable_challenge_day() {
        use helix_trainer::gamification::ChallengeDay;

        let unreadable = || {
            let mut profile = UserProfile::new();
            profile.challenge_progress.last_challenge_date = ChallengeDay::Unreadable;
            profile.challenge_progress.attempts_used_today = 3;
            profile
        };

        let mut ready = empty_test_app_state();
        handle_data_message(&mut ready, DataLoadMessage::ProfileReady(unreadable()));
        let mut fallback = empty_test_app_state();
        handle_data_message(
            &mut fallback,
            DataLoadMessage::ProfileError {
                error: corrupt_profile_error(),
                fallback: unreadable(),
                backup: no_backup(),
                previous: PreviousCopy::Unavailable,
            },
        );

        for state in [ready, fallback] {
            let today = state.progress.today();
            let progress = &state.progress.profile.challenge_progress;
            assert_eq!(progress.last_challenge_date, ChallengeDay::On(today));
            assert!(!progress.can_attempt(today));
        }
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

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileSaveError(Arc::new(SaveFailure::Storage(
                StorageError::Write {
                    doc: Document::Profile,
                    path: "profile.json".into(),
                    source: std::io::Error::other("Disk full"),
                },
            ))),
        );

        assert!(
            state.progress.should_save(),
            "a failed save must re-dirty the profile so the next save retries"
        );
        assert!(
            state
                .ui
                .notifications
                .visible()
                .any(|n| matches!(&n.notification_type, NotificationType::SaveFailed)),
            "a failed save must surface a user-facing notification"
        );
    }

    fn embedded_quest_registry() -> QuestTemplateRegistry {
        QuestTemplateRegistry::from_embedded("en").unwrap()
    }

    fn quest_warning_count(state: &AppState) -> usize {
        state
            .ui
            .notifications
            .visible()
            .filter(|n| matches!(n.notification_type, NotificationType::QuestTemplatesMissing))
            .count()
    }

    #[test]
    fn test_handle_quest_registry_ready() {
        let mut state = empty_test_app_state();

        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryReady(embedded_quest_registry()),
        );

        assert_matches!(state.progress.quest_registry, QuestRegistrySlot::Ready(_));
    }

    #[test]
    fn test_quests_are_generated_when_registry_arrives_before_profile() {
        let mut state = empty_test_app_state();
        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryReady(embedded_quest_registry()),
        );
        assert_eq!(
            state.progress.profile.daily_quests,
            [] as [helix_trainer::gamification::Quest; 0]
        );

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );

        assert_eq!(state.progress.profile.daily_quests.len(), 3);
        assert_eq!(quest_warning_count(&state), 0);
    }

    #[test]
    fn test_quest_refresh_is_deferred_until_registry_arrives_after_profile() {
        let mut state = empty_test_app_state();

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );
        assert_matches!(state.progress.quest_registry, QuestRegistrySlot::Pending);
        assert_eq!(
            state.progress.profile.daily_quests,
            [] as [helix_trainer::gamification::Quest; 0]
        );
        assert_eq!(quest_warning_count(&state), 0);

        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryReady(embedded_quest_registry()),
        );

        assert_eq!(state.progress.profile.daily_quests.len(), 3);
        assert_eq!(quest_warning_count(&state), 0);
    }

    /// Regression test for #469: a registry that failed to load leaves quests
    /// empty with one warning instead of aborting the process.
    #[test]
    fn test_unavailable_registry_after_profile_warns_once_without_panicking() {
        let mut state = empty_test_app_state();
        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );

        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryError(LoaderError::QuestRegistry(
                UserError::scenario_load_error(),
            )),
        );

        assert_matches!(
            state.progress.quest_registry,
            QuestRegistrySlot::Unavailable
        );
        assert_eq!(
            state.progress.profile.daily_quests,
            [] as [helix_trainer::gamification::Quest; 0]
        );
        assert_eq!(quest_warning_count(&state), 1);
    }

    #[test]
    fn test_unavailable_registry_before_profile_warns_once_without_panicking() {
        let mut state = empty_test_app_state();
        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryError(LoaderError::QuestRegistry(
                UserError::scenario_load_error(),
            )),
        );
        assert_eq!(quest_warning_count(&state), 0);

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );

        assert_eq!(
            state.progress.profile.daily_quests,
            [] as [helix_trainer::gamification::Quest; 0]
        );
        assert_eq!(quest_warning_count(&state), 1);
    }

    #[test]
    fn test_empty_ready_registry_warns_without_panicking() {
        let mut state = empty_test_app_state();
        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryReady(QuestTemplateRegistry::new()),
        );

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileReady(UserProfile::new()),
        );

        assert_eq!(
            state.progress.profile.daily_quests,
            [] as [helix_trainer::gamification::Quest; 0]
        );
        assert_eq!(quest_warning_count(&state), 1);
    }

    #[test]
    fn test_quarantined_profile_does_not_generate_quests() {
        let mut state = empty_test_app_state();
        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryReady(embedded_quest_registry()),
        );

        handle_data_message(
            &mut state,
            DataLoadMessage::ProfileError {
                error: corrupt_profile_error(),
                fallback: UserProfile::new(),
                backup: no_backup(),
                previous: PreviousCopy::Unavailable,
            },
        );

        assert_eq!(
            state.progress.profile.daily_quests,
            [] as [helix_trainer::gamification::Quest; 0]
        );
    }

    #[test]
    fn test_handle_quest_registry_error() {
        let mut state = empty_test_app_state();

        handle_data_message(
            &mut state,
            DataLoadMessage::QuestRegistryError(LoaderError::QuestRegistry(
                UserError::scenario_load_error(),
            )),
        );

        // Should not panic, just log error
    }

    /// FR-094: every loader and save failure arm logs the full cause chain.
    #[test]
    fn test_failure_arms_log_the_cause_chain() {
        let cause = serde_json::from_str::<u8>("x").unwrap_err().to_string();
        let logs = crate::log_capture::capture_logs(|| {
            let mut state = empty_test_app_state();
            handle_data_message(
                &mut state,
                DataLoadMessage::ProfileError {
                    error: corrupt_profile_error(),
                    fallback: UserProfile::new(),
                    backup: no_backup(),
                    previous: PreviousCopy::Unavailable,
                },
            );
            handle_data_message(
                &mut state,
                DataLoadMessage::ScenariosError(LoaderError::Scenarios(
                    UserError::scenario_load_error(),
                )),
            );
            handle_data_message(
                &mut state,
                DataLoadMessage::QuestRegistryError(LoaderError::QuestRegistry(
                    UserError::scenario_load_error(),
                )),
            );
            handle_data_message(
                &mut state,
                DataLoadMessage::ProfileSaveError(Arc::new(SaveFailure::Storage(
                    corrupt_profile_error(),
                ))),
            );
        });

        assert_eq!(logs.matches(&cause).count(), 2, "{logs}");
        assert!(logs.contains("failed to load the scenarios: "), "{logs}");
        assert!(
            logs.contains("failed to load the quest templates: "),
            "{logs}"
        );
        assert!(logs.contains("failed to save the profile: "), "{logs}");
    }
}
