//! Profile storage and persistence

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{GamificationError, Result, UserProfile};

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
    /// Creates parent directory if needed
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be written
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

        fs::write(&self.file_path, json).map_err(|e| {
            GamificationError::StorageError(format!("Failed to write profile: {}", e))
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
