//! Profile storage and persistence

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::json_store::{JsonStore, unique_tmp_path};
use crate::security::{limits::MAX_PROFILE_FILE_SIZE, private_fs};

use super::{GamificationError, Result, UserProfile};

/// Check whether a process with the given pid is currently alive.
///
/// Shells out to a platform utility (`kill -0` on Unix, `tasklist` on
/// Windows) rather than a raw process API: the standard library has no
/// portable "is this pid alive" check, and the usual FFI-based approaches
/// require `unsafe`, which this crate forbids. Uses the absolute path
/// `/bin/kill` rather than a bare `kill` so this can't be fooled by an
/// unrelated `kill` earlier on `$PATH`.
#[cfg(unix)]
fn is_process_alive(pid: u32) -> bool {
    match std::process::Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .output()
    {
        Ok(output) => output.status.success(),
        Err(e) => {
            // Could not even run `/bin/kill` — treat as "not alive" (the
            // safe direction: reclaiming a lock is silently recoverable,
            // a missed warning is not), but this is a distinct failure
            // mode from an actually-dead pid and worth surfacing.
            tracing::debug!("Failed to run /bin/kill to check pid {pid}: {e}");
            false
        }
    }
}

#[cfg(windows)]
fn is_process_alive(pid: u32) -> bool {
    match std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()),
        Err(e) => {
            tracing::debug!("Failed to run tasklist to check pid {pid}: {e}");
            false
        }
    }
}

/// Wrapper for profile serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileData {
    profile: UserProfile,
}

/// Result of preserving an unreadable `profile.json` via [`ProfileStorage::quarantine`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuarantineOutcome {
    /// The original file was copied to this path; the original is untouched.
    BackedUp(PathBuf),

    /// The backup could not be made; the original is untouched.
    Failed(String),
}

/// Outcome of [`ProfileStorage::check_and_refresh_lock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStatus {
    /// No other live instance was using this profile; the lock file now
    /// records the current process's pid.
    Acquired,

    /// Another live process appears to already be using this profile file.
    OtherInstanceRunning(u32),
}

/// Handles profile persistence to disk
#[derive(Debug, Clone)]
pub struct ProfileStorage {
    store: JsonStore<ProfileData>,
}

impl ProfileStorage {
    /// Create a new storage handler
    ///
    /// Default path: `helix-trainer/profile.json` in the OS config directory
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::gamification::ProfileStorage;
    ///
    /// let storage = ProfileStorage::new();
    /// ```
    pub fn new() -> Self {
        Self::with_path(Self::default_path())
    }

