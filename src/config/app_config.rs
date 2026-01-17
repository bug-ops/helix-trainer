//! Application configuration storage and persistence
//!
//! Handles loading and saving application configuration to disk.
//! Configuration is stored in `~/.config/helix-trainer/config.json`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Enable arrow keys for movement in normal mode (for exotic keyboard layouts like bépo)
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
    pub fn load(&self) -> Result<AppConfig, String> {
        if !self.file_path.exists() {
            return Ok(AppConfig::default());
        }

        let contents = fs::read_to_string(&self.file_path).map_err(|e| {
            format!("Failed to read config: {}", e)
        })?;

        let data: ConfigData = serde_json::from_str(&contents).map_err(|e| {
            format!("Failed to parse config: {}", e)
        })?;

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
    pub fn save(&self, config: &AppConfig) -> Result<(), String> {
        // Create parent directory if needed
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!("Failed to create directory: {}", e)
            })?;
        }

        let data = ConfigData {
            config: config.clone(),
        };

        let json = serde_json::to_string_pretty(&data).map_err(|e| {
            format!("Failed to serialize config: {}", e)
        })?;

        fs::write(&self.file_path, json).map_err(|e| {
            format!("Failed to write config: {}", e)
        })?;

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
        let mut config = AppConfig::default();
        config.enable_arrow_keys_in_normal_mode = true;

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

        let mut config = AppConfig::default();
        config.enable_arrow_keys_in_normal_mode = true;

        storage.save(&config).unwrap();

        // Read raw JSON
        let json = fs::read_to_string(&file_path).unwrap();
        assert!(json.contains("\"config\""));
        assert!(json.contains("\"enable_arrow_keys_in_normal_mode\""));
    }
}

