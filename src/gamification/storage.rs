//! Profile storage and persistence

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::security::{limits::MAX_PROFILE_FILE_SIZE, path_validator};

use super::{GamificationError, Result, UserProfile};

/// Per-process counter used to make temporary save files unique.
///
/// Combined with the process id, this guarantees two concurrent saves (e.g.
/// two `ProfileStorage` instances pointed at the same path, as happens in
/// tests) never share a temp file name and race each other's rename.
static TMP_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Fsync a file's parent directory so a preceding `fs::rename` into it is
/// durable across a power loss, not just visible after a normal process
/// restart. POSIX does not guarantee a directory-entry update is on disk
/// until the directory itself is synced; Windows/NTFS has no equivalent
/// "open a directory as a file" operation, so this is Unix-only.
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) {
    // `Path::parent()` returns `Some("")` (not `None`) for a bare relative
    // filename with no directory component; treat that the same as "no
    // parent given" and fsync the current directory instead of no-op'ing.
    let parent = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

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
    file_path: PathBuf,
}

impl ProfileStorage {
    /// Create a new storage handler
    ///
    /// Default path: `~/.config/helix-trainer/profile.json`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::gamification::ProfileStorage;
    ///
    /// let storage = ProfileStorage::new();
    /// ```
    pub fn new() -> Self {
        let default_path = Self::default_path();
        Self {
            file_path: default_path,
        }
    }

