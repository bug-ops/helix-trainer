//! Resolution of the trainer's data directory and the XDG base shared with
//! the Helix keymap path.
//!
//! On Linux and Windows the data directory is `dirs::config_dir()/helix-trainer`.
//! When no per-user directory can be determined there is no data directory at
//! all: nothing falls back to the current working directory.
//! On macOS it is `~/.config/helix-trainer` (honoring a non-empty absolute
//! `$XDG_CONFIG_HOME`), matching where Helix keeps its own configuration; the
//! former `~/Library/Application Support/helix-trainer` becomes the
//! [`DataDir::legacy_file`] source for a one-time per-file copy (see
//! [`crate::config::json_store::JsonStore::migrating_from`]).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

const APP_DIR: &str = "helix-trainer";

/// XDG-style base directory: `xdg_config_home` when it is a non-empty
/// absolute path, otherwise `<home>/.config` when `home` is absolute.
/// A relative value is treated as unset so the data directory never depends on
/// the current working directory.
///
/// # Examples
///
/// ```
/// use std::ffi::OsStr;
/// use std::path::Path;
/// use helix_trainer::config::data_dir::xdg_config_base;
///
/// let home = Path::new("/home/u");
/// assert_eq!(xdg_config_base(None, Some(home)), Some(home.join(".config")));
/// assert_eq!(
///     xdg_config_base(Some(OsStr::new("/x")), Some(home)),
///     Some("/x".into())
/// );
/// assert_eq!(xdg_config_base(Some(OsStr::new("rel")), None), None);
/// assert_eq!(xdg_config_base(None, Some(Path::new("rel"))), None);
/// ```
pub fn xdg_config_base(xdg_config_home: Option<&OsStr>, home: Option<&Path>) -> Option<PathBuf> {
    xdg_config_home
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home.filter(|h| h.is_absolute()).map(|h| h.join(".config")))
}

/// [`xdg_config_base`] resolved from `$XDG_CONFIG_HOME` and the home directory.
pub fn xdg_config_base_from_env() -> Option<PathBuf> {
    xdg_config_base(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        dirs::home_dir().as_deref(),
    )
}

/// The directory holding `profile.json` and `config.json`, plus the previous
/// location files are migrated from when the platform's directory changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDir {
    current: PathBuf,
    legacy: Option<PathBuf>,
}

impl DataDir {
    /// Resolves the data directory for the running platform, or `None` when
    /// no per-user configuration directory can be determined (no usable
    /// `HOME` or `XDG_CONFIG_HOME`). There is deliberately no fallback to the
    /// current working directory.
    pub fn resolve() -> Option<Self> {
        Self::resolve_for(
            cfg!(target_os = "macos"),
            xdg_config_base_from_env(),
            dirs::config_dir(),
        )
    }

    /// Pure resolution behind [`DataDir::resolve`]: on macOS with a known XDG
    /// base the data directory moves there and the OS config directory
    /// becomes the legacy one; everywhere else it is the OS config directory
    /// and there is no legacy location. Returns `None` when neither base is
    /// known.
    pub fn resolve_for(
        macos: bool,
        xdg_base: Option<PathBuf>,
        os_config_dir: Option<PathBuf>,
    ) -> Option<Self> {
        let os_dir = os_config_dir
            .filter(|d| d.is_absolute())
            .map(|d| d.join(APP_DIR));
        match xdg_base.filter(|b| b.is_absolute()) {
            Some(base) if macos => {
                let current = base.join(APP_DIR);
                let legacy = os_dir.filter(|d| *d != current);
                Some(Self { current, legacy })
            }
            _ => os_dir.map(|current| Self {
                current,
                legacy: None,
            }),
        }
    }

    /// Directory the application reads and writes.
    pub fn dir(&self) -> &Path {
        &self.current
    }

    /// Path of `name` in the current directory.
    pub fn file(&self, name: &str) -> PathBuf {
        self.current.join(name)
    }

