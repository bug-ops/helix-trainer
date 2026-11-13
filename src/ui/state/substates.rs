//! Sub-structures for organizing `AppState`
//!
//! This module defines focused sub-structures that group related fields
//! from `AppState`, improving code organization and maintainability.

use crate::async_state::SaveRequest;
use crate::config::AppConfig;
use crate::constants::PROFILE_SAVE_DEBOUNCE;
use crate::game::ScenarioCollection;
use crate::gamification::{
    PreviousCopy, ProfileStorage, QuarantineOutcome, QuestRegistrySlot, UserProfile,
};
use crate::ids::NormalizedCommand;
use crate::input::keymap::KeymapOverlay;
use crate::learning::{PerformanceTracker, Scheduler};
use crate::sound::SoundManager;
#[cfg(test)]
use crate::sound::{SoundConfig, Volume};
use crate::time::{Clock, MonotonicClock, SystemMonotonicClock};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::ui::notification::NotificationQueue;

/// UI rendering and display state
///
/// Contains global UI state that persists across screen transitions.
/// Screen-specific data is stored in `TypedScreen` variants.
#[derive(Debug)]
pub struct UIState {
    /// Whether the application is running
    pub running: bool,

    /// Whether to show key history popup (global setting)
    pub show_key_history: bool,

    /// Terminal size, updated from the resize event; list layouts are clamped against it
    pub viewport: crate::ui::layout::Viewport,

    /// Notification queue for transient messages (level-up, achievements, etc.)
    pub notifications: NotificationQueue,

    /// Side effects queued during one `update`; drained into its `effects` sink at the end
    pub(crate) effects: Vec<super::Effect>,

    /// Last selected scenario index in menu (session-only, for position persistence)
    pub last_menu_selected: usize,

    /// Last scroll offset in menu (session-only, for position persistence)
    pub last_menu_scroll: usize,
}

impl UIState {
    /// Create new `UIState` with default values, timing notifications on the system clock
    pub fn new() -> Self {
        Self::with_clock(Arc::new(SystemMonotonicClock))
    }

    /// Create new `UIState` whose notifications expire on `clock`
    pub fn with_clock(clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            running: true,
            show_key_history: true,
            viewport: crate::ui::layout::Viewport::MINIMUM,
            notifications: NotificationQueue::with_clock(clock),
            effects: Vec::new(),
            last_menu_selected: 0,
            last_menu_scroll: 0,
        }
    }
}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

/// Active game state (session, scenarios)
pub struct GameState {
    /// All available scenarios with filtering and sorting
    pub scenario_collection: ScenarioCollection,
}

impl GameState {
    /// Create new `GameState` with scenarios
    pub fn new(scenarios: Vec<crate::config::Scenario>) -> Self {
        Self {
            scenario_collection: ScenarioCollection::new(scenarios),
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            scenario_collection: ScenarioCollection::new(vec![]),
        }
    }
}

impl std::fmt::Debug for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameState")
            .field("scenario_count", &self.scenario_collection.count())
            .finish()
    }
}

/// What is known about a profile that failed to load: whether its file was preserved and
/// whether a previous copy can be restored.
#[derive(Debug, Clone)]
pub struct Quarantine {
    /// Whether the unreadable file was preserved as a backup copy.
    pub backup: QuarantineOutcome,

    /// Whether `profile.json.prev` can be restored.
    pub previous: PreviousCopy,
}

impl Quarantine {
    /// Whether Start fresh is offered: only after the unreadable file was preserved, so the
    /// original is never discarded.
    pub fn can_start_fresh(&self) -> bool {
        matches!(self.backup, QuarantineOutcome::BackedUp(_))
    }

    /// Whether Restore previous is offered: a backup exists and the previous copy loads.
    pub fn can_restore_previous(&self) -> bool {
        self.can_start_fresh() && self.previous == PreviousCopy::Available
    }
}

/// Whether the in-memory profile may be written to disk.
///
/// Every save path consults this, so a placeholder or fallback profile can
/// never overwrite the user's real `profile.json`.
#[derive(Debug, Clone)]
pub enum ProfilePersistence {
    /// The real profile is still loading; the in-memory one is a placeholder.
    Loading,

    /// The profile was loaded (or legitimately created on first launch).
    Writable,

    /// Loading failed; the in-memory profile is a throwaway fallback and
    /// saving is disabled for the rest of the session.
    Quarantined(Quarantine),
}

impl ProfilePersistence {
    /// Whether saves are allowed.
    pub fn is_writable(&self) -> bool {
        matches!(self, Self::Writable)
    }

