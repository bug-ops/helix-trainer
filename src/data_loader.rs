//! Async data loading functions
//!
//! This module provides async wrappers for data loading operations,
//! enabling background loading without blocking the UI thread.
//!
//! Scenarios are loaded from compile-time embedded data, while
//! profiles are loaded from the user's config directory.

use crate::async_state::{
    DataLoadMessage, LoaderError, SaveFailure, SaveRequest, SaveWriterOutcome,
};
use crate::config::json_store::{Document, StorageError};
use crate::config::{Scenario, ScenarioLoader};
use crate::constants::PROFILE_SAVE_QUEUE_CAPACITY;
use crate::gamification::{
    PreviousCopy, ProfileStorage, QuarantineOutcome, QuestTemplateRegistry, UserProfile,
};
use crate::security::CappedReadError;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::{JoinError, JoinHandle};

/// Spawn background data loaders for scenarios, profile, and quest registry
///
/// This function spawns async tasks that load data in the background and
/// send results through the provided channel.
///
/// # Arguments
///
/// * `tx` - Channel sender for communicating loading results
/// * `storage` - Profile storage to load from. Pass a clone of the application's own handle:
///   clones share the last known-valid profile text, so the first save after startup can keep
///   the profile that just loaded as `profile.json.prev`
///
/// # Examples
///
/// ```no_run
/// use tokio::sync::mpsc;
/// use helix_trainer::data_loader::spawn_data_loaders;
/// use helix_trainer::gamification::ProfileStorage;
///
/// #[tokio::main]
/// async fn main() {
///     let (tx, mut rx) = mpsc::channel(32);
///     spawn_data_loaders(tx, ProfileStorage::with_path("/tmp/example-profile.json"));
///
///     // Handle messages from loaders
///     while let Some(msg) = rx.recv().await {
///         // Process data load messages
///     }
/// }
/// ```
pub fn spawn_data_loaders(tx: mpsc::Sender<DataLoadMessage>, storage: ProfileStorage) {
    let tx_scenarios = tx.clone();
    let tx_quests = tx.clone();
    let tx_profile = tx;

    // Spawn scenario loader (runs in parallel)
    tokio::spawn(async move {
        let result = load_scenarios_async().await;
        let msg = match result {
            Ok(scenarios) => DataLoadMessage::ScenariosReady(scenarios),
            Err(e) => DataLoadMessage::ScenariosError(e),
        };
        if tx_scenarios.send(msg).await.is_err() {
            tracing::warn!("Failed to send scenarios message: receiver dropped");
        }
    });

    // Spawn quest registry loader (runs in parallel)
    tokio::spawn(async move {
        let locale = rust_i18n::locale();
        let msg = match load_quest_registry_async(&locale).await {
            Ok(registry) => DataLoadMessage::QuestRegistryReady(registry),
            Err(e) => DataLoadMessage::QuestRegistryError(e),
        };
        if tx_quests.send(msg).await.is_err() {
            tracing::warn!("Failed to send quest registry message: receiver dropped");
        }
    });

    // Spawn profile loader (runs in parallel)
    tokio::spawn(async move {
        let msg = match load_profile_message(storage.clone()).await {
            Ok(msg) => msg,
            Err(e) => profile_task_failed(&storage, e),
        };
        if tx_profile.send(msg).await.is_err() {
            tracing::warn!("Failed to send profile message: receiver dropped");
        }
    });
}

