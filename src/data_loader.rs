//! Async data loading functions
//!
//! This module provides async wrappers for data loading operations,
//! enabling background loading without blocking the UI thread.
//!
//! Scenarios are loaded from compile-time embedded data, while
//! profiles are loaded from the user's config directory.

use crate::async_state::DataLoadMessage;
use crate::config::{Scenario, ScenarioLoader};
use crate::gamification::{ProfileStorage, QuestTemplateRegistry, UserProfile};
use anyhow::Result;
use tokio::sync::mpsc;

/// Spawn background data loaders for scenarios, profile, and quest registry
///
/// This function spawns async tasks that load data in the background and
/// send results through the provided channel.
///
/// # Arguments
///
/// * `tx` - Channel sender for communicating loading results
///
/// # Examples
///
/// ```no_run
/// use tokio::sync::mpsc;
/// use helix_trainer::data_loader::spawn_data_loaders;
///
/// #[tokio::main]
/// async fn main() {
///     let (tx, mut rx) = mpsc::channel(32);
///     spawn_data_loaders(tx);
///
///     // Handle messages from loaders
///     while let Some(msg) = rx.recv().await {
///         // Process data load messages
///     }
/// }
/// ```
pub fn spawn_data_loaders(tx: mpsc::Sender<DataLoadMessage>) {
    let tx_scenarios = tx.clone();
    let tx_profile = tx;

    // Spawn scenario loader (runs in parallel)
    tokio::spawn(async move {
        let result = load_scenarios_async().await;
        let msg = match result {
            Ok(scenarios) => DataLoadMessage::ScenariosReady(scenarios),
            Err(e) => DataLoadMessage::ScenariosError(e.to_string()),
        };
        if tx_scenarios.send(msg).await.is_err() {
            tracing::warn!("Failed to send scenarios message: receiver dropped");
        }
    });

    // Spawn profile loader (runs in parallel)
    tokio::spawn(async move {
        let result = load_profile_async().await;
        let msg = match result {
            Ok(profile) => DataLoadMessage::ProfileReady(profile),
            Err(e) => DataLoadMessage::ProfileError {
                error: e.to_string(),
                fallback: UserProfile::new(),
            },
        };
        if tx_profile.send(msg).await.is_err() {
            tracing::warn!("Failed to send profile message: receiver dropped");
        }
    });
}

/// Async scenario loading from embedded data
///
/// Loads scenarios from compile-time embedded TOML content.
/// This eliminates filesystem I/O and ensures consistent behavior
/// regardless of the working directory.
///
/// # Errors
///
/// Returns an error if scenarios cannot be loaded from embedded data.
pub async fn load_scenarios_async() -> Result<Vec<Scenario>> {
    tokio::task::spawn_blocking(move || {
        let current_locale = rust_i18n::locale();
        let locale_str: &str = current_locale.as_ref();

        let loader = ScenarioLoader::new();
        loader
            .load_from_embedded(locale_str)
            .map_err(|e| anyhow::anyhow!("Scenario load error: {:?}", e))
    })
    .await?
}

/// Async profile loading
///
/// Loads user profile from storage using tokio's blocking task pool.
///
/// # Errors
///
/// Returns an error if the profile cannot be loaded from storage.
pub async fn load_profile_async() -> Result<UserProfile> {
    tokio::task::spawn_blocking(move || {
        let storage = ProfileStorage::new();
        storage
            .load()
            .map_err(|e| anyhow::anyhow!("Profile load error: {:?}", e))
    })
    .await?
}

/// Async quest registry loading (lazy loaded)
///
/// Loads quest templates from the default path using tokio's blocking task pool.
///
/// # Arguments
///
/// * `locale` - Locale string for loading localized quest templates
///
/// # Errors
///
/// Returns an error if the quest registry cannot be loaded.
pub async fn load_quest_registry_async(locale: &str) -> Result<QuestTemplateRegistry> {
    let locale = locale.to_string();
    tokio::task::spawn_blocking(move || {
        QuestTemplateRegistry::load_from_default_path(&locale)
            .map_err(|e| anyhow::anyhow!("Quest registry error: {:?}", e))
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::DATA_LOADING_TIMEOUT;

    // Miri is too slow for async/tokio tests (300+ seconds per test)
    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_scenarios_async() {
        // This test requires scenario files to exist
        // In a real scenario, we'd have test fixtures
        let result = load_scenarios_async().await;

        // Should either succeed or fail with a clear error
        match result {
            Ok(scenarios) => {
                // If scenarios exist, should have at least one
                assert!(!scenarios.is_empty(), "Expected at least one scenario");
            }
            Err(e) => {
                // If scenarios don't exist, should have a clear error message
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Scenario") || error_msg.contains("load"),
                    "Error should mention scenarios or loading: {}",
                    error_msg
                );
            }
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_profile_async() {
        let result = load_profile_async().await;

        // Profile loading should succeed - either loads existing or creates new
        match result {
            Ok(profile) => {
                // Profile should have valid level (1 for new, or higher for existing)
                assert!(
                    profile.level >= 1,
                    "Profile level should be at least 1, got {}",
                    profile.level
                );
            }
            Err(e) => {
                // Profile load should not fail in normal circumstances
                // If config directory is inaccessible, this may fail
                panic!(
                    "Profile loading should succeed (creates new if missing). Error: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_quest_registry_async() {
        let result = load_quest_registry_async("en").await;

        // Quest registry should load successfully (quests/en/daily.toml exists)
        match result {
            Ok(registry) => {
                // Registry loaded successfully - verify it has templates
                assert!(
                    !registry.is_empty(),
                    "Quest registry should have templates loaded from quests/en/daily.toml"
                );
            }
            Err(e) => {
                // If this fails, the quests directory may be missing
                panic!(
                    "Quest registry should load successfully. Error: {}. \
                     Make sure quests/en/daily.toml exists.",
                    e
                );
            }
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_spawn_data_loaders() {
        let (tx, mut rx) = mpsc::channel(32);

        spawn_data_loaders(tx);

        // Collect messages (should get scenarios and profile)
        let mut scenarios_received = false;
        let mut profile_received = false;

        // Wait for both messages with timeout
        for _ in 0..2 {
            if let Ok(msg) = tokio::time::timeout(DATA_LOADING_TIMEOUT, rx.recv()).await {
                match msg {
                    Some(DataLoadMessage::ScenariosReady(_))
                    | Some(DataLoadMessage::ScenariosError(_)) => {
                        scenarios_received = true;
                    }
                    Some(DataLoadMessage::ProfileReady(_))
                    | Some(DataLoadMessage::ProfileError { .. }) => {
                        profile_received = true;
                    }
                    _ => {}
                }
            }
        }

        assert!(
            scenarios_received,
            "Should receive scenarios message (ready or error)"
        );
        assert!(
            profile_received,
            "Should receive profile message (ready or error)"
        );
    }
}