    /// Create storage with custom path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            file_path: path.as_ref().to_path_buf(),
        }
    }

    /// Create storage backed by a unique file under a process-lifetime temp directory.
    ///
    /// This is the single safe default for test fixtures: every call returns a
    /// distinct path, so it can never collide with the real
    /// `~/.config/helix-trainer/profile.json` or with another test's storage, even
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

        static TEST_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();
        static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

        let dir = TEST_DIR.get_or_init(|| tempfile::TempDir::new().expect("create test temp dir"));
        let id = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self::with_path(dir.path().join(format!("profile-{id}.json")))
    }

    /// Get default storage path
    ///
    /// Returns `~/.config/helix-trainer/profile.json`
    fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("helix-trainer");
        path.push("profile.json");
        path
    }

    /// Load profile from disk
    ///
    /// Returns new profile if file doesn't exist
    ///
    /// # Errors
    ///
    /// Returns error if file exists but cannot be read or parsed
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
        if !self.file_path.exists() {
            return Ok(UserProfile::new());
        }

        path_validator::validate_file_size(&self.file_path, MAX_PROFILE_FILE_SIZE).map_err(
            |e| GamificationError::StorageError(format!("Profile file too large: {}", e)),
        )?;

        let contents = fs::read_to_string(&self.file_path).map_err(|e| {
            GamificationError::StorageError(format!("Failed to read profile: {}", e))
        })?;

        let data: ProfileData = serde_json::from_str(&contents).map_err(|e| {
            GamificationError::StorageError(format!("Failed to parse profile: {}", e))
        })?;

        Ok(data.profile)
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
        // Create parent directory if needed
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                GamificationError::StorageError(format!("Failed to create directory: {}", e))
            })?;
        }

        let data = ProfileData {
            profile: profile.clone(),
        };

        let json = serde_json::to_string_pretty(&data).map_err(|e| {
            GamificationError::StorageError(format!("Failed to serialize profile: {}", e))
        })?;

        let suffix = TMP_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut tmp_file_name = self
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_os_string();
        tmp_file_name.push(format!(".{}.{}.tmp", std::process::id(), suffix));
        let tmp_path = self.file_path.with_file_name(tmp_file_name);

        let write_result = (|| {
            let mut tmp_file = fs::File::create(&tmp_path)?;
            tmp_file.write_all(json.as_bytes())?;
            tmp_file.sync_all()
        })();
        if let Err(e) = write_result {
            let _ = fs::remove_file(&tmp_path);
            return Err(GamificationError::StorageError(format!(
                "Failed to write profile: {}",
                e
            )));
        }

        // Best-effort: carry the existing file's permissions onto the temp file so a
        // save doesn't silently reset them to the umask default via the rename below.
        // Skipped (not an error) if the target doesn't exist yet or metadata can't be read.
        if let Ok(metadata) = fs::metadata(&self.file_path) {
            let _ = fs::set_permissions(&tmp_path, metadata.permissions());
        }

        fs::rename(&tmp_path, &self.file_path).map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            GamificationError::StorageError(format!("Failed to persist profile: {}", e))
        })?;

        #[cfg(unix)]
        fsync_parent_dir(&self.file_path);

        // Best-effort: keep the PID lock pointed at this process on every
        // save, not just at startup, so a second instance launched mid-way
        // through a long session still finds an accurate, live lock rather
        // than one written only once minutes or hours earlier.
        self.refresh_lock();

        Ok(())
    }

    /// Path of the PID lock file that sits alongside the profile file.
    fn lock_path(&self) -> PathBuf {
        let mut lock_file_name = self
            .file_path
            .file_name()
            .unwrap_or_default()
            .to_os_string();
        lock_file_name.push(".lock");
        self.file_path.with_file_name(lock_file_name)
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
        // Same `pid` + `TMP_SUFFIX_COUNTER` scheme as `save`'s temp file
        // (see its comment): guarantees this doesn't collide with another
        // `refresh_lock`/`save` call racing on the same process, e.g. the
        // save writer refreshing the lock while a concurrent
        // `check_and_refresh_lock` call is also mid-refresh.
        let suffix = TMP_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut tmp_file_name = lock_path.file_name().unwrap_or_default().to_os_string();
        tmp_file_name.push(format!(".{}.{}.tmp", std::process::id(), suffix));
        let tmp_path = lock_path.with_file_name(tmp_file_name);

        if fs::write(&tmp_path, std::process::id().to_string()).is_ok() {
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
        self.file_path.exists()
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
            fs::remove_file(&self.file_path).map_err(|e| {
                GamificationError::StorageError(format!("Failed to delete profile: {}", e))
            })?;
        }
        let _ = fs::remove_file(self.lock_path());
        Ok(())
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        &self.file_path
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

    /// Regression test for S3: a save that fails partway (e.g. the temp-file write
    /// errors before the atomic rename) must leave the existing `profile.json`
    /// byte-for-byte untouched, not truncated or partially overwritten.
    ///
    /// Failure is injected by pre-occupying the exact temp-file path the next
    /// `save()` call will use (predictable from `save`'s pid + monotonic-counter
    /// naming scheme, see [`TMP_SUFFIX_COUNTER`]) with a directory, so
    /// `fs::File::create` deterministically fails with "Is a directory". Unlike
    /// a directory-permission approach (e.g. `chmod 0555`), this is a hard
    /// type-level OS constraint that root cannot bypass, so the test is
    /// reliable regardless of the privilege level it runs under.
    #[test]
    fn test_failed_save_does_not_corrupt_existing_profile() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        let mut profile = UserProfile::new();
        profile.add_xp(100);
        storage.save(&profile).unwrap();
        let bytes_before = fs::read_to_string(&file_path).unwrap();

        let next_suffix = TMP_SUFFIX_COUNTER.load(Ordering::Relaxed);
        let mut tmp_file_name = file_path.file_name().unwrap().to_os_string();
        tmp_file_name.push(format!(".{}.{}.tmp", std::process::id(), next_suffix));
        let predicted_tmp_path = file_path.with_file_name(tmp_file_name);
        fs::create_dir(&predicted_tmp_path).unwrap();

        let mut profile2 = profile.clone();
        profile2.add_xp(500);
        let result = storage.save(&profile2);

        assert!(
            result.is_err(),
            "save should fail when its temp-file target is occupied by a directory"
        );

        let bytes_after = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "existing profile.json must be untouched by a failed save"
        );

        let reloaded = storage.load().unwrap();
        assert_eq!(reloaded.total_xp, profile.total_xp);
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
}