    /// The quarantine details while loading has failed.
    pub fn quarantine(&self) -> Option<&Quarantine> {
        match self {
            Self::Quarantined(quarantine) => Some(quarantine),
            Self::Loading | Self::Writable => None,
        }
    }
}

/// User progress (profile, learning, achievements)
pub struct ProgressState {
    /// User profile with XP, level, achievements
    pub profile: UserProfile,

    /// Performance tracker for spaced repetition
    pub performance_tracker: PerformanceTracker,

    /// Scheduler for review sessions
    pub scheduler: Scheduler,

    /// Profile storage handler
    pub storage: ProfileStorage,

    /// Daily-quest template registry and its load state
    pub quest_registry: QuestRegistrySlot,

    /// Scenarios completed today
    pub scenarios_completed_today: usize,

    /// Commands used today (for exploration quests)
    pub commands_used_today: HashSet<NormalizedCommand>,

    /// Practice time not yet credited to time quests (always under one minute)
    pub time_remainder: std::time::Duration,

    /// Session start time
    pub session_start_time: Instant,

    /// Last save time (for debouncing)
    pub last_save_time: Option<Instant>,

    /// Sound manager for audio feedback
    pub sound_manager: SoundManager,

    /// Source of the current time, injected for testability
    ///
    /// Private: construction is routed exclusively through [`ProgressState::new`] /
    /// [`ProgressState::with_clocks`] so that every clock-consuming field
    /// (`performance_tracker`, `scheduler`) is guaranteed to share this same instance.
    /// Reassigning the field directly after construction would not re-point those fields
    /// and would silently reintroduce clock drift between them.
    clock: Arc<dyn Clock>,

    /// Source of monotonic time for the session timer and the save debounce
    monotonic: Arc<dyn MonotonicClock>,

    /// Channel to the serialized save writer (see
    /// [`crate::data_loader::spawn_save_writer`]), used to dispatch profile
    /// saves off the event-loop thread.
    ///
    /// `None` (the default) makes [`ProgressState::save_immediate`] and
    /// [`ProgressState::save_debounced`] fall back to a synchronous save —
    /// this is what every test fixture gets, since dispatching to a writer
    /// task without one running to receive the request would be pointless.
    /// The real application wires this up via
    /// [`ProgressState::set_save_channel`] during startup.
    save_tx: Option<mpsc::Sender<SaveRequest>>,

    /// Whether `profile` may be persisted; see [`ProfilePersistence`].
    persistence: ProfilePersistence,
}

impl ProgressState {
    /// Create new `ProgressState` backed by the system clocks
    pub fn new(
        profile: UserProfile,
        performance_tracker: PerformanceTracker,
        storage: ProfileStorage,
    ) -> Self {
        Self::with_clocks(
            profile,
            performance_tracker,
            storage,
            Arc::new(crate::time::SystemClock),
            Arc::new(SystemMonotonicClock),
        )
    }

    /// Create new `ProgressState` with explicit wall and monotonic clocks
    ///
    /// `clock` is propagated to every clock-consuming field (`performance_tracker`,
    /// `scheduler`) so they observe the same time, not just `self.clock`; `monotonic`
    /// times the session and the save debounce.
    pub fn with_clocks(
        profile: UserProfile,
        mut performance_tracker: PerformanceTracker,
        storage: ProfileStorage,
        clock: Arc<dyn Clock>,
        monotonic: Arc<dyn MonotonicClock>,
    ) -> Self {
        // Initialize sound manager from profile config
        let mut sound_manager = SoundManager::new(profile.sound_config().clone());
        // Try to initialize audio (graceful failure)
        if let Err(e) = sound_manager.try_init() {
            tracing::warn!("Audio initialization failed: {e}. Running in silent mode.");
        }

        performance_tracker.set_clock(clock.clone());

        Self {
            profile,
            performance_tracker,
            scheduler: Scheduler::with_clock(clock.clone()),
            storage,
            quest_registry: QuestRegistrySlot::default(),
            scenarios_completed_today: 0,
            commands_used_today: HashSet::new(),
            time_remainder: std::time::Duration::ZERO,
            session_start_time: monotonic.now(),
            last_save_time: None,
            sound_manager,
            clock,
            monotonic,
            save_tx: None,
            persistence: ProfilePersistence::Loading,
        }
    }

    /// Test fixture helper: treat the constructor's profile as already loaded
    /// (production starts in [`ProfilePersistence::Loading`], fail-safe).
    #[cfg(test)]
    pub(crate) fn loaded(mut self) -> Self {
        self.mark_profile_loaded();
        self
    }

    /// Current persistence state of the profile.
    pub fn persistence(&self) -> &ProfilePersistence {
        &self.persistence
    }