/// Spawn the single serialized profile-save writer.
///
/// Every profile save — mid-session (`ProgressState::save_immediate`/
/// `save_debounced`) and the final exit-time save — is funneled through
/// the returned sender rather than writing directly. The writer processes
/// requests strictly one at a time, awaiting each write's completion
/// before starting the next, so a later save can never be raced by (and
/// lose to) an earlier one still in flight. Each write's outcome is
/// reported back via `result_tx` as [`DataLoadMessage::ProfileSaved`] or
/// [`DataLoadMessage::ProfileSaveError`].
///
/// The caller is responsible for draining on shutdown: drop every clone
/// of the returned sender (closing the queue) and `.await` the returned
/// [`JoinHandle`] so the process does not exit while a write is still
/// queued or in progress. That `JoinHandle` resolves to a
/// [`SaveWriterOutcome`] carrying the *last* processed save's outcome —
/// `Ok(())` from `JoinHandle::await` only means the writer task didn't
/// panic, not that the save actually succeeded, and nothing else drains
/// `result_tx` once the caller has stopped polling it (as is the case once
/// the application's event loop has exited), so this is the only place
/// that outcome is still observable.
///
/// # Examples
///
/// ```no_run
/// use helix_trainer::async_state::{SaveRequest, SaveWriterOutcome};
/// use helix_trainer::data_loader::spawn_save_writer;
/// use helix_trainer::gamification::{ProfileStorage, UserProfile};
///
/// #[tokio::main]
/// async fn main() {
///     let (result_tx, _result_rx) = tokio::sync::mpsc::channel(32);
///     let (save_tx, writer_handle) = spawn_save_writer(result_tx);
///
///     save_tx
///         .send(SaveRequest {
///             storage: ProfileStorage::with_path("/tmp/example-profile.json"),
///             profile: UserProfile::new(),
///         })
///         .await
///         .unwrap();
///
///     drop(save_tx);
///     let outcome = writer_handle.await.unwrap();
///     assert!(matches!(outcome, SaveWriterOutcome::LastSaveSucceeded));
/// }
/// ```
pub fn spawn_save_writer(
    result_tx: mpsc::Sender<DataLoadMessage>,
) -> (mpsc::Sender<SaveRequest>, JoinHandle<SaveWriterOutcome>) {
    let (save_tx, mut save_rx) = mpsc::channel::<SaveRequest>(PROFILE_SAVE_QUEUE_CAPACITY);

    let handle = tokio::spawn(async move {
        let mut outcome = SaveWriterOutcome::NoRequestsProcessed;
        while let Some(request) = save_rx.recv().await {
            let SaveRequest { storage, profile } = request;
            let result = tokio::task::spawn_blocking(move || storage.save(&profile)).await;
            let failure = match result {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(SaveFailure::Storage(e)),
                Err(e) => Some(SaveFailure::Task(e)),
            };
            let (msg, latest_outcome) = failure.map_or_else(
                || {
                    (
                        DataLoadMessage::ProfileSaved,
                        SaveWriterOutcome::LastSaveSucceeded,
                    )
                },
                |failure| {
                    let failure = Arc::new(failure);
                    (
                        DataLoadMessage::ProfileSaveError(Arc::clone(&failure)),
                        SaveWriterOutcome::LastSaveFailed(failure),
                    )
                },
            );
            outcome = latest_outcome;
            if result_tx.send(msg).await.is_err() {
                // The receiver is intentionally gone once the event loop has exited at shutdown.
                tracing::debug!("Profile save result not delivered: receiver dropped");
            }
        }
        outcome
    });

    (save_tx, handle)
}

/// Async scenario loading from embedded data
///
/// Loads scenarios from compile-time embedded TOML content.
/// This eliminates filesystem I/O and ensures consistent behavior
/// regardless of the working directory.
///
/// # Errors
///
/// Returns [`LoaderError::Scenarios`] if scenarios cannot be loaded from embedded data and
/// [`LoaderError::Task`] if the blocking task fails.
pub(crate) async fn load_scenarios_async() -> Result<Vec<Scenario>, LoaderError> {
    tokio::task::spawn_blocking(move || {
        let current_locale = rust_i18n::locale();
        let locale_str: &str = current_locale.as_ref();

        let loader = ScenarioLoader::new();
        loader
            .load_from_embedded(locale_str)
            .map_err(LoaderError::Scenarios)
    })
    .await?
}

/// Async profile loading
///
/// Loads the profile from `storage` on tokio's blocking task pool and
/// reports the outcome as a message. When the load fails, the unreadable
/// file is copied aside (see [`ProfileStorage::quarantine`]) before the
/// failure is reported, so the original can never be lost to a later save.
///
/// # Errors
///
/// Returns an error only if the blocking task itself panics or is cancelled;
/// load failures are reported as [`DataLoadMessage::ProfileError`].
pub(crate) async fn load_profile_message(
    storage: ProfileStorage,
) -> Result<DataLoadMessage, JoinError> {
    tokio::task::spawn_blocking(move || match storage.load() {
        Ok(profile) => DataLoadMessage::ProfileReady(profile),
        Err(error) => profile_refused(&storage, error),
    })
    .await
}

