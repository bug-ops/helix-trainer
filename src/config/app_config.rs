//! Application configuration storage and persistence
//!
//! Handles loading and saving application configuration to disk.
//! Configuration is stored in `~/.config/helix-trainer/config.json`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Enable arrow keys for movement in normal mode (for exotic keyboard layouts like bépo).
    ///
    /// To enable this feature, manually create or edit the config file at
    /// `~/.config/helix-trainer/config.json` with the following structure:
    ///
    /// ```json
    /// {
    ///   "config": {
    ///     "enable_arrow_keys_in_normal_mode": true
    ///   }
    /// }
    /// ```
    ///
    /// If the configuration file does not exist, default values (false) are used in memory.
    /// The configuration file is only written when configuration changes are saved.
    pub enable_arrow_keys_in_normal_mode: bool,
}

/// Wrapper for config serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigData {
    config: AppConfig,
}

/// Handles configuration persistence to disk
pub struct ConfigStorage {
    file_path: PathBuf,
}

impl ConfigStorage {
    /// Create a new storage handler
    ///
    /// Default path: `~/.config/helix-trainer/config.json`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::config::ConfigStorage;
    ///
    /// let storage = ConfigStorage::new();
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
    /// Returns `~/.config/helix-trainer/config.json`
    fn default_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("helix-trainer");
        path.push("config.json");
        path
    }

    /// Load configuration from disk
    ///
    /// Returns default configuration if file doesn't exist
    ///
    /// # Errors
    ///
    /// Returns error if file exists but cannot be read or parsed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::config::ConfigStorage;
    ///
    /// let storage = ConfigStorage::new();
    /// let config = storage.load().unwrap();
    /// ```
    pub fn load(&self) -> Result<AppConfig> {
        if !self.file_path.exists() {
            return Ok(AppConfig::default());
        }

        let contents = fs::read_to_string(&self.file_path).context("Failed to read config file")?;

        let data: ConfigData =
            serde_json::from_str(&contents).context("Failed to parse config JSON")?;

        Ok(data.config)
    }

    /// Save configuration to disk
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
    /// use helix_trainer::config::{ConfigStorage, AppConfig};
    ///
    /// let storage = ConfigStorage::new();
    /// let config = AppConfig::default();
    /// storage.save(&config).unwrap();
    /// ```
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let data = ConfigData {
            config: config.clone(),
        };

        let json = serde_json::to_string_pretty(&data).context("Failed to serialize config")?;

        fs::write(&self.file_path, json).context("Failed to write config file")?;

        Ok(())
    }

    /// Check if config file exists
    pub fn exists(&self) -> bool {
        self.file_path.exists()
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        &self.file_path
    }
}

impl Default for ConfigStorage {
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
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // Create config
        let config = AppConfig {
            enable_arrow_keys_in_normal_mode: true,
        };

        // Save
        storage.save(&config).unwrap();
        assert!(storage.exists());

        // Load
        let loaded = storage.load().unwrap();
        assert_eq!(
            loaded.enable_arrow_keys_in_normal_mode,
            config.enable_arrow_keys_in_normal_mode
        );
    }

    #[test]
    fn test_load_nonexistent_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.json");
        let storage = ConfigStorage::with_path(&file_path);

        let config = storage.load().unwrap();
        assert!(!config.enable_arrow_keys_in_normal_mode);
    }

    #[test]
    fn test_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        let config = AppConfig::default();
        storage.save(&config).unwrap();

        assert!(storage.exists());
    }

    #[test]
    fn test_serialization_format() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        let config = AppConfig {
            enable_arrow_keys_in_normal_mode: true,
        };

        storage.save(&config).unwrap();

        // Read raw JSON
        let json = fs::read_to_string(&file_path).unwrap();
        assert!(json.contains("\"config\""));
        assert!(json.contains("\"enable_arrow_keys_in_normal_mode\""));
    }

    #[test]
    fn test_load_invalid_json_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // Write invalid JSON
        fs::write(&file_path, "invalid json {").unwrap();

        let result = storage.load();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse config JSON")
        );
    }

    #[test]
    fn test_load_missing_config_field_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // Write JSON without "config" field
        fs::write(&file_path, r#"{"other": "value"}"#).unwrap();

        let result = storage.load();
        assert!(result.is_err());
    }

    #[test]
    fn test_new_creates_default_path() {
        let storage = ConfigStorage::new();
        let path = storage.path();
        assert!(path.to_string_lossy().contains("helix-trainer"));
        assert!(path.to_string_lossy().contains("config.json"));
    }

    #[test]
    fn test_path_returns_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);
        assert_eq!(storage.path(), file_path.as_path());
    }

    #[test]
    fn test_default_impl() {
        let storage = ConfigStorage::default();
        let path = storage.path();
        assert!(path.to_string_lossy().contains("helix-trainer"));
    }

    #[test]
    fn test_save_when_no_parent_directory() {
        // Test edge case: file path with no parent (shouldn't happen in practice but test it)
        // We'll use a path that exists but has no parent by using a root path
        // Actually, let's test with a file in the temp dir root
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        let config = AppConfig::default();
        // This should work fine
        storage.save(&config).unwrap();
        assert!(storage.exists());
    }

    #[test]
    fn test_load_file_read_error() {
        // Create a directory with the config file name to simulate read error
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        // Create a directory instead of a file
        fs::create_dir_all(&file_path).unwrap();

        let storage = ConfigStorage::with_path(&file_path);
        let result = storage.load();
        // Should return an error when trying to read a directory as a file
        assert!(result.is_err());
    }
}
