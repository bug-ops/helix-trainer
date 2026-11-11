//! Profile storage and persistence

use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::data_dir::DataDir;
use crate::config::json_store::{Document, JsonStore, StorageError, create_unique_tmp};
use crate::security::limits::{MAX_LOCK_FILE_SIZE, MAX_PROFILE_FILE_SIZE};
use crate::security::{path_validator, private_fs};

use super::UserProfile;

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

const PROFILE_FILE_NAME: &str = "profile.json";

/// Whether two files of equal length hold the same bytes; `a` is rewound first.
fn same_content(a: &mut fs::File, b: &mut fs::File) -> std::io::Result<bool> {
    a.rewind()?;
    let mut buf_a = [0u8; 8192];
    let mut buf_b = [0u8; 8192];
    loop {
        let n = a.read(&mut buf_a)?;
        if n == 0 {
            return Ok(b.read(&mut buf_b)? == 0);
        }
        match b.read_exact(&mut buf_b[..n]) {
            Ok(()) if buf_a[..n] == buf_b[..n] => {}
            Ok(()) => return Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(false),
            Err(e) => return Err(e),
        }
    }
}

/// Whether `a` and `b` are the same file (same device and inode); always
/// `false` where the platform exposes no such identity.
#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino()
}

#[cfg(not(unix))]
fn same_file(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    false
}

/// Whether `candidate` is a regular file of `source_len` bytes that is not
/// `source` itself, judged from metadata only (symlinks, FIFOs and hard links
/// to the source never qualify as an independent copy).
fn is_independent_copy_shape(
    candidate: &fs::Metadata,
    source: &fs::Metadata,
    source_len: u64,
) -> bool {
    candidate.is_file() && candidate.len() == source_len && !same_file(candidate, source)
}

/// Wrapper for profile serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileData {
    profile: UserProfile,
}

/// Why an unreadable `profile.json` could not be backed up.
#[derive(Debug, Error)]
pub enum QuarantineError {
    /// There is no profile file (and no untransferred legacy file) to back up.
    #[error("there is no profile file to back up")]
    Missing,

    /// The path exists but is not a regular file, so it is not copied (reading it could block
    /// or never end).
    #[error("the profile path is not a regular file")]
    NotRegularFile,

    /// Inspecting, reading or copying the file failed.
    #[error("failed to copy the profile file")]
    Copy {
        /// The I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl From<std::io::Error> for QuarantineError {
    fn from(source: std::io::Error) -> Self {
        Self::Copy { source }
    }
}

/// Result of preserving an unreadable `profile.json` via [`ProfileStorage::quarantine`].
#[derive(Debug, Clone)]
pub enum QuarantineOutcome {
    /// The original file was copied to this path; the original is untouched.
    BackedUp(PathBuf),

    /// The backup could not be made; the original is untouched.
    Failed(Arc<QuarantineError>),
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
    /// Default path: `profile.json` in the data directory ([`DataDir::resolve`]):
    /// `dirs::config_dir()/helix-trainer`, or `~/.config/helix-trainer` on macOS
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::gamification::ProfileStorage;
    ///
    /// let storage = ProfileStorage::new();
    /// ```
    pub fn new() -> Self {
        Self::in_data_dir(&DataDir::resolve())
    }

    /// Create storage for `profile.json` in `data_dir`.
    ///
    /// When the data directory moved (macOS), [`ProfileStorage::load`] first
    /// copies the profile from the previous directory if the new one has
    /// none; a failed copy is a load failure, so the caller never replaces
    /// an untransferred profile with a default one.
    pub fn in_data_dir(data_dir: &DataDir) -> Self {
        Self {
            store: JsonStore::new(
                data_dir.file(PROFILE_FILE_NAME),
                Document::Profile,
                MAX_PROFILE_FILE_SIZE,
            )
            .migrating_from(data_dir.legacy_file(PROFILE_FILE_NAME)),
        }
    }