    /// Declare the in-memory profile a placeholder until the real one loads.
    ///
    /// While loading, every save (including the exit-time one) is refused, so
    /// quitting before the loader reports back cannot overwrite the real file.
    pub fn begin_profile_load(&mut self) {
        self.persistence = ProfilePersistence::Loading;
    }

    /// Declare `profile` the successfully loaded profile: enable saving and
    /// apply its saved sound settings to the sound manager.
    pub fn mark_profile_loaded(&mut self) {
        self.persistence = ProfilePersistence::Writable;
        let config = &self.profile.sound_config();
        self.sound_manager.set_volume(config.volume());
        self.sound_manager.set_enabled(config.enabled());
    }

    /// Declare `profile` a throwaway fallback after a failed load: saving is
    /// disabled for the rest of the session.
    pub fn quarantine_profile(&mut self, quarantine: Quarantine) {
        self.persistence = ProfilePersistence::Quarantined(quarantine);
    }

    /// Toggle sound and, when the profile is saveable, record the new state
    /// in it and mark it dirty so the next save persists it.
    ///
    /// Returns the new enabled state.
    pub fn toggle_sound(&mut self) -> bool {
        let enabled = self.sound_manager.toggle();
        if self.persistence.is_writable() {
            self.profile.sound_config_mut().set_enabled(enabled);
            self.mark_unsaved();
        }
        enabled
    }

    /// Get the current time from the injected clock
    pub fn now(&self) -> chrono::DateTime<chrono::Utc> {
        self.clock.now()
    }

    /// Get the current monotonic instant from the injected clock
    pub fn monotonic_now(&self) -> Instant {
        self.monotonic.now()
    }

    /// Get the player's current day from the injected clock
    pub fn today(&self) -> crate::time::PlayerDay {
        self.clock.player_day()
    }

    /// The current time with the player's zone, from the injected clock
    pub fn moment(&self) -> crate::time::Moment {
        self.clock.moment()
    }

    /// The player's local day at `instant`, from the injected clock
    pub fn day_of(&self, instant: chrono::DateTime<chrono::Utc>) -> crate::time::PlayerDay {
        self.clock.day_of(instant)
    }

    /// Get a shared handle to the injected clock, e.g. to construct another clock-consuming
    /// type (such as `PerformanceTracker::from_stats_with_clock`) that shares the same time.
    pub fn clock(&self) -> Arc<dyn Clock> {
        self.clock.clone()
    }

    /// Wire up the channel used to dispatch profile saves to the serialized
    /// save writer (see [`crate::data_loader::spawn_save_writer`]), off the
    /// event-loop thread.
    ///
    /// Once set, [`ProgressState::save_immediate`] and
    /// [`ProgressState::save_debounced`] hand the write off to the writer
    /// task instead of running it inline, and learn the outcome later via
    /// [`crate::async_state::DataLoadMessage::ProfileSaved`] /
    /// [`crate::async_state::DataLoadMessage::ProfileSaveError`].
    pub fn set_save_channel(&mut self, tx: mpsc::Sender<SaveRequest>) {
        self.save_tx = Some(tx);
    }

    /// Drop this instance's clone of the save channel, reverting
    /// [`ProgressState::save_immediate`]/[`ProgressState::save_debounced`]
    /// to their synchronous fallback.
    ///
    /// The serialized save writer's receive loop only exits once every
    /// clone of its sender is dropped — including this one. Call this
    /// explicitly on the exit path (rather than relying solely on dropping
    /// the whole `ProgressState`/`AppState`) so that invariant holds
    /// regardless of how long the surrounding value happens to stay alive.
    pub fn close_save_channel(&mut self) {
        self.save_tx = None;
    }

    /// Check if enough time has passed since last save
    pub fn should_save(&self) -> bool {
        self.last_save_time.is_none_or(|last| {
            self.monotonic.now().saturating_duration_since(last) >= PROFILE_SAVE_DEBOUNCE
        })
    }

    /// Mark save as completed
    pub fn mark_saved(&mut self) {
        self.last_save_time = Some(self.monotonic.now());
    }

    /// Mark the profile as needing a save again.
    ///
    /// Used when an off-thread save dispatched by
    /// [`ProgressState::save_immediate`] later turns out to have failed
    /// (`DataLoadMessage::ProfileSaveError`): the optimistic
    /// [`ProgressState::mark_saved`] call made at dispatch time was wrong,
    /// so the debounce gate must be reset — otherwise the next attempt
    /// would be skipped for up to `PROFILE_SAVE_DEBOUNCE` as if the
    /// failed write had actually succeeded.
    pub fn mark_unsaved(&mut self) {
        self.last_save_time = None;
    }