/// The [`DataLoadMessage::ProfileError`] for a profile that could not be loaded or replaced:
/// a throwaway fallback, the backup state of the unreadable file and whether its previous
/// copy can be restored.
fn profile_refused(storage: &ProfileStorage, error: StorageError) -> DataLoadMessage {
    DataLoadMessage::ProfileError {
        error,
        fallback: UserProfile::new(),
        backup: storage.quarantine(),
        previous: storage.previous_copy(),
    }
}

/// The refusal reported when a profile task panicked or was cancelled before it produced a
/// message, so the event loop still receives a [`DataLoadMessage::ProfileError`].
fn profile_task_failed(storage: &ProfileStorage, error: JoinError) -> DataLoadMessage {
    DataLoadMessage::ProfileError {
        error: StorageError::Read {
            doc: Document::Profile,
            path: storage.path().to_path_buf(),
            source: CappedReadError::Io(std::io::Error::other(error)),
        },
        fallback: UserProfile::new(),
        backup: QuarantineOutcome::Failed(Arc::new(
            crate::config::json_store::QuarantineError::Copy {
                source: std::io::Error::other("the profile task failed"),
            },
        )),
        previous: PreviousCopy::Unavailable,
    }
}

/// Async "Start fresh" for a quarantined profile.
///
/// Runs [`ProfileStorage::start_fresh`] on tokio's blocking task pool: the
/// unreadable file's backup is kept, a new profile is written, and the
/// outcome is reported as [`DataLoadMessage::ProfileReady`] carrying the
/// fresh profile, which the usual profile-loaded handling then adopts and
/// re-enables saving with. A refusal (no backup could be made, write error)
/// is reported as [`DataLoadMessage::ProfileError`] and the file is untouched.
///
/// # Errors
///
/// Returns an error only if the blocking task itself panics or is cancelled.
///
/// # Examples
///
/// ```
/// use helix_trainer::async_state::DataLoadMessage;
/// use helix_trainer::data_loader::start_fresh_profile_message;
/// use helix_trainer::gamification::ProfileStorage;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("profile.json");
/// std::fs::write(&path, "{broken").unwrap();
///
/// let msg = start_fresh_profile_message(ProfileStorage::with_path(&path))
///     .await
///     .unwrap();
/// assert!(matches!(msg, DataLoadMessage::ProfileReady(p) if p.level == 1));
/// # }
/// ```
pub async fn start_fresh_profile_message(
    storage: ProfileStorage,
) -> Result<DataLoadMessage, JoinError> {
    tokio::task::spawn_blocking(move || match storage.start_fresh() {
        Ok(profile) => DataLoadMessage::ProfileReady(profile),
        Err(error) => profile_refused(&storage, error),
    })
    .await
}

/// Async "Restore previous" for a quarantined profile.
///
/// Runs [`ProfileStorage::restore_previous`] on tokio's blocking task pool: the previous
/// copy is re-validated, written over the unreadable profile and reported as
/// [`DataLoadMessage::ProfileReady`], which the usual profile-loaded handling adopts and
/// re-enables saving with. A refusal (no backup, previous copy gone or invalid, write error)
/// is reported as [`DataLoadMessage::ProfileError`] and every file is untouched.
///
/// # Errors
///
/// Returns an error only if the blocking task itself panics or is cancelled.
///
/// # Examples
///
/// ```
/// use helix_trainer::async_state::DataLoadMessage;
/// use helix_trainer::data_loader::restore_previous_profile_message;
/// use helix_trainer::gamification::{ProfileStorage, UserProfile};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let dir = tempfile::tempdir().unwrap();
/// let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
/// let mut profile = UserProfile::new();
/// profile.add_xp(500);
/// storage.save(&profile).unwrap();
/// storage.save(&UserProfile::new()).unwrap();
/// std::fs::write(storage.path(), "{broken").unwrap();
///
/// let msg = restore_previous_profile_message(storage).await.unwrap();
/// assert!(matches!(msg, DataLoadMessage::ProfileReady(p) if p.total_xp == 500));
/// # }
/// ```
pub async fn restore_previous_profile_message(
    storage: ProfileStorage,
) -> Result<DataLoadMessage, JoinError> {
    tokio::task::spawn_blocking(move || match storage.restore_previous() {
        Ok(profile) => DataLoadMessage::ProfileReady(profile),
        Err(error) => profile_refused(&storage, error),
    })
    .await
}

