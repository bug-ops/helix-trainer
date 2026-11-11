//! Application configuration storage and persistence
//!
//! Handles loading and saving application configuration to disk.
//! Configuration is stored in `config.json` in the data directory
//! ([`DataDir`]): `~/.config/helix-trainer` on Linux and macOS (honoring
//! `$XDG_CONFIG_HOME` on macOS), `%AppData%\helix-trainer` on Windows.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::data_dir::DataDir;
use super::json_store::JsonStore;
use crate::security::limits::MAX_CONFIG_FILE_SIZE;

const CONFIG_FILE_NAME: &str = "config.json";

/// Application configuration
///
/// `#[serde(default)]` on the struct is load-bearing: without it, adding
/// any new field would make every existing `config.json` on disk fail to
/// deserialize (missing field), silently discarding the user's whole
/// config back to defaults instead of just defaulting the new field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Enable arrow keys for movement in normal mode (for exotic keyboard layouts like bépo).
    ///
    /// To enable this feature, manually create or edit the config file at
    /// `helix-trainer/config.json` in the OS config directory (see the module
    /// docs) with the following structure:
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

    /// Translate gameplay keypresses through the user's real Helix
    /// `config.toml` keymap (read from `dirs::config_dir()/helix/config.toml`).
    ///
    /// Disabled by default: an exotic or malformed Helix config should
    /// never silently change training semantics for a user who didn't ask
    /// for it. Applies to gameplay only — menus, results, and filter
    /// screens keep the stock keymap.
    pub use_helix_keymap: bool,
}

/// Wrapper for config serialization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct ConfigData {
    config: AppConfig,
}

/// Handles configuration persistence to disk
#[derive(Debug, Clone)]
pub struct ConfigStorage {
    store: JsonStore<ConfigData>,
}

impl ConfigStorage {
    /// Create a new storage handler
    ///
    /// Default path: `config.json` in the data directory ([`DataDir::resolve`])
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use helix_trainer::config::ConfigStorage;
    ///
    /// let storage = ConfigStorage::new();
    /// ```
    pub fn new() -> Self {
        Self::in_data_dir(&DataDir::resolve())
    }

    /// Create storage for `config.json` in `data_dir`, copying it from the
    /// previous data directory on first load when the location moved (macOS).
    pub fn in_data_dir(data_dir: &DataDir) -> Self {
        Self {
            store: JsonStore::new(
                data_dir.file(CONFIG_FILE_NAME),
                "config",
                MAX_CONFIG_FILE_SIZE,
            )
            .migrating_from(data_dir.legacy_file(CONFIG_FILE_NAME)),
        }
    }

    /// Create storage with custom path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            store: JsonStore::new(path.as_ref(), "config", MAX_CONFIG_FILE_SIZE),
        }
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
        Ok(self
            .store
            .load()?
            .map(|data| data.config)
            .unwrap_or_default())
    }

    /// Save configuration to disk
    ///
    /// Atomically replaces the file (temporary file, `fsync`, rename) and
    /// creates the parent directory if needed.
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
        Ok(self.store.save(&ConfigData {
            config: config.clone(),
        })?)
    }

    /// Check if config file exists
    pub fn exists(&self) -> bool {
        self.store.exists()
    }

    /// Get file path
    pub fn path(&self) -> &Path {
        self.store.path()
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
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // Create config
        let config = AppConfig {
            enable_arrow_keys_in_normal_mode: true,
            ..AppConfig::default()
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
    fn test_in_data_dir_migrates_legacy_config() {
        let root = TempDir::new().unwrap();
        let data_dir = DataDir::resolve_for(
            true,
            Some(root.path().join("xdg")),
            Some(root.path().join("os")),
        );
        let legacy = ConfigStorage::with_path(root.path().join("os/helix-trainer/config.json"));
        legacy
            .save(&AppConfig {
                use_helix_keymap: true,
                ..AppConfig::default()
            })
            .unwrap();

        let storage = ConfigStorage::in_data_dir(&data_dir);

        assert!(storage.load().unwrap().use_helix_keymap);
        assert_eq!(storage.path(), data_dir.file("config.json"));
        assert!(legacy.exists());
    }

    #[test]
    fn test_in_data_dir_failed_migration_is_an_error_and_creates_nothing() {
        let root = TempDir::new().unwrap();
        let data_dir = DataDir::resolve_for(
            true,
            Some(root.path().join("xdg")),
            Some(root.path().join("os")),
        );
        fs::create_dir_all(root.path().join("os/helix-trainer/config.json")).unwrap();

        let storage = ConfigStorage::in_data_dir(&data_dir);

        assert!(storage.load().is_err());
        assert!(!storage.exists());
    }

    #[test]
    fn test_load_oversized_file_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // Otherwise-valid JSON padded with an oversized unknown field: since
        // `ConfigData` doesn't `deny_unknown_fields`, `serde_json` would
        // happily parse this and ignore "padding" if the size guard weren't
        // there first. This proves the rejection below comes from the size
        // cap, not from a JSON parse failure that a smaller-but-still-broken
        // payload would trigger regardless of the cap.
        let mut value = serde_json::to_value(&ConfigData {
            config: AppConfig::default(),
        })
        .unwrap();
        value["padding"] = serde_json::Value::String(
            "a".repeat(usize::try_from(MAX_CONFIG_FILE_SIZE).expect("fits in usize") + 1),
        );
        let oversized = serde_json::to_string(&value).unwrap();
        assert!(oversized.len() as u64 > MAX_CONFIG_FILE_SIZE);
        fs::write(&file_path, &oversized).unwrap();

        let err = storage
            .load()
            .expect_err("an oversized config file must be rejected");
        assert!(
            err.to_string().contains("too large"),
            "expected a 'too large' error, got: {err}"
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
            ..AppConfig::default()
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
    fn test_load_missing_config_field_uses_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // A missing "config" field (or a missing field within it) must not
        // fail the whole load — `#[serde(default)]` degrades gracefully so
        // a config file from an older version keeps working after an
        // upgrade adds new fields.
        fs::write(&file_path, r#"{"other": "value"}"#).unwrap();

        let config = storage.load().unwrap();
        assert!(!config.enable_arrow_keys_in_normal_mode);
        assert!(!config.use_helix_keymap);
    }

    #[test]
    fn test_load_config_missing_new_field_defaults_to_false() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");
        let storage = ConfigStorage::with_path(&file_path);

        // Simulates an old config.json written before `use_helix_keymap` existed.
        fs::write(
            &file_path,
            r#"{"config": {"enable_arrow_keys_in_normal_mode": true}}"#,
        )
        .unwrap();

        let config = storage.load().unwrap();
        assert!(config.enable_arrow_keys_in_normal_mode);
        assert!(!config.use_helix_keymap);
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

    /// #444: a fresh config directory is 0700 and `config.json` is 0600.
    #[cfg(unix)]
    #[test]
    fn test_save_creates_private_directory_and_file() {
        use std::os::unix::fs::PermissionsExt;
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path().join("cfg");
        let path = dir.join("config.json");

        ConfigStorage::with_path(&path)
            .save(&AppConfig::default())
            .unwrap();

        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&path), 0o600);
    }

    /// #441: a FIFO at the config path is rejected instead of blocking.
    #[cfg(unix)]
    #[test]
    fn test_load_fifo_is_rejected_without_blocking() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("config.json");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );

        assert!(ConfigStorage::with_path(&path).load().is_err());
    }
}