    /// Synchronously write the profile to disk, bypassing off-thread dispatch.
    fn save_sync(&mut self) -> Result<(), crate::config::json_store::StorageError> {
        if !self.persistence.is_writable() {
            return Ok(());
        }
        self.storage.save(&self.profile)?;
        self.mark_saved();
        Ok(())
    }

    /// Sync FSRS performance data from the live tracker and persist the profile immediately.
    ///
    /// This is the single source of truth for saving progress: every call site that
    /// persists the profile mid-session must go through this method (or
    /// [`ProgressState::save_debounced`]) rather than calling `storage.save` directly,
    /// otherwise `profile.performance_data()` silently goes stale.
    ///
    /// When a save channel has been wired up via
    /// [`ProgressState::set_save_channel`] (the case for the real app, never
    /// for test fixtures), the write is handed off to the serialized save
    /// writer rather than performed inline — important because `fsync` can
    /// take tens of milliseconds and this is called from the TUI
    /// event-loop thread on every review answer, session completion, and
    /// mini-game round. Routing every save through one writer (rather than
    /// each spawning its own independent write task) also guarantees
    /// writes apply in the order they were requested, so a save can never
    /// be raced by, and lose to, an earlier one still in flight.
    ///
    /// If the writer has shut down, this falls back to a synchronous write
    /// so a save is never silently dropped; the same fallback applies when
    /// no channel is configured at all (mainly tests), so callers there
    /// still observe the write completing before this method returns. If
    /// the writer's queue is merely full (pathologically behind), this
    /// save is skipped rather than written inline — see the comment on the
    /// `Full` arm below for why an inline write there would be actively
    /// harmful, not just redundant.
    ///
    /// # Errors
    ///
    /// Returns an error if the save could not be dispatched or (on the
    /// synchronous fallback path) failed outright — the async path's real
    /// write outcome otherwise arrives later as a
    /// [`crate::async_state::DataLoadMessage`].
    pub fn save_immediate(&mut self) -> Result<(), crate::config::json_store::StorageError> {
        if !self.persistence.is_writable() {
            tracing::debug!(state = ?self.persistence, "Profile save skipped: profile is not saveable");
            return Ok(());
        }
        self.profile
            .set_performance_data(self.performance_tracker.stats_clone());

        let Some(tx) = &self.save_tx else {
            return self.save_sync();
        };

        let request = SaveRequest {
            storage: self.storage.clone(),
            profile: self.profile.clone(),
        };

        match tx.try_send(request) {
            Ok(()) => {
                // Mark saved optimistically so a debounced save requested
                // while this one is still queued/in flight doesn't enqueue
                // a redundant write; the real outcome is confirmed once the
                // writer completes it and sends
                // `ProfileSaved`/`ProfileSaveError` back (the latter also
                // calls `mark_unsaved` to undo this).
                self.mark_saved();
                Ok(())
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // The writer is pathologically behind, with up to
                // `PROFILE_SAVE_QUEUE_CAPACITY` older snapshots still
                // queued ahead of this one. Writing inline here (as the
                // `Closed` arm does) would race ahead of those — the
                // writer would then replay the stale queued snapshots
                // *over* this fresher one, exactly the unordered-write bug
                // this whole serialized-writer design exists to prevent.
                // Skipping is safe: the profile is cumulative in-memory
                // state, and the exit path always enqueues its snapshot
                // last regardless of debounce/queue state, so this data
                // isn't lost — only deferred to the next save that
                // successfully enqueues.
                self.mark_unsaved();
                Ok(())
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // The writer task is gone: fall back to a synchronous
                // write rather than silently losing this save.
                self.save_sync()
            }
        }
    }

    /// Save the profile only if the debounce interval has elapsed since the last save.
    ///
    /// Syncs FSRS performance data from the live tracker before writing, same as
    /// [`ProgressState::save_immediate`].
    ///
    /// # Errors
    ///
    /// Returns an error if the save operation fails.
    pub fn save_debounced(&mut self) -> Result<(), crate::config::json_store::StorageError> {
        if !self.should_save() {
            return Ok(());
        }
        self.save_immediate()
    }