/// Locale whose quest templates stand in when the active locale has none
/// (the `rust_i18n` fallback locale).
const FALLBACK_QUEST_LOCALE: &str = "en";

/// Async quest registry loading
///
/// Parses the embedded quest templates for `locale` on tokio's blocking task
/// pool, falling back to the English templates when `locale` has none.
///
/// # Errors
///
/// Returns [`LoaderError::QuestRegistry`] if neither `locale` nor the fallback locale yields a
/// valid registry, and [`LoaderError::Task`] if the blocking task fails.
pub(crate) async fn load_quest_registry_async(
    locale: &str,
) -> Result<QuestTemplateRegistry, LoaderError> {
    let locale = locale.to_string();
    tokio::task::spawn_blocking(move || {
        QuestTemplateRegistry::from_embedded(&locale)
            .or_else(|first| {
                if locale == FALLBACK_QUEST_LOCALE {
                    return Err(first);
                }
                tracing::warn!(%locale, "No quest templates for locale, using the fallback");
                QuestTemplateRegistry::from_embedded(FALLBACK_QUEST_LOCALE)
            })
            .map_err(LoaderError::QuestRegistry)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const DATA_LOADING_TIMEOUT: Duration = Duration::from_secs(5);

    #[test]
    #[cfg_attr(miri, ignore)]
    fn save_writer_does_not_warn_when_the_result_receiver_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));

        let logs = crate::testing::log_capture::capture_logs(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let (result_tx, result_rx) = mpsc::channel(1);
                drop(result_rx);
                let (save_tx, handle) = spawn_save_writer(result_tx);
                save_tx
                    .send(SaveRequest {
                        storage,
                        profile: UserProfile::new(),
                    })
                    .await
                    .unwrap();
                drop(save_tx);
                assert!(matches!(
                    handle.await.unwrap(),
                    SaveWriterOutcome::LastSaveSucceeded
                ));
            });
        });

        assert!(!logs.contains("WARN"), "unexpected warning: {logs}");
    }

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
                    "Error should mention scenarios or loading: {error_msg}"
                );
            }
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_profile_message_missing_file_is_ready() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));

        let msg = load_profile_message(storage).await.unwrap();

        assert!(matches!(msg, DataLoadMessage::ProfileReady(p) if p.level == 1));
    }

    /// Regression test for #440: an unparsable profile is reported as an
    /// error with a backup copy, and the original file is left untouched.
    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_profile_message_unparsable_file_is_backed_up_and_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.json");
        std::fs::write(&path, "{broken").unwrap();

        let msg = load_profile_message(ProfileStorage::with_path(&path))
            .await
            .unwrap();

        let DataLoadMessage::ProfileError { backup, .. } = msg else {
            panic!("expected ProfileError, got {msg:?}");
        };
        let QuarantineOutcome::BackedUp(backup_path) = backup else {
            panic!("expected a backup, got {backup:?}");
        };
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken");
        assert_eq!(std::fs::read_to_string(backup_path).unwrap(), "{broken");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_start_fresh_message_keeps_backup_and_reports_fresh_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.json");
        std::fs::write(&path, "{broken").unwrap();
        let storage = ProfileStorage::with_path(&path);
        let DataLoadMessage::ProfileError { backup, .. } =
            load_profile_message(storage.clone()).await.unwrap()
        else {
            panic!("expected a load failure");
        };

        let msg = start_fresh_profile_message(storage.clone()).await.unwrap();

        assert!(matches!(msg, DataLoadMessage::ProfileReady(p) if p.total_xp == 0));
        assert!(storage.load().is_ok());
        let QuarantineOutcome::BackedUp(backup_path) = backup else {
            panic!("expected a backup");
        };
        assert_eq!(std::fs::read_to_string(backup_path).unwrap(), "{broken");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_start_fresh_message_without_a_profile_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));

        let msg = start_fresh_profile_message(storage.clone()).await.unwrap();

        assert!(matches!(msg, DataLoadMessage::ProfileError { .. }));
        assert!(!storage.exists());
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
                    "Quest registry should load successfully. Error: {e}. \
                     Make sure quests/en/daily.toml exists."
                );
            }
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_quest_registry_async_falls_back_for_unknown_locale() {
        let registry = load_quest_registry_async("zz").await.unwrap();
        assert!(!registry.is_empty());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_spawn_data_loaders_sends_quest_registry() {
        let (tx, mut rx) = mpsc::channel(32);
        spawn_data_loaders(tx, ProfileStorage::for_test());

        let mut quest_registry_received = false;
        for _ in 0..3 {
            if let Ok(Some(DataLoadMessage::QuestRegistryReady(registry))) =
                tokio::time::timeout(DATA_LOADING_TIMEOUT, rx.recv()).await
            {
                assert!(!registry.is_empty());
                quest_registry_received = true;
            }
        }
        assert!(quest_registry_received);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_spawn_data_loaders() {
        let (tx, mut rx) = mpsc::channel(32);

        spawn_data_loaders(tx, ProfileStorage::for_test());

        // Collect messages (scenarios, quest registry and profile)
        let mut scenarios_received = false;
        let mut profile_received = false;

        // Wait for both messages with timeout
        for _ in 0..3 {
            if let Ok(msg) = tokio::time::timeout(DATA_LOADING_TIMEOUT, rx.recv()).await {
                match msg {
                    Some(
                        DataLoadMessage::ScenariosReady(_) | DataLoadMessage::ScenariosError(_),
                    ) => {
                        scenarios_received = true;
                    }
                    Some(
                        DataLoadMessage::ProfileReady(_) | DataLoadMessage::ProfileError { .. },
                    ) => {
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

    /// S1 / FR-103, SC-011: the loader shares the application's storage handle, so the first
    /// save after startup keeps the profile that just loaded as `.prev`.
    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_first_save_after_spawn_data_loaders_keeps_the_loaded_profile_as_prev() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
        let mut original = UserProfile::new();
        original.add_xp(700);
        storage.save(&original).unwrap();
        let loaded_bytes = std::fs::read(storage.path()).unwrap();

        // A fresh handle, as `main` builds it, so nothing is held before the loader runs.
        let app_storage = ProfileStorage::with_path(storage.path());
        let (tx, mut rx) = mpsc::channel(32);
        spawn_data_loaders(tx, app_storage.clone());
        let mut profile = loop {
            match tokio::time::timeout(DATA_LOADING_TIMEOUT, rx.recv()).await {
                Ok(Some(DataLoadMessage::ProfileReady(profile))) => break profile,
                Ok(Some(_)) => {}
                other => panic!("no profile message: {:?}", other.map(|m| m.is_some())),
            }
        };
        assert_eq!(profile.total_xp, 700);

        profile.add_xp(50);
        app_storage.save(&profile).unwrap();

        let prev = dir.path().join("profile.json.prev");
        assert_eq!(std::fs::read(prev).unwrap(), loaded_bytes);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_restore_previous_profile_message_restores_or_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
        let mut profile = UserProfile::new();
        profile.add_xp(500);
        storage.save(&profile).unwrap();
        storage.save(&UserProfile::new()).unwrap();
        std::fs::write(storage.path(), "{broken").unwrap();

        let message = restore_previous_profile_message(storage.clone())
            .await
            .unwrap();
        assert!(matches!(message, DataLoadMessage::ProfileReady(p) if p.total_xp == 500));

        std::fs::write(storage.path(), "{broken again").unwrap();
        std::fs::write(storage.path().with_added_extension("prev"), "{also broken").unwrap();
        let message = restore_previous_profile_message(storage.clone())
            .await
            .unwrap();
        assert!(matches!(
            message,
            DataLoadMessage::ProfileError {
                previous: PreviousCopy::Unavailable,
                ..
            }
        ));
        assert_eq!(
            std::fs::read_to_string(storage.path()).unwrap(),
            "{broken again"
        );
    }
}