    /// Path of `name` in the previous directory, when the location moved.
    pub fn legacy_file(&self, name: &str) -> Option<PathBuf> {
        self.legacy.as_ref().map(|d| d.join(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn base(xdg: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
        xdg_config_base(xdg.map(Path::as_os_str), home)
    }

    #[test]
    fn base_defaults_to_home_dot_config() {
        let home = abs("h");
        assert_eq!(base(None, Some(&home)), Some(home.join(".config")));
    }

    #[test]
    fn base_honors_absolute_xdg_config_home() {
        let xdg = abs("x");
        assert_eq!(base(Some(&xdg), Some(&abs("h"))), Some(xdg));
    }

    #[test]
    fn base_ignores_empty_or_relative_xdg_config_home() {
        let home = abs("h");
        let fallback = Some(home.join(".config"));
        assert_eq!(base(Some(Path::new("")), Some(&home)), fallback);
        assert_eq!(base(Some(Path::new("rel")), Some(&home)), fallback);
    }

    #[test]
    fn base_treats_relative_home_as_unset() {
        assert_eq!(base(None, Some(Path::new("rel"))), None);
        assert_eq!(base(Some(Path::new("rel")), Some(Path::new(""))), None);
    }

    #[test]
    fn resolve_for_ignores_relative_bases() {
        assert_eq!(
            DataDir::resolve_for(false, None, Some(PathBuf::from("rel/.config"))),
            None
        );
        assert_eq!(
            DataDir::resolve_for(
                true,
                Some(PathBuf::from("rel")),
                Some(PathBuf::from("rel2"))
            ),
            None
        );
        let dirs = DataDir::resolve_for(true, Some(PathBuf::from("rel")), Some(abs("os"))).unwrap();
        assert_eq!(dirs.dir(), abs("os/helix-trainer"));
    }

    #[test]
    fn base_is_none_without_any_directory() {
        assert_eq!(base(None, None), None);
        assert_eq!(base(Some(Path::new("")), None), None);
    }

    #[test]
    fn macos_moves_to_xdg_base_and_keeps_os_dir_as_legacy() {
        let dirs =
            DataDir::resolve_for(true, Some(abs("home/.config")), Some(abs("home/Lib"))).unwrap();
        assert_eq!(dirs.dir(), abs("home/.config/helix-trainer"));
        assert_eq!(
            dirs.legacy_file("profile.json"),
            Some(abs("home/Lib/helix-trainer/profile.json"))
        );
    }

    #[test]
    fn macos_without_a_distinct_legacy_dir_has_none() {
        let dirs = DataDir::resolve_for(true, Some(abs("c")), Some(abs("c"))).unwrap();
        assert_eq!(dirs.legacy_file("profile.json"), None);
    }

    #[test]
    fn other_platforms_keep_the_os_config_dir() {
        let dirs = DataDir::resolve_for(false, Some(abs("xdg")), Some(abs("os"))).unwrap();
        assert_eq!(
            dirs.file("config.json"),
            abs("os/helix-trainer/config.json")
        );
        assert_eq!(dirs.legacy_file("config.json"), None);
    }

    #[test]
    fn macos_without_xdg_base_falls_back_to_os_dir() {
        let dirs = DataDir::resolve_for(true, None, Some(abs("os"))).unwrap();
        assert_eq!(dirs.dir(), abs("os/helix-trainer"));
        assert_eq!(dirs.legacy_file("profile.json"), None);
    }

    #[test]
    fn missing_os_dir_resolves_to_none_instead_of_cwd() {
        assert_eq!(DataDir::resolve_for(false, None, None), None);
        assert_eq!(DataDir::resolve_for(false, Some(abs("xdg")), None), None);
        assert_eq!(DataDir::resolve_for(true, None, None), None);
    }

    #[test]
    fn macos_with_xdg_base_resolves_without_an_os_dir() {
        let dirs = DataDir::resolve_for(true, Some(abs("x")), None).unwrap();
        assert_eq!(dirs.dir(), abs("x/helix-trainer"));
        assert_eq!(dirs.legacy_file("profile.json"), None);
    }
}