    /// Sync FSRS data and snapshot the profile for the application's
    /// exit-time save, without performing any I/O.
    ///
    /// The caller (`main`) must enqueue the returned [`SaveRequest`] as the
    /// *last* item sent to the serialized save writer, then drop every
    /// sender clone and await the writer's `JoinHandle` before the process
    /// exits. Because the writer processes requests strictly in order, and
    /// nothing enqueues further requests once the event loop has stopped,
    /// this guarantees the final, most up-to-date snapshot is written last
    /// — and so is never overwritten by an older mid-session save that was
    /// still in flight when the app exited.
    ///
    /// Returns `None` while the profile is not saveable (still loading, or
    /// its load failed): the file on disk must stay as it is.
    pub fn prepare_final_save_request(&mut self) -> Option<SaveRequest> {
        if !self.persistence.is_writable() {
            return None;
        }
        self.profile
            .set_performance_data(self.performance_tracker.stats_clone());
        Some(SaveRequest {
            storage: self.storage.clone(),
            profile: self.profile.clone(),
        })
    }
}

impl std::fmt::Debug for ProgressState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressState")
            .field("profile", &self.profile)
            .field("performance_tracker", &"<PerformanceTracker>")
            .field("scenarios_completed_today", &self.scenarios_completed_today)
            .field("commands_used_today", &self.commands_used_today.len())
            .finish_non_exhaustive()
    }
}

/// Application configuration (persistent settings and the resolved keymap)
#[derive(Debug, Clone)]
pub struct ConfigState {
    /// Persistent configuration (loaded from/saved to disk)
    pub persistent: AppConfig,

    /// Resolved keymap overlay for gameplay key translation.
    ///
    /// [`KeymapOverlay::identity`] (the default) when `persistent.use_helix_keymap`
    /// is `false` or no config could be loaded — every lookup misses, so
    /// gameplay input is unaffected. Populated once at startup in `main.rs`;
    /// menus, results, and filter screens never consult it.
    pub keymap: KeymapOverlay,

    /// What [`keymap`](Self::keymap) resolution applied and ignored from
    /// the user's Helix config, retained for the lifetime of the session
    /// (not just the auto-dismissing startup toast built from it) so a
    /// future status screen can render it, and so it survives past
    /// whatever `tracing` line accompanied it at startup.
    pub keymap_report: crate::input::keymap::KeymapReport,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            persistent: AppConfig::default(),
            keymap_report: crate::input::keymap::KeymapReport::default(),
            keymap: KeymapOverlay::identity(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::AttemptOutcome;
    use crate::testing::cid;
    use std::assert_matches;

    fn writable_progress() -> ProgressState {
        ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
        )
        .loaded()
    }

    /// Regression test for #440: neither a loading placeholder nor a
    /// quarantined fallback profile may ever reach the disk.
    #[test]
    fn test_non_writable_profile_refuses_every_save_path() {
        for state in [
            ProfilePersistence::Loading,
            ProfilePersistence::Quarantined(crate::testing::quarantine(
                crate::testing::failed_backup(),
            )),
        ] {
            let mut progress = writable_progress();
            match &state {
                ProfilePersistence::Loading => progress.begin_profile_load(),
                ProfilePersistence::Quarantined(q) => progress.quarantine_profile(q.clone()),
                ProfilePersistence::Writable => unreachable!(),
            }

            progress.save_immediate().unwrap();
            progress.save_debounced().unwrap();

            assert!(!progress.storage.exists(), "{state:?} must not write");
            assert!(progress.prepare_final_save_request().is_none());
        }
    }

    /// Fail-safe default: a freshly constructed `ProgressState` must not be
    /// saveable until a loaded profile is declared.
    #[test]
    fn test_new_progress_state_is_not_saveable_until_loaded() {
        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
        );