    /// Create storage with custom path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            store: JsonStore::new(path.as_ref(), "profile", MAX_PROFILE_FILE_SIZE),
        }
    }

    /// Create storage backed by a unique file under a process-lifetime temp directory.
    ///
    /// This is the single safe default for test fixtures: every call returns a
    /// distinct path, so it can never collide with the real
    /// `helix-trainer/profile.json` or with another test's storage, even
    /// under parallel `cargo nextest` execution. All test fixture builders (
    /// `src/testing/app_state.rs` and the per-file `create_test_state()` helpers)
    /// must construct `ProfileStorage` through this function, not `new()` — `new()`
    /// resolves to the real user profile path and a test that reaches a save call
    /// with it will silently overwrite the developer's real progress data.
    ///
    /// The backing temp directory is intentionally never cleaned up mid-process
    /// (it outlives any single test), relying on the OS to reclaim it; this mirrors
    /// the tradeoff every other `tempfile::TempDir`-based test already makes, just
    /// centralized instead of per-call-site.
    #[cfg(test)]
    pub fn for_test() -> Self {
        use std::sync::OnceLock;
        use std::sync::atomic::{AtomicU64, Ordering};

        static TEST_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
        static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

        let dir = TEST_DIR.get_or_init(|| tempfile::TempDir::new().expect("create test temp dir"));
        let id = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self::with_path(dir.path().join(format!("profile-{id}.json")))
    }

    /// Get default storage path
    ///
    /// Returns `helix-trainer/profile.json` in the OS config directory
    fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("helix-trainer");
        path.push("profile.json");
        path
    }

    /// Load profile from disk
    ///
    /// Returns a new profile only if the file does not exist. The stored
    /// `level` is recomputed from `total_xp` and unknown achievement ids are
    /// dropped.
    ///
    /// # Errors
    ///
    /// Returns error if the path exists but cannot be read (not a regular
    /// file, over [`MAX_PROFILE_FILE_SIZE`], I/O failure) or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::gamification::ProfileStorage;
    ///
    /// let storage = ProfileStorage::new();
    /// let profile = storage.load().unwrap();
    /// println!("Level: {}", profile.level);
    /// ```
    pub fn load(&self) -> Result<UserProfile> {
        match self.store.load()? {
            Some(mut data) => {
                data.profile.recompute_level();
                Ok(data.profile)
            }
            None => Ok(UserProfile::new()),
        }
    }

    /// Copy an unreadable profile file to `profile.json.corrupt-<timestamp>`
    /// so its contents survive whatever the application does next.
    ///
    /// Never modifies or deletes the original. A name collision (two
    /// quarantines in the same second) gets a numeric suffix. Anything that
    /// is not a regular file is not copied (reading it could block or never
    /// end).
    pub fn quarantine(&self) -> QuarantineOutcome {
        match self.try_quarantine() {
            Ok(path) => QuarantineOutcome::BackedUp(path),
            Err(e) => QuarantineOutcome::Failed(e.to_string()),
        }
    }

    fn try_quarantine(&self) -> std::io::Result<PathBuf> {
        let not_regular = || std::io::Error::other("not a regular file");
        if !fs::metadata(self.path())?.is_file() {
            return Err(not_regular());
        }
        let mut source = fs::File::open(self.path())?;
        if !source.metadata()?.is_file() {
            return Err(not_regular());
        }

        let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
        let mut attempt = 0u32;
        let (backup_path, mut backup) = loop {
            let extension = if attempt == 0 {
                format!("corrupt-{stamp}")
            } else {
                format!("corrupt-{stamp}-{attempt}")
            };
            let candidate = self.path().with_added_extension(extension);
            match private_fs::create_new_file(&candidate) {
                Ok(file) => break (candidate, file),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => attempt += 1,
                Err(e) => return Err(e),
            }
        };

        let copied = std::io::copy(&mut source, &mut backup).and_then(|_| backup.sync_all());
        if let Err(e) = copied {
            let _ = fs::remove_file(&backup_path);
            return Err(e);
        }
        Ok(backup_path)
    }

    /// Save profile to disk
    ///
    /// Creates parent directory if needed. Writes to a temporary file in the
    /// same directory, `fsync`s it, and renames it into place: `fs::rename`
    /// is atomic on the same filesystem on both POSIX and Windows, so a
    /// crash or kill mid-write can never leave a truncated or
    /// partially-written `profile.json` behind — a reader always sees
    /// either the old file or the fully-written new one, and this guarantee
    /// holds regardless of power loss too. On Unix, the parent directory is
    /// additionally `fsync`'d after the rename, so the directory-entry
    /// update itself (not just the file's contents) is durable across a
    /// power loss, not only a process crash or kill.
    ///
    /// # Errors
    ///
    /// Returns error if the file cannot be written, synced, or renamed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::gamification::{ProfileStorage, UserProfile};
    ///
    /// let storage = ProfileStorage::new();
    /// let mut profile = UserProfile::new();
    /// profile.add_xp(100);
    /// storage.save(&profile).unwrap();
    /// ```
    pub fn save(&self, profile: &UserProfile) -> Result<()> {
        self.store.save(&ProfileData {
            profile: profile.clone(),
        })?;

        // Best-effort: keep the PID lock pointed at this process on every
        // save, not just at startup, so a second instance launched mid-way
        // through a long session still finds an accurate, live lock rather
        // than one written only once minutes or hours earlier.
        self.refresh_lock();

        Ok(())
    }

    /// Path of the PID lock file that sits alongside the profile file.
    fn lock_path(&self) -> PathBuf {
        self.path().with_added_extension("lock")
    }

    /// Best-effort: write this process's pid into the lock file, claiming or
    /// refreshing ownership without checking who (if anyone) held it before.
    ///
    /// Writes via a pid-suffixed temp file plus rename, same shape as
    /// [`ProfileStorage::save`], rather than a direct truncating
    /// `fs::write`: the lock file's own path is shared by every instance
    /// pointed at the same profile, so a plain write racing against a
    /// concurrent reader (another instance's
    /// [`ProfileStorage::check_and_refresh_lock`]) could hand back a
    /// truncated or partial read. No `fsync` here, unlike `save` — the
    /// lock is advisory only, so atomicity against partial reads matters,
    /// durability across a power loss does not.
    fn refresh_lock(&self) {
        let lock_path = self.lock_path();
        // Unique temp name: this must not collide with another
        // `refresh_lock`/`save` call racing in the same process, e.g. the
        // save writer refreshing the lock while a concurrent
        // `check_and_refresh_lock` call is also mid-refresh.
        let tmp_path = unique_tmp_path(&lock_path);

        let written = private_fs::create_file(&tmp_path)
            .and_then(|mut file| file.write_all(std::process::id().to_string().as_bytes()));
        if written.is_ok() {
            let _ = fs::rename(&tmp_path, &lock_path);
        } else {
            let _ = fs::remove_file(&tmp_path);
        }
    }

    /// Check whether another live process already has this profile open,
    /// then claim (or refresh) the lock file for the current process.
    ///
    /// The lock is advisory only — a small text file containing the owning
    /// process's pid, next to `profile.json`. It does not prevent concurrent
    /// writes; [`ProfileStorage::save`] already guarantees a save can never
    /// corrupt the file regardless of how many processes write to it. This
    /// exists purely to warn the user when two instances point at the same
    /// profile, since without it the "last save wins" behavior would
    /// silently discard one instance's progress with no indication why.
    ///
    /// A lock file left behind by a crashed process (a pid that is no
    /// longer running) is treated as stale and silently reclaimed — startup
    /// is never blocked by a leftover lock.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::gamification::{LockStatus, ProfileStorage};
    ///
    /// let storage = ProfileStorage::new();
    /// match storage.check_and_refresh_lock() {
    ///     LockStatus::Acquired => {}
    ///     LockStatus::OtherInstanceRunning(pid) => {
    ///         eprintln!("Another instance (pid {pid}) appears to be running");
    ///     }
    /// }
    /// ```
    pub fn check_and_refresh_lock(&self) -> LockStatus {
        let current_pid = std::process::id();

        let other_live_pid = fs::read_to_string(self.lock_path())
            .ok()
            .and_then(|contents| contents.trim().parse::<u32>().ok())
            .filter(|&pid| pid != current_pid && is_process_alive(pid));

        self.refresh_lock();

        match other_live_pid {
            Some(pid) => LockStatus::OtherInstanceRunning(pid),
            None => LockStatus::Acquired,
        }
    }

    /// Check if profile file exists
    pub fn exists(&self) -> bool {
        self.store.exists()
    }

    /// Delete profile file
    ///
    /// Also best-effort removes the PID lock file alongside it (see
    /// [`ProfileStorage::check_and_refresh_lock`]) so a deleted profile
    /// doesn't leave a stale lock behind; this is never an error condition
    /// since the lock file is advisory and may legitimately not exist.
    ///
    /// # Errors
    ///
    /// Returns error if file exists but cannot be deleted
    pub fn delete(&self) -> Result<()> {
        if self.exists() {
            fs::remove_file(self.path()).map_err(|e| {
                GamificationError::StorageError(format!("Failed to delete profile: {}", e))
            })?;
        }
        let _ = fs::remove_file(self.lock_path());
        Ok(())
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        self.store.path()
    }
}

