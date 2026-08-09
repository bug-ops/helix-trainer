//! Data loading message handling
//!
//! Processes messages from background data loaders and updates app state.

use anyhow::Result;
use chrono::{DateTime, Utc};

use helix_trainer::{
    async_state::DataLoadMessage,
    config::ScenarioCollection,
    gamification::{QuestGenerator, QuestTemplateRegistry, StreakManager, UserProfile},
    learning::PerformanceTracker,
    ui::AppState,
};

/// Check if daily quests should be refreshed
///
/// Returns true if the current date differs from the last quest refresh date.
pub fn should_refresh_quests(profile: &UserProfile, now: DateTime<Utc>) -> bool {
    now.date_naive() != profile.last_quest_refresh.date_naive()
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
            // Update streak and refresh quests if needed
            let mut updated_profile = profile;
            let streak_change = StreakManager::update_streak(&mut updated_profile);
            tracing::debug!("Streak status: {:?}", streak_change);

            // Check if we need to refresh daily quests
            let now = Utc::now();
            if should_refresh_quests(&updated_profile, now)
                || updated_profile.daily_quests.is_empty()
            {
                tracing::info!("Refreshing daily quests for new day");
                let tracker = PerformanceTracker::new();
                updated_profile.reset_daily_quests();

                // Load quest registry synchronously
                let quest_registry = QuestTemplateRegistry::load_from_default_path("en")
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            "Failed to load quest templates: {}, using empty registry",
                            e
                        );
                        QuestTemplateRegistry::new()
                    });

                updated_profile.daily_quests =
                    QuestGenerator::generate_quests(&updated_profile, &tracker, &quest_registry);
            }

            state.progress.performance_tracker =
                PerformanceTracker::from_stats(updated_profile.performance_data.clone());
            state.progress.profile = updated_profile;
            tracing::info!("Profile loaded");
        }

        DataLoadMessage::ProfileError { error, fallback } => {
            state.progress.profile = fallback;
            tracing::warn!("Profile load failed, using default: {}", error);
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
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use helix_trainer::{
        config::{Scenario, ScoringConfig, Setup, Solution, TargetState},
        gamification::ProfileStorage,
    };

    // Binary crate tests cannot access the library's #[cfg(test)] modules,
    // so we define minimal local helpers here
    fn empty_test_app_state() -> AppState {
        let profile = UserProfile::new();
        let storage = ProfileStorage::new();
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
                cursor_position: Some((0, 0)),
                selection: None,
                cursors: None,
                selections: None,
            },
            target: TargetState {
                file_content: "line 2\nline 3\n".to_string(),
                cursor_position: Some((0, 0)),
                selection: None,
                cursors: None,
                selections: None,
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

    #[test]
    fn test_handle_profile_saved() {
        let mut state = empty_test_app_state();

        let result = handle_data_message(&mut state, DataLoadMessage::ProfileSaved);

        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_profile_save_error() {
        let mut state = empty_test_app_state();

        let result = handle_data_message(
            &mut state,
            DataLoadMessage::ProfileSaveError("Disk full".to_string()),
        );

        // Should not panic, just log error
        assert!(result.is_ok());
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