        assert_matches!(progress.persistence(), ProfilePersistence::Loading);
        progress.save_immediate().unwrap();
        assert!(!progress.storage.exists());
        assert!(progress.prepare_final_save_request().is_none());
    }

    #[test]
    fn test_mark_profile_loaded_applies_saved_sound_config_clamped() {
        let mut progress = writable_progress();
        progress.begin_profile_load();
        progress.profile.set_sound_config(
            serde_json::from_str(r#"{"volume": 12.0, "enabled": false}"#).unwrap(),
        );

        progress.mark_profile_loaded();

        assert!(progress.persistence().is_writable());
        assert!(!progress.sound_manager.config().enabled());
        assert!((progress.sound_manager.config().volume().get() - 1.0).abs() < f32::EPSILON);
        assert!((progress.profile.sound_config().volume().get() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_mark_profile_loaded_rejects_non_finite_volume() {
        let mut progress = writable_progress();
        progress
            .profile
            .set_sound_config(SoundConfig::new(Volume::new(f32::NAN), true));

        progress.mark_profile_loaded();

        assert!(
            (progress.sound_manager.config().volume().get() - SoundConfig::DEFAULT_VOLUME).abs()
                < f32::EPSILON
        );
    }

    /// Regression test for #459: the toggle is mirrored into the profile and
    /// the profile is marked dirty so it is persisted.
    #[test]
    fn test_toggle_sound_mirrors_into_profile_and_marks_dirty() {
        let mut progress = writable_progress();
        progress.mark_saved();
        assert!(progress.profile.sound_config().enabled());

        let enabled = progress.toggle_sound();

        assert!(!enabled);
        assert!(!progress.profile.sound_config().enabled());
        assert!(progress.should_save());
    }

    #[test]
    fn test_toggle_sound_does_not_touch_quarantined_profile() {
        let mut progress = writable_progress();
        progress.quarantine_profile(crate::testing::quarantine(crate::testing::failed_backup()));

        progress.toggle_sound();

        assert!(progress.profile.sound_config().enabled());
    }

    #[test]
    fn test_ui_state_new() {
        let ui = UIState::new();
        assert!(ui.running);
        assert!(ui.show_key_history);
        assert_eq!(ui.last_menu_selected, 0);
        assert_eq!(ui.last_menu_scroll, 0);
    }

    #[test]
    fn test_config_state_default() {
        let config = ConfigState::default();
        assert!(!config.persistent.enable_arrow_keys_in_normal_mode);
    }

    #[test]
    fn test_game_state_default() {
        let game = GameState::default();
        assert_eq!(game.scenario_collection.count(), 0);
    }

    #[test]
    fn test_progress_state_should_save_initially_true() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;

        let progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
        )
        .loaded();

        // Should save immediately when no previous save
        assert!(progress.should_save());
    }

    #[test]
    fn test_progress_state_mark_saved_updates_time() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
        )
        .loaded();

        assert!(progress.last_save_time.is_none());
        progress.mark_saved();
        assert!(progress.last_save_time.is_some());
    }

    /// Regression test: `ProgressState::with_clocks` must propagate the injected clock to
    /// every clock-consuming field, not just `self.clock`. Previously `performance_tracker`
    /// kept whatever clock it was constructed with (typically `SystemClock`), so a command
    /// recorded through it stamped `due`/`last_review` from the wall clock while
    /// `scheduler.get_due_reviews` compared against the injected fake clock — due dates
    /// would land far off from what the fake clock reported "now" as.
    #[test]
    fn test_with_clocks_propagates_to_performance_tracker_and_scheduler() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use crate::time::{Clock, FakeClock};
        use std::sync::Arc;

        let clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
        let mut progress = ProgressState::with_clocks(
            UserProfile::new(),
            PerformanceTracker::new(), // built with the default SystemClock
            ProfileStorage::for_test(),
            clock.clone(),
            Arc::new(crate::time::SystemMonotonicClock),
        )
        .loaded();

        progress.performance_tracker.record_attempt(
            &cid("x"),
            std::time::Duration::from_secs(1),
            AttemptOutcome::Succeeded,
            std::time::Duration::from_secs(1),
        );

        // If the tracker still used SystemClock, `last_review` would be wall-clock "now",
        // not the fake clock's fixed instant.
        let perf = progress.performance_tracker.performance(&cid("x")).unwrap();
        assert_eq!(perf.last_review(), clock.now());

        // The scheduler must observe the same fake "now" the tracker recorded against, so
        // a command recorded at the fake clock's current time is immediately due.
        let due = progress
            .scheduler
            .get_due_reviews(&progress.performance_tracker);
        assert!(due.contains(&cid("x")));
    }

    #[test]
    fn test_progress_state_debounce_prevents_immediate_resave() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
        )
        .loaded();

        // First save should be allowed
        assert!(progress.should_save());
        progress.mark_saved();

        // Immediate second save should be blocked (within 5 second window)
        assert!(!progress.should_save());
    }

    /// Compares the fields that `save_immediate`/`save_debounced` are responsible for
    /// syncing. `CommandPerformance` has no `PartialEq` impl, so this checks the subset
    /// of fields that prove the persisted copy reflects the tracker's state.
    fn stats_match(
        a: &crate::learning::PerformanceData,
        b: &crate::learning::PerformanceData,
    ) -> bool {
        a.len() == b.len()
            && a.iter().all(|(k, v)| {
                b.get(k).is_some_and(|other| {
                    other.attempts() == v.attempts()
                        && other.successes() == v.successes()
                        && other.reps() == v.reps()
                })
            })
    }

    #[test]
    fn test_save_immediate_syncs_tracker_stats_into_persisted_profile() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();

        progress.performance_tracker.record_attempt(
            &cid("x"),
            Duration::from_millis(500),
            AttemptOutcome::Succeeded,
            Duration::from_millis(500),
        );

        progress.save_immediate().unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(!persisted.performance_data().is_empty());
        assert!(stats_match(
            persisted.performance_data(),
            &progress.performance_tracker.stats_clone()
        ));
    }

    #[test]
    fn test_save_debounced_does_not_write_within_debounce_window() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();

        // Establish a recent save so the debounce window is active.
        progress.save_immediate().unwrap();
        let after_first_save = std::fs::read_to_string(&profile_path).unwrap();

        // Mutate the tracker after the save; a debounced save within the window
        // must not pick this up.
        progress.performance_tracker.record_attempt(
            &cid("d"),
            Duration::from_millis(500),
            AttemptOutcome::Succeeded,
            Duration::from_millis(500),
        );
        progress.save_debounced().unwrap();

        let after_debounced_call = std::fs::read_to_string(&profile_path).unwrap();
        assert_eq!(
            after_first_save, after_debounced_call,
            "debounced save must not have rewritten the file within the debounce window"
        );
    }

    #[test]
    fn test_save_debounced_writes_once_debounce_elapses() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use crate::time::{FakeClock, FakeMonotonicClock};
        use std::time::Duration;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");
        let monotonic = Arc::new(FakeMonotonicClock::new());

        let mut progress = ProgressState::with_clocks(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
            Arc::new(FakeClock::at("2026-01-15T12:00:00Z")),
            monotonic.clone(),
        )
        .loaded();

        progress.mark_saved();
        assert!(!progress.should_save());
        monotonic.advance(PROFILE_SAVE_DEBOUNCE + Duration::from_secs(1));

        progress.performance_tracker.record_attempt(
            &cid("j"),
            Duration::from_millis(500),
            AttemptOutcome::Succeeded,
            Duration::from_millis(500),
        );

        assert!(progress.should_save());
        progress.save_debounced().unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert!(stats_match(
            persisted.performance_data(),
            &progress.performance_tracker.stats_clone()
        ));
        assert!(!persisted.performance_data().is_empty());
    }

    /// Regression test for #294: once a save channel is wired up,
    /// `save_immediate` must not block the caller on the write/fsync — it
    /// should return immediately and report the outcome later via
    /// `DataLoadMessage::ProfileSaved`.
    #[tokio::test]
    async fn test_save_immediate_dispatches_off_thread_when_channel_configured() {
        use crate::async_state::DataLoadMessage;
        use crate::data_loader::spawn_save_writer;
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();
        let (result_tx, mut result_rx) = mpsc::channel(4);
        let (save_tx, writer_handle) = spawn_save_writer(result_tx);
        progress.set_save_channel(save_tx.clone());
        progress.profile.add_xp(100);

        progress.save_immediate().unwrap();

        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), result_rx.recv())
            .await
            .expect("save result should arrive")
            .expect("channel should not be closed");
        assert_matches!(msg, DataLoadMessage::ProfileSaved);

        let expected_xp = progress.profile.total_xp();
        // Drop every sender clone — including the one `progress` holds
        // internally — so the writer's receive loop sees the channel close
        // and its `JoinHandle` actually resolves.
        drop(save_tx);
        drop(progress);
        writer_handle.await.unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert_eq!(persisted.total_xp(), expected_xp);
    }

    /// Regression test for review finding N1: when the writer's queue is
    /// full, `save_immediate` must skip rather than fall back to an inline
    /// synchronous write — an inline write there would race ahead of the
    /// older snapshots still queued, which the writer would then apply
    /// *after* it, silently reverting to stale data.
    #[tokio::test]
    async fn test_save_immediate_skips_inline_write_when_queue_is_full() {
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();

        // No writer task drains this, and its capacity is 1, so the first
        // dispatch fills it and the second is guaranteed to see `Full`.
        let (save_tx, mut save_rx) = mpsc::channel::<SaveRequest>(1);
        progress.set_save_channel(save_tx);

        progress.profile.set_total_xp(100);
        progress.save_immediate().unwrap();
        assert!(
            !progress.should_save(),
            "the first dispatch marks saved optimistically"
        );

        progress.profile.set_total_xp(999);
        progress.save_immediate().unwrap();

        assert!(
            progress.should_save(),
            "a save skipped because the queue is full must re-dirty the profile"
        );
        assert!(
            !profile_path.exists(),
            "a skipped save must never write inline — that would race ahead of \
             the older snapshot still sitting in the queue"
        );

        let queued = save_rx
            .try_recv()
            .expect("the first request must still be queued");
        assert_eq!(
            queued.profile.total_xp(),
            100,
            "the queued snapshot must be the older one, not overwritten by the skipped save"
        );
    }

    /// Regression test for #294: the debounce gate must still be respected
    /// when saves are dispatched off-thread — a call within the debounce
    /// window must not enqueue a second write.
    #[tokio::test]
    async fn test_save_debounced_skips_dispatch_within_window_when_channel_configured() {
        use crate::data_loader::spawn_save_writer;
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();
        let (result_tx, mut result_rx) = mpsc::channel(4);
        let (save_tx, _writer_handle) = spawn_save_writer(result_tx);
        progress.set_save_channel(save_tx);

        progress.save_immediate().unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), result_rx.recv())
            .await
            .expect("first save result should arrive")
            .expect("channel should not be closed");

        // Within the debounce window: should not dispatch another save.
        progress.save_debounced().unwrap();
        assert_matches!(
            result_rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty),
            "debounced call within the window must not dispatch a second save"
        );
    }

    /// Regression test for critique finding B1: several `save_immediate`
    /// calls made back-to-back (not gated by the debounce check
    /// `save_debounced` uses) must be applied by the serialized save writer
    /// strictly in the order they were requested, so the *last* call's data
    /// always wins on disk. Before this fix, each call spawned its own
    /// independent write task with no ordering guarantee between them, so
    /// an earlier call could race ahead of (and overwrite) a later one.
    #[tokio::test]
    async fn test_save_immediate_calls_are_applied_in_request_order() {
        const REQUESTED_XP_VALUES: [u64; 5] = [100, 150, 200, 250, 300];
        use crate::async_state::DataLoadMessage;
        use crate::data_loader::spawn_save_writer;
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();
        let (result_tx, mut result_rx) = mpsc::channel(8);
        let (save_tx, writer_handle) = spawn_save_writer(result_tx);
        progress.set_save_channel(save_tx.clone());

        for &xp in &REQUESTED_XP_VALUES {
            progress.profile.set_total_xp(xp);
            progress.save_immediate().unwrap();
        }

        for _ in REQUESTED_XP_VALUES {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(5), result_rx.recv())
                .await
                .expect("save result should arrive")
                .expect("channel should not be closed");
            assert_matches!(
                msg,
                DataLoadMessage::ProfileSaved,
                "every save must succeed, got {msg:?}"
            );
        }

        // Drop every sender clone — including the one `progress` holds
        // internally — so the writer's receive loop sees the channel close
        // and its `JoinHandle` actually resolves.
        drop(save_tx);
        drop(progress);
        writer_handle.await.unwrap();

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert_eq!(
            persisted.total_xp(),
            *REQUESTED_XP_VALUES.last().unwrap(),
            "the last-requested save must be the one left on disk"
        );
    }

    /// Regression test for critique finding B1's exit-ordering scenario:
    /// [`ProgressState::prepare_final_save_request`] must produce a
    /// snapshot that, once enqueued after an already-dispatched mid-session
    /// save and drained, is the one left on disk — mirroring exactly what
    /// `main` does around process exit.
    #[tokio::test]
    async fn test_final_save_request_wins_over_earlier_in_flight_save() {
        use crate::async_state::DataLoadMessage;
        use crate::data_loader::spawn_save_writer;
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");

        let mut progress = ProgressState::new(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::with_path(&profile_path),
        )
        .loaded();
        let (result_tx, mut result_rx) = mpsc::channel(8);
        let (save_tx, writer_handle) = spawn_save_writer(result_tx);
        progress.set_save_channel(save_tx.clone());

        // A mid-session save dispatched moments before exit (e.g. a
        // review-session completion), still possibly in flight.
        progress.profile.set_total_xp(100);
        progress.save_immediate().unwrap();

        // More happens before the user quits.
        progress.profile.set_total_xp(999);

        // The exit path: build the final snapshot, enqueue it last, drop
        // every sender clone, and drain.
        let final_request = progress
            .prepare_final_save_request()
            .expect("a writable profile yields an exit-time save");
        save_tx.send(final_request).await.unwrap();
        drop(save_tx);
        drop(progress);
        writer_handle.await.unwrap();

        for _ in 0..2 {
            let msg = tokio::time::timeout(std::time::Duration::from_secs(5), result_rx.recv())
                .await
                .expect("save result should arrive")
                .expect("channel should not be closed");
            assert_matches!(msg, DataLoadMessage::ProfileSaved);
        }

        let persisted = ProfileStorage::with_path(&profile_path).load().unwrap();
        assert_eq!(
            persisted.total_xp(),
            999,
            "the exit-time snapshot must win over the earlier in-flight save"
        );
    }
}