impl Default for ProfileStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        // Create profile
        let mut profile = UserProfile::new();
        profile.add_xp(100);
        profile.current_streak = 5;

        // Save
        storage.save(&profile).unwrap();
        assert!(storage.exists());

        // Load
        let loaded = storage.load().unwrap();
        assert_eq!(loaded.level, profile.level);
        assert_eq!(loaded.total_xp, profile.total_xp);
        assert_eq!(loaded.current_streak, profile.current_streak);
    }

    #[test]
    fn test_load_malformed_last_challenge_date_keeps_rest_of_profile() {
        for malformed in [
            serde_json::json!("not-a-date"),
            serde_json::json!(12345),
            serde_json::json!([1, 2]),
            serde_json::json!({"a": 1}),
        ] {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("profile.json");
            let storage = ProfileStorage::with_path(&file_path);

            let mut profile = UserProfile::new();
            profile.add_xp(100);
            profile.challenge_progress.all_time_best_score = 42;
            storage.save(&profile).unwrap();

            let mut json: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&file_path).unwrap()).unwrap();
            json["profile"]["challenge_progress"]["last_challenge_date"] = malformed.clone();
            fs::write(&file_path, json.to_string()).unwrap();

            let loaded = storage
                .load()
                .unwrap_or_else(|e| panic!("{malformed}: {e}"));
            assert!(loaded.challenge_progress.last_challenge_date.is_none());
            assert_eq!(loaded.challenge_progress.all_time_best_score, 42);
            assert_eq!(loaded.total_xp, profile.total_xp);
        }
    }

    #[test]
    fn test_load_oversized_file_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        // Otherwise-valid JSON padded with an oversized unknown field: since
        // `ProfileData` doesn't `deny_unknown_fields`, `serde_json` would
        // happily parse this and ignore "padding" if the size guard weren't
        // there first. This proves the rejection below comes from the size
        // cap, not from a JSON parse failure that a smaller-but-still-broken
        // payload would trigger regardless of the cap.
        let mut value = serde_json::to_value(&ProfileData {
            profile: UserProfile::new(),
        })
        .unwrap();
        value["padding"] =
            serde_json::Value::String("a".repeat(MAX_PROFILE_FILE_SIZE as usize + 1));
        let oversized = serde_json::to_string(&value).unwrap();
        assert!(oversized.len() as u64 > MAX_PROFILE_FILE_SIZE);
        fs::write(&file_path, &oversized).unwrap();

        let err = storage
            .load()
            .expect_err("an oversized profile file must be rejected");
        assert!(
            err.to_string().contains("too large"),
            "expected a 'too large' error, got: {err}"
        );
    }

    #[test]
    fn test_load_nonexistent_returns_new_profile() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");
        let storage = ProfileStorage::with_path(&file_path);

        let profile = storage.load().unwrap();
        assert_eq!(profile.level, 1);
        assert_eq!(profile.total_xp, 0);
    }

    #[test]
    fn test_delete() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        // Save
        let profile = UserProfile::new();
        storage.save(&profile).unwrap();
        assert!(storage.exists());

        // Delete
        storage.delete().unwrap();
        assert!(!storage.exists());
    }

    #[test]
    fn test_delete_nonexistent_ok() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");
        let storage = ProfileStorage::with_path(&file_path);

        // Should not error
        storage.delete().unwrap();
    }

    #[test]
    fn test_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        let profile = UserProfile::new();
        storage.save(&profile).unwrap();

        assert!(storage.exists());
    }

    #[test]
    fn test_serialization_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        let mut profile = UserProfile::new();
        profile.add_xp(100);

        storage.save(&profile).unwrap();

        // Read raw JSON
        let json = fs::read_to_string(&file_path).unwrap();
        assert!(json.contains("\"profile\""));
        assert!(json.contains("\"level\""));
        assert!(json.contains("\"total_xp\""));
    }

    #[test]
    fn test_check_and_refresh_lock_acquires_when_no_lock_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

        let status = storage.check_and_refresh_lock();

        assert_eq!(status, LockStatus::Acquired);
        let lock_contents = fs::read_to_string(temp_dir.path().join("profile.json.lock")).unwrap();
        assert_eq!(lock_contents.trim(), std::process::id().to_string());
    }

    #[test]
    fn test_check_and_refresh_lock_own_pid_is_not_other_instance() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        fs::write(
            temp_dir.path().join("profile.json.lock"),
            std::process::id().to_string(),
        )
        .unwrap();

        assert_eq!(storage.check_and_refresh_lock(), LockStatus::Acquired);
    }

    /// Regression test for #298: a lock file left behind by a process that is
    /// no longer running must never block startup or be reported as another
    /// live instance.
    #[test]
    fn test_check_and_refresh_lock_reclaims_stale_pid() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

        // A pid guaranteed to no longer be running: spawn a trivial child
        // process and wait for it to exit so the pid is reclaimed by the OS.
        #[cfg(unix)]
        let mut dead_pid_child = std::process::Command::new("true").spawn().unwrap();
        #[cfg(windows)]
        let mut dead_pid_child = std::process::Command::new("cmd.exe")
            .args(["/C", "exit"])
            .spawn()
            .unwrap();
        let dead_pid = dead_pid_child.id();
        dead_pid_child.wait().unwrap();

        fs::write(
            temp_dir.path().join("profile.json.lock"),
            dead_pid.to_string(),
        )
        .unwrap();

        assert_eq!(
            storage.check_and_refresh_lock(),
            LockStatus::Acquired,
            "a stale pid must be treated as no other instance running"
        );
    }

    /// Regression test for #298: a lock file pointing at a genuinely running
    /// process must be reported as `OtherInstanceRunning`, not silently
    /// reclaimed.
    #[cfg(unix)]
    #[test]
    fn test_check_and_refresh_lock_detects_other_live_instance() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));

        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let other_pid = child.id();
        fs::write(
            temp_dir.path().join("profile.json.lock"),
            other_pid.to_string(),
        )
        .unwrap();

        let status = storage.check_and_refresh_lock();

        let _ = child.kill();
        let _ = child.wait();

        assert_eq!(status, LockStatus::OtherInstanceRunning(other_pid));
    }

    #[test]
    fn test_save_refreshes_lock_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        storage.save(&UserProfile::new()).unwrap();

        let lock_contents = fs::read_to_string(temp_dir.path().join("profile.json.lock")).unwrap();
        assert_eq!(lock_contents.trim(), std::process::id().to_string());
    }

    /// Non-blocking finding from the #298 critique: a deleted profile must
    /// not leave a stale lock file behind that could later misreport a
    /// long-dead (or PID-reused) process as still running.
    #[test]
    fn test_delete_removes_lock_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);
        let lock_path = temp_dir.path().join("profile.json.lock");

        storage.save(&UserProfile::new()).unwrap();
        assert!(lock_path.exists());

        storage.delete().unwrap();

        assert!(!lock_path.exists());
    }

    fn saved_profile_json(storage: &ProfileStorage, profile: &UserProfile) -> serde_json::Value {
        storage.save(profile).unwrap();
        serde_json::from_str(&fs::read_to_string(storage.path()).unwrap()).unwrap()
    }

    /// FR-004 (#440): an achievement id written by a newer version is
    /// ignored; the rest of the profile still loads.
    #[test]
    fn test_load_ignores_unknown_achievement_ids() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        let mut profile = UserProfile::new();
        profile.add_xp(300);
        let mut json = saved_profile_json(&storage, &profile);
        json["profile"]["achievements_unlocked"] =
            serde_json::json!(["Streak7Days", "FutureAchievement", 42]);
        fs::write(storage.path(), json.to_string()).unwrap();

        let loaded = storage.load().unwrap();

        assert_eq!(loaded.total_xp, 300);
        assert_eq!(loaded.achievements_unlocked.len(), 1);
        assert!(loaded.has_achievement(&crate::gamification::AchievementId::Streak7Days));
    }

    /// #450: a stored level inconsistent with `total_xp` is silently corrected.
    #[test]
    fn test_load_recomputes_level_from_total_xp() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("profile.json"));
        let mut json = saved_profile_json(&storage, &UserProfile::new());
        json["profile"]["level"] = serde_json::json!(50);
        fs::write(storage.path(), json.to_string()).unwrap();
        assert_eq!(storage.load().unwrap().level, 1);

        json["profile"]["level"] = serde_json::json!(1);
        json["profile"]["total_xp"] = serde_json::json!(1_000_000u64);
        fs::write(storage.path(), json.to_string()).unwrap();
        assert_eq!(
            storage.load().unwrap().level,
            crate::gamification::XPCalculator::level_from_xp(1_000_000)
        );
    }

    #[test]
    fn test_quarantine_copies_without_touching_original_and_names_are_unique() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let storage = ProfileStorage::with_path(&path);

        let first = storage.quarantine();
        let second = storage.quarantine();

        let (QuarantineOutcome::BackedUp(a), QuarantineOutcome::BackedUp(b)) = (first, second)
        else {
            panic!("both quarantines should succeed");
        };
        assert_ne!(a, b);
        for backup in [&a, &b] {
            assert!(
                backup
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("profile.json.corrupt-")
            );
            assert_eq!(fs::read_to_string(backup).unwrap(), "{broken");
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), "{broken");
    }

    #[test]
    fn test_quarantine_fails_without_a_regular_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path().join("missing.json"));
        assert!(matches!(storage.quarantine(), QuarantineOutcome::Failed(_)));

        let dir_storage = ProfileStorage::with_path(temp_dir.path());
        assert!(matches!(
            dir_storage.quarantine(),
            QuarantineOutcome::Failed(_)
        ));
    }

    #[test]
    fn test_load_unreadable_path_is_error_not_new_profile() {
        let temp_dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(temp_dir.path());
        assert!(storage.load().is_err());
    }

    /// #441: a symlink to a device must be rejected, not read without bound.
    #[cfg(unix)]
    #[test]
    fn test_load_symlink_to_dev_zero_is_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("profile.json");
        std::os::unix::fs::symlink("/dev/zero", &path).unwrap();

        let err = ProfileStorage::with_path(&path).load().unwrap_err();

        assert!(err.to_string().contains("Not a regular file"), "{err}");
    }

    /// #441: a FIFO must be rejected without blocking.
    #[cfg(unix)]
    #[test]
    fn test_load_fifo_is_rejected_without_blocking() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("profile.json");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );

        assert!(ProfileStorage::with_path(&path).load().is_err());
    }

    #[cfg(unix)]
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    /// #444: fresh data directory is 0700; profile and lock files are 0600.
    #[cfg(unix)]
    #[test]
    fn test_save_creates_private_directory_and_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path().join("data");
        let path = dir.join("profile.json");

        ProfileStorage::with_path(&path)
            .save(&UserProfile::new())
            .unwrap();

        assert_eq!(mode_of(&dir), 0o700);
        assert_eq!(mode_of(&path), 0o600);
        assert_eq!(mode_of(&dir.join("profile.json.lock")), 0o600);
    }

    /// #444 / FR-021: an existing world-readable profile is tightened on save.
    #[cfg(unix)]
    #[test]
    fn test_save_tightens_existing_broad_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("profile.json");
        fs::write(&path, "x").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        ProfileStorage::with_path(&path)
            .save(&UserProfile::new())
            .unwrap();

        assert_eq!(mode_of(&path), 0o600);
    }
}
