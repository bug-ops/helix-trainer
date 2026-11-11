//! Messages exchanged between the background data loaders, the serialized
//! profile save writer and the event loop.

use crate::config::Scenario;
use crate::gamification::{ProfileStorage, QuarantineOutcome, QuestTemplateRegistry, UserProfile};

/// Messages sent from background loaders to main event loop
///
/// These messages communicate the results of async operations to the
/// main application state.
#[derive(Debug)]
pub enum DataLoadMessage {
    /// Scenarios loaded successfully
    ScenariosReady(Vec<Scenario>),

    /// Scenarios failed to load
    ScenariosError(String),

    /// Profile loaded successfully
    ProfileReady(UserProfile),

    /// Profile failed to load (with fallback new profile)
    ///
    /// The fallback is for display only: the application must not write it
    /// over the unreadable file (see `ProfilePersistence::Quarantined`).
    ProfileError {
        /// Human-readable load failure.
        error: String,
        /// Fresh profile shown while saves stay disabled.
        fallback: UserProfile,
        /// Whether the unreadable file was preserved as a backup copy.
        backup: QuarantineOutcome,
    },

    /// Quest template registry parsed for the active locale
    QuestRegistryReady(QuestTemplateRegistry),

    /// Quest template registry failed to load
    QuestRegistryError(String),

    /// Profile saved successfully
    ProfileSaved,

    /// Profile save failed
    ProfileSaveError(String),
}

/// A pending profile write, sent to the serialized save writer spawned by
/// [`crate::data_loader::spawn_save_writer`].
///
/// Every save (mid-session or on exit) is funneled through that writer's
/// queue rather than writing directly, so writes execute strictly in the
/// order they were requested. Without this, each save spawning its own
/// independent write task lets `fs::rename` calls complete out of order,
/// so a save carrying older data can race ahead of — and silently
/// overwrite — one carrying newer data (most concretely: a mid-session
/// save still in flight when the app exits could finish writing after,
/// and clobber, the exit-time save).
#[derive(Debug)]
pub struct SaveRequest {
    /// Where to write the profile.
    pub storage: ProfileStorage,

    /// The profile snapshot to persist.
    pub profile: UserProfile,
}

/// Final outcome reported by [`crate::data_loader::spawn_save_writer`]'s
/// `JoinHandle` once its queue closes and drains.
///
/// The `JoinHandle` alone only tells a caller that the writer task didn't
/// panic — not whether the save it cared about actually succeeded. This
/// carries the real answer for the *last* request the writer processed,
/// which is what the application's exit path needs: it always enqueues its
/// final snapshot last, so "last processed" and "the exit save" are the
/// same request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveWriterOutcome {
    /// The writer's queue closed before it ever received a request.
    NoRequestsProcessed,

    /// The most recently processed save succeeded.
    LastSaveSucceeded,

    /// The most recently processed save failed, with its error message.
    LastSaveFailed(String),
}