    /// Create storage with custom path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            store: JsonStore::new(path.as_ref(), Document::Profile, MAX_PROFILE_FILE_SIZE),
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
    pub fn load(&self) -> Result<UserProfile, StorageError> {
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
    /// When a byte-identical `profile.json.corrupt-*` already exists, no new
    /// copy is made and that backup's path is returned, so repeated launches
    /// with the same broken file keep a single backup.
    ///
    /// Never modifies or deletes the original. A name collision (two
    /// quarantines in the same second) gets a numeric suffix. Anything that
    /// is not a regular file is not copied (reading it could block or never
    /// end).
    pub fn quarantine(&self) -> QuarantineOutcome {
        match self.try_quarantine() {
            Ok(path) => QuarantineOutcome::BackedUp(path),
            Err(e) => QuarantineOutcome::Failed(Arc::new(e)),
        }
    }

    fn try_quarantine(&self) -> Result<PathBuf, QuarantineError> {
        let metadata = match fs::metadata(self.path()) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return self.untransferred_legacy().ok_or(QuarantineError::Missing);
            }
            Err(e) => return Err(e.into()),
        };
        if !metadata.is_file() {
            return Err(QuarantineError::NotRegularFile);
        }
        let mut source = fs::File::open(self.path())?;
        if !source.metadata()?.is_file() {
            return Err(QuarantineError::NotRegularFile);
        }
        if let Some(existing) = self.find_identical_backup(&mut source) {
            return Ok(existing);
        }
        source.rewind()?;

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
                Err(e) => return Err(e.into()),
            }
        };

        let copied = std::io::copy(&mut source, &mut backup).and_then(|_| backup.sync_all());
        if let Err(e) = copied {
            let _ = fs::remove_file(&backup_path);
            return Err(e.into());
        }
        Ok(backup_path)
    }

    /// The previous-location file when it exists but could not be copied into
    /// place: it is untouched, so it serves as the backup of the data that
    /// failed to migrate.
    fn untransferred_legacy(&self) -> Option<PathBuf> {
        self.store
            .legacy_path()
            .filter(|legacy| fs::symlink_metadata(legacy).is_ok())
            .map(Path::to_path_buf)
    }

    /// An existing `<file>.corrupt-*` sibling that is byte-identical to
    /// `source`, so repeated launches with the same unreadable file do not pile
    /// up copies. Unreadable candidates are skipped (the caller then makes a
    /// fresh copy, the safe direction). Only independent regular files count:
    /// a symlink or hard link to the source, or a FIFO, is never a backup, and
    /// candidates are inspected with `lstat` before being opened.
    fn find_identical_backup(&self, source: &mut fs::File) -> Option<PathBuf> {
        let prefix = format!("{}.corrupt-", self.path().file_name()?.to_str()?);
        let source_meta = source.metadata().ok()?;
        let source_len = source_meta.len();
        let mut candidates: Vec<PathBuf> = fs::read_dir(self.path().parent()?)
            .ok()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .map(|entry| entry.path())
            .collect();
        candidates.sort();
        candidates.into_iter().find(|candidate| {
            let is_candidate =
                |meta: &fs::Metadata| is_independent_copy_shape(meta, &source_meta, source_len);
            fs::symlink_metadata(candidate).is_ok_and(|meta| is_candidate(&meta))
                && fs::File::open(candidate)
                    .and_then(|mut file| {
                        if !is_candidate(&file.metadata()?) {
                            return Ok(false);
                        }
                        same_content(source, &mut file)
                    })
                    .unwrap_or(false)
        })
    }

    /// Replace an unreadable profile with a new default one, keeping its backup.
    ///
    /// The unreadable file is first quarantined (an identical existing backup
    /// is reused), and only when a backup exists on disk is `profile.json`
    /// overwritten with a fresh [`UserProfile`]; unreadable data is never
    /// replaced without a copy. The caller then adopts the returned profile
    /// and re-enables saving.
    ///
    /// # Errors
    ///
    /// Returns an error, leaving the file untouched, when no backup could be
    /// made (including when there is no profile file), or when the new
    /// profile cannot be written.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::ProfileStorage;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let path = dir.path().join("profile.json");
    /// std::fs::write(&path, "{broken").unwrap();
    ///
    /// let storage = ProfileStorage::with_path(&path);
    /// assert!(storage.load().is_err());
    /// let fresh = storage.start_fresh().unwrap();
    /// assert_eq!(fresh.level, 1);
    /// assert_eq!(storage.load().unwrap().level, 1);
    /// ```
    pub fn start_fresh(&self) -> Result<UserProfile, StorageError> {
        if let QuarantineOutcome::Failed(source) = self.quarantine() {
            return Err(StorageError::NoBackup {
                doc: Document::Profile,
                source,
            });
        }
        let profile = UserProfile::new();
        self.save(&profile)?;
        Ok(profile)
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
    pub fn save(&self, profile: &UserProfile) -> Result<(), StorageError> {
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
        // Unique, exclusively created temp name: it must not collide with
        // another `refresh_lock`/`save` call racing in the same process, nor
        // follow a file or symlink planted at the predicted name.
        let Ok((tmp_path, mut file)) = create_unique_tmp(&lock_path) else {
            return;
        };

        let written = file.write_all(std::process::id().to_string().as_bytes());
        drop(file);
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

        let other_live_pid = path_validator::read_capped(&self.lock_path(), MAX_LOCK_FILE_SIZE)
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
    pub fn delete(&self) -> Result<(), StorageError> {
        if self.exists() {
            fs::remove_file(self.path()).map_err(|source| StorageError::Delete {
                doc: Document::Profile,
                path: self.path().to_path_buf(),
                source,
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
        value["padding"] = serde_json::Value::String(
            "a".repeat(usize::try_from(MAX_PROFILE_FILE_SIZE).expect("fits in usize") + 1),
        );
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
        let storage = ProfileStorage::with_path(&path);
        fs::write(&path, "{broken-a").unwrap();
        let first = storage.quarantine();
        fs::write(&path, "{broken-b").unwrap();
        let second = storage.quarantine();

        let (QuarantineOutcome::BackedUp(a), QuarantineOutcome::BackedUp(b)) = (first, second)
        else {
            panic!("both quarantines should succeed");
        };
        assert_ne!(a, b);
        for (backup, content) in [(&a, "{broken-a"), (&b, "{broken-b")] {
            assert!(
                backup
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("profile.json.corrupt-")
            );
            assert_eq!(fs::read_to_string(backup).unwrap(), content);
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), "{broken-b");
    }

    fn backups_in(dir: &Path) -> Vec<PathBuf> {
        let mut paths: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.to_string_lossy().contains(".corrupt-"))
            .collect();
        paths.sort();
        paths
    }

    #[test]
    fn test_quarantine_skips_copy_when_identical_backup_exists() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let storage = ProfileStorage::with_path(&path);

        let first = storage.quarantine();
        let second = storage.quarantine();

        assert!(matches!(first, QuarantineOutcome::BackedUp(_)));
        std::assert_matches!((&first, &second), (QuarantineOutcome::BackedUp(a), QuarantineOutcome::BackedUp(b)) if a == b);
        assert_eq!(backups_in(dir.path()).len(), 1);
    }

    #[test]
    fn test_quarantine_copies_again_when_content_differs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&path);
        fs::write(&path, "{broken-a").unwrap();
        storage.quarantine();
        fs::write(&path, "{much-longer-broken").unwrap();

        let QuarantineOutcome::BackedUp(second) = storage.quarantine() else {
            panic!("expected a backup");
        };

        assert_eq!(backups_in(dir.path()).len(), 2);
        assert_eq!(fs::read_to_string(second).unwrap(), "{much-longer-broken");
    }

    #[test]
    fn test_quarantine_dedup_handles_files_larger_than_one_read_buffer() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&path);
        let mut body = "x".repeat(20_000);
        fs::write(&path, &body).unwrap();
        storage.quarantine();
        body.replace_range(19_999.., "y");
        fs::write(&path, &body).unwrap();

        let QuarantineOutcome::BackedUp(second) = storage.quarantine() else {
            panic!("expected a backup");
        };

        assert_eq!(backups_in(dir.path()).len(), 2);
        assert_eq!(fs::read_to_string(second).unwrap(), body);
    }

    #[test]
    fn test_start_fresh_keeps_backup_and_writes_new_profile() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let storage = ProfileStorage::with_path(&path);
        assert!(storage.load().is_err());
        storage.quarantine();

        let fresh = storage.start_fresh().unwrap();

        assert_eq!(fresh.level, 1);
        assert_eq!(storage.load().unwrap().total_xp, 0);
        let backups = backups_in(dir.path());
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read_to_string(&backups[0]).unwrap(), "{broken");
    }

    #[test]
    fn test_start_fresh_makes_a_backup_when_none_exists_yet() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();

        ProfileStorage::with_path(&path).start_fresh().unwrap();

        assert_eq!(backups_in(dir.path()).len(), 1);
    }

    #[test]
    fn test_start_fresh_refuses_without_a_profile_file_to_back_up() {
        let dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));

        assert!(storage.start_fresh().is_err());
        assert!(!storage.exists());
    }

    #[test]
    fn test_start_fresh_refuses_a_non_regular_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::create_dir(&path).unwrap();

        assert!(ProfileStorage::with_path(&path).start_fresh().is_err());
        assert!(path.is_dir());
    }

    fn backed_up_path(outcome: QuarantineOutcome) -> PathBuf {
        let QuarantineOutcome::BackedUp(path) = outcome else {
            panic!("expected a backup, got {outcome:?}");
        };
        path
    }

    #[cfg(unix)]
    #[test]
    fn test_quarantine_ignores_a_symlink_to_the_original_as_backup() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let link = dir.path().join("profile.json.corrupt-00000000T000000");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        let storage = ProfileStorage::with_path(&path);

        let backup = backed_up_path(storage.quarantine());

        assert_ne!(backup, link);
        assert!(!fs::symlink_metadata(&backup).unwrap().is_symlink());
        assert_eq!(fs::read_to_string(&backup).unwrap(), "{broken");
    }

    #[cfg(unix)]
    #[test]
    fn test_start_fresh_with_symlinked_backup_keeps_the_original_bytes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let link = dir.path().join("profile.json.corrupt-00000000T000000");
        std::os::unix::fs::symlink(&path, &link).unwrap();

        ProfileStorage::with_path(&path).start_fresh().unwrap();

        let independent: Vec<_> = backups_in(dir.path())
            .into_iter()
            .filter(|p| !fs::symlink_metadata(p).unwrap().is_symlink())
            .collect();
        assert_eq!(independent.len(), 1);
        assert_eq!(fs::read_to_string(&independent[0]).unwrap(), "{broken");
    }

    #[cfg(unix)]
    #[test]
    fn test_quarantine_ignores_a_hard_link_to_the_original_as_backup() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let link = dir.path().join("profile.json.corrupt-00000000T000000");
        fs::hard_link(&path, &link).unwrap();

        let backup = backed_up_path(ProfileStorage::with_path(&path).quarantine());

        assert_ne!(backup, link);
    }

    #[cfg(unix)]
    #[test]
    fn test_quarantine_skips_a_fifo_candidate_without_blocking() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        let fifo = dir.path().join("profile.json.corrupt-00000000T000000");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );

        let backup = backed_up_path(ProfileStorage::with_path(&path).quarantine());

        assert_ne!(backup, fifo);
        assert_eq!(fs::read_to_string(&backup).unwrap(), "{broken");
    }

    /// When the backup cannot be created the original must stay byte-identical.
    #[cfg(unix)]
    #[test]
    fn test_start_fresh_keeps_the_original_when_the_backup_fails() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::write(&path, "{broken").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).unwrap();
        let writable = fs::write(dir.path().join("probe"), "").is_ok();

        let result = ProfileStorage::with_path(&path).start_fresh();

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        if writable {
            return;
        }
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{broken");
        assert!(backups_in(dir.path()).is_empty());
    }

    #[test]
    fn test_lock_file_read_is_size_capped() {
        let dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));
        let huge = format!("{}{}", std::process::id() + 1, " ".repeat(4096));
        fs::write(storage.lock_path(), huge).unwrap();

        assert_eq!(storage.check_and_refresh_lock(), LockStatus::Acquired);
    }

    fn moved_data_dir(root: &Path) -> DataDir {
        DataDir::resolve_for(true, Some(root.join("xdg")), Some(root.join("os")))
    }

    fn legacy_profile_path(root: &Path) -> PathBuf {
        root.join("os").join("helix-trainer").join("profile.json")
    }

    #[test]
    fn test_in_data_dir_migrates_legacy_profile_and_keeps_the_source() {
        let root = TempDir::new().unwrap();
        let data_dir = moved_data_dir(root.path());
        let mut profile = UserProfile::new();
        profile.add_xp(250);
        ProfileStorage::with_path(legacy_profile_path(root.path()))
            .save(&profile)
            .unwrap();

        let storage = ProfileStorage::in_data_dir(&data_dir);
        let loaded = storage.load().unwrap();

        assert_eq!(loaded.total_xp, 250);
        assert_eq!(storage.path(), data_dir.file("profile.json"));
        assert!(storage.exists());
        assert!(legacy_profile_path(root.path()).exists());
    }

    #[test]
    fn test_in_data_dir_prefers_new_profile_over_legacy() {
        let root = TempDir::new().unwrap();
        let data_dir = moved_data_dir(root.path());
        let mut old = UserProfile::new();
        old.add_xp(100);
        ProfileStorage::with_path(legacy_profile_path(root.path()))
            .save(&old)
            .unwrap();
        let mut new = UserProfile::new();
        new.add_xp(900);
        let storage = ProfileStorage::in_data_dir(&data_dir);
        ProfileStorage::with_path(storage.path())
            .save(&new)
            .unwrap();

        assert_eq!(storage.load().unwrap().total_xp, 900);
    }

    #[test]
    fn test_in_data_dir_does_not_migrate_backups_or_lock() {
        let root = TempDir::new().unwrap();
        let data_dir = moved_data_dir(root.path());
        let legacy = ProfileStorage::with_path(legacy_profile_path(root.path()));
        legacy.save(&UserProfile::new()).unwrap();
        let legacy_dir = legacy.path().parent().unwrap().to_path_buf();
        fs::write(legacy_dir.join("profile.json.corrupt-20260101T000000"), "x").unwrap();

        let storage = ProfileStorage::in_data_dir(&data_dir);
        storage.load().unwrap();

        let mut names: Vec<_> = fs::read_dir(storage.path().parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["profile.json"]);
    }

    /// A failed copy must surface as a load failure and leave no destination,
    /// so the caller can never overwrite untransferred data with a default.
    #[test]
    fn test_in_data_dir_failed_migration_is_a_load_error_without_destination() {
        let root = TempDir::new().unwrap();
        let data_dir = moved_data_dir(root.path());
        fs::create_dir_all(legacy_profile_path(root.path())).unwrap();

        let storage = ProfileStorage::in_data_dir(&data_dir);

        assert!(storage.load().is_err());
        assert!(!storage.exists());
    }

    /// A legacy profile that cannot be copied is itself the untouched backup,
    /// so "Start fresh" stays reachable and never loses it.
    #[test]
    fn test_unmigratable_legacy_profile_is_the_backup_and_start_fresh_works() {
        let root = TempDir::new().unwrap();
        let data_dir = moved_data_dir(root.path());
        let legacy = legacy_profile_path(root.path());
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let oversized = usize::try_from(MAX_PROFILE_FILE_SIZE).unwrap() + 1;
        fs::write(&legacy, " ".repeat(oversized)).unwrap();
        let before = fs::read(&legacy).unwrap();
        let storage = ProfileStorage::in_data_dir(&data_dir);
        assert!(storage.load().is_err());

        assert_eq!(backed_up_path(storage.quarantine()), legacy);
        let fresh = storage.start_fresh().unwrap();

        assert_eq!(fresh.level, 1);
        assert_eq!(storage.load().unwrap().total_xp, 0);
        assert_eq!(fs::read(&legacy).unwrap(), before);
    }

    /// A broken but small legacy file is copied, then fails to parse from the
    /// new location, where the normal quarantine path applies.
    #[test]
    fn test_malformed_legacy_profile_fails_to_parse_and_can_start_fresh() {
        let root = TempDir::new().unwrap();
        let data_dir = moved_data_dir(root.path());
        let legacy = legacy_profile_path(root.path());
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, "{garbage").unwrap();
        let storage = ProfileStorage::in_data_dir(&data_dir);

        assert!(storage.load().is_err());
        assert_eq!(fs::read_to_string(storage.path()).unwrap(), "{garbage");
        let backup = backed_up_path(storage.quarantine());
        assert_eq!(fs::read_to_string(backup).unwrap(), "{garbage");

        storage.start_fresh().unwrap();
        assert_eq!(storage.load().unwrap().level, 1);
        assert_eq!(fs::read_to_string(&legacy).unwrap(), "{garbage");
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

        assert!(
            matches!(
                err,
                StorageError::Read {
                    source: crate::security::CappedReadError::NotRegularFile,
                    ..
                }
            ),
            "{err}"
        );
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

    #[test]
    fn start_fresh_without_a_profile_file_is_a_no_backup_error() {
        let dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));

        let err = storage.start_fresh().unwrap_err();

        assert!(matches!(err, StorageError::NoBackup { .. }), "{err}");
        let chain = crate::security::error_chain(&err);
        assert_eq!(chain.matches("no profile file").count(), 1, "{chain}");
        assert!(!storage.exists());
    }

    #[test]
    fn delete_failure_is_a_typed_delete_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("profile.json");
        fs::create_dir(&path).unwrap();

        let err = ProfileStorage::with_path(&path).delete().unwrap_err();

        assert!(matches!(err, StorageError::Delete { .. }), "{err}");
    }

    #[test]
    fn quarantine_failure_for_a_missing_file_is_typed() {
        let dir = TempDir::new().unwrap();
        let storage = ProfileStorage::with_path(dir.path().join("profile.json"));

        std::assert_matches!(
            storage.quarantine(),
            QuarantineOutcome::Failed(e) if matches!(*e, QuarantineError::Missing)
        );
    }
}
