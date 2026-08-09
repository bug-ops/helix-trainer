//! Profile storage and persistence

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::{GamificationError, Result, UserProfile};

/// Per-process counter used to make temporary save files unique.
///
/// Combined with the process id, this guarantees two concurrent saves (e.g.
/// two `ProfileStorage` instances pointed at the same path, as happens in
/// tests) never share a temp file name and race each other's rename.
static TMP_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Wrapper for profile serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileData {
    profile: UserProfile,
}

/// Handles profile persistence to disk
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
    /// same directory, `fsync`s it, and renames it into place, so a crash,
    /// kill, or power loss mid-write cannot leave a truncated `profile.json`
    /// behind — `fs::rename` is atomic on the same filesystem on both POSIX
    /// and Windows, and the prior `sync_all` ensures the renamed data is
    /// actually durable rather than sitting in a write-back cache.
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

        Ok(())
    }

    /// Check if profile file exists
    pub fn exists(&self) -> bool {
        self.file_path.exists()
    }

    /// Delete profile file
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
    #[cfg(unix)]
    #[test]
    fn test_failed_save_does_not_corrupt_existing_profile() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("profile.json");
        let storage = ProfileStorage::with_path(&file_path);

        let mut profile = UserProfile::new();
        profile.add_xp(100);
        storage.save(&profile).unwrap();
        let bytes_before = fs::read_to_string(&file_path).unwrap();

        // Strip write permission from the directory so the next save's
        // temp-file write fails before it ever reaches `fs::rename`.
        let original_perms = fs::metadata(temp_dir.path()).unwrap().permissions();
        let mut readonly_perms = original_perms.clone();
        readonly_perms.set_mode(0o555);
        fs::set_permissions(temp_dir.path(), readonly_perms).unwrap();

        let mut profile2 = profile.clone();
        profile2.add_xp(500);
        let result = storage.save(&profile2);

        // Restore permissions before any assertion so TempDir can clean up
        // even if an assertion below panics.
        fs::set_permissions(temp_dir.path(), original_perms).unwrap();

        assert!(
            result.is_err(),
            "save should fail when the directory is read-only"
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
}
