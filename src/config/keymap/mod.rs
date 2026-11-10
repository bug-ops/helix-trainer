//! Loading and resolving the user's Helix `config.toml` keymap.
//!
//! Opt-in via `AppConfig::use_helix_keymap`; when enabled,
//! [`resolve_startup_overlay`] resolves a fixed path
//! (Helix's own `config.toml` location: `$XDG_CONFIG_HOME/helix/` or
//! `~/.config/helix/` on Unix, `%AppData%\helix\` on Windows — no arbitrary
//! user-supplied path) and calls [`load_keymap_file`] to produce a
//! [`crate::input::keymap::KeymapOverlay`] plus a startup notification
//! summarizing what was applied and ignored.

mod parse;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use rust_i18n::t;

pub use parse::{
    KeymapLoadError, KeymapReport, KeymapWarning, KeymapWarningReason, load_keymap_file,
    resolve_str,
};

use crate::config::AppConfig;
use crate::input::keymap::KeymapOverlay;

/// The Helix `config.toml` path, resolved the way Helix does: on Unix
/// (including macOS) `$XDG_CONFIG_HOME/helix/config.toml` when
/// `XDG_CONFIG_HOME` is a non-empty absolute path, else
/// `~/.config/helix/config.toml`; on Windows `%AppData%\helix\config.toml`.
/// It is the one path this feature ever reads. Not user-configurable (see M2
/// in the design decision record): `use_helix_keymap` is a plain `bool`, not
/// a path, so this trainer never reads an arbitrary file on the user's
/// behalf.
///
/// Returns `None` when the base directory can't be determined. Falling back
/// to a CWD-relative `./helix/config.toml` in that case would turn the
/// "fixed path" guarantee into one an attacker could plant a file at by
/// controlling the working directory the trainer is launched from — treated
/// as "no config" instead, matching a missing file.
fn default_helix_config_path() -> Option<PathBuf> {
    #[cfg(windows)]
    let base = dirs::config_dir();
    #[cfg(not(windows))]
    let base = helix_config_base(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        dirs::home_dir().as_deref(),
    );
    Some(base?.join("helix").join("config.toml"))
}

/// Pure XDG-style base directory selection behind
/// [`default_helix_config_path`]: `xdg_config_home` when non-empty and
/// absolute, otherwise `<home>/.config`.
#[cfg_attr(windows, allow(dead_code))] // NOTE: only the non-Windows path uses it
fn helix_config_base(xdg_config_home: Option<&OsStr>, home: Option<&Path>) -> Option<PathBuf> {
    xdg_config_home
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home.map(|h| h.join(".config")))
}

/// Resolve the gameplay keymap overlay for a freshly-loaded [`AppConfig`].
///
/// Returns [`KeymapOverlay::identity`] (no translation) whenever
/// `config.use_helix_keymap` is `false`, or when loading fails for any
/// reason — a missing file, invalid TOML, or too many bindings never
/// blocks startup or falls back to a partially-applied overlay.
///
/// Alongside the overlay, returns the [`KeymapReport`] (empty when the
/// feature is off or loading failed) and a localized summary message, if
/// there's anything worth telling the user about. The caller is expected
/// to store the report on `ConfigState` — a startup toast alone is gone
/// within seconds, and `tracing` output isn't visible in the TUI, so the
/// report is what keeps this information reachable for as long as the
/// session runs (e.g. a future status screen render).
///
/// # Examples
///
/// ```
/// use helix_trainer::config::AppConfig;
/// use helix_trainer::config::keymap::resolve_startup_overlay;
///
/// let (overlay, report, message) = resolve_startup_overlay(&AppConfig::default());
/// assert!(overlay.is_empty());
/// assert_eq!(report.applied, 0);
/// assert!(message.is_none()); // use_helix_keymap defaults to false
/// ```
pub fn resolve_startup_overlay(
    config: &AppConfig,
) -> (KeymapOverlay, KeymapReport, Option<String>) {
    if !config.use_helix_keymap {
        return (KeymapOverlay::identity(), KeymapReport::default(), None);
    }

    let Some(path) = default_helix_config_path() else {
        tracing::warn!("Helix keymap enabled but the OS config directory could not be determined");
        return (
            KeymapOverlay::identity(),
            KeymapReport::default(),
            Some(t!("keymap.startup_not_found").to_string()),
        );
    };
    match load_keymap_file(&path) {
        Ok((overlay, report)) => {
            tracing::info!(
                applied = report.applied,
                ignored = report.ignored.len(),
                path = %path.display(),
                "Helix keymap loaded"
            );
            let message = t!(
                "keymap.startup_summary",
                applied = report.applied,
                ignored = report.ignored.len()
            )
            .to_string();
            (overlay, report, Some(message))
        }
        Err(KeymapLoadError::NotFound) => {
            tracing::warn!(
                path = %path.display(),
                "Helix keymap enabled but config file not found"
            );
            (
                KeymapOverlay::identity(),
                KeymapReport::default(),
                Some(t!("keymap.startup_not_found").to_string()),
            )
        }
        Err(KeymapLoadError::Security(e)) => {
            tracing::warn!(error = %e, path = %path.display(), "Failed to load Helix keymap config");
            (
                KeymapOverlay::identity(),
                KeymapReport::default(),
                Some(t!("keymap.startup_invalid").to_string()),
            )
        }
    }
}

/// Localized notification text for a keymap fingerprint mismatch: the
/// loaded profile's FSRS review history was recorded under a different
/// resolved keymap than the one now active.
///
/// A free function rather than inline `t!` at the call site because the
/// caller (`data_handling.rs`) lives in the binary crate, which never
/// invokes `rust_i18n::i18n!` itself - only this library crate does.
pub fn keymap_fingerprint_mismatch_message() -> String {
    t!("keymap.fingerprint_mismatch").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }

    fn base(xdg: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
        helix_config_base(xdg.map(Path::as_os_str), home)
    }

    #[test]
    fn helix_base_defaults_to_home_dot_config() {
        let home = abs("h");
        assert_eq!(base(None, Some(&home)), Some(home.join(".config")));
    }

    #[test]
    fn helix_base_honors_absolute_xdg_config_home() {
        let xdg = abs("x");
        assert_eq!(base(Some(&xdg), Some(&abs("h"))), Some(xdg));
    }

    #[test]
    fn helix_base_ignores_empty_or_relative_xdg_config_home() {
        let home = abs("h");
        let fallback = Some(home.join(".config"));
        assert_eq!(base(Some(Path::new("")), Some(&home)), fallback);
        assert_eq!(base(Some(Path::new("rel")), Some(&home)), fallback);
    }

    #[test]
    fn helix_base_is_none_without_any_directory() {
        assert_eq!(base(None, None), None);
        assert_eq!(base(Some(Path::new("")), None), None);
    }

    #[test]
    fn disabled_config_returns_empty_overlay_and_report_together() {
        let (overlay, report, message) = resolve_startup_overlay(&AppConfig::default());
        assert!(overlay.is_empty());
        assert_eq!(report.applied, 0);
        assert!(report.ignored.is_empty());
        assert!(message.is_none());
    }
}
