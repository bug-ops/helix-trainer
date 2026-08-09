//! Loading and resolving the user's Helix `config.toml` keymap.
//!
//! Opt-in via `AppConfig::use_helix_keymap`; when enabled,
//! [`resolve_startup_overlay`] resolves a fixed path
//! (`dirs::config_dir()/helix/config.toml` — no arbitrary user-supplied
//! path) and calls [`load_keymap_file`] to produce a
//! [`crate::input::keymap::KeymapOverlay`] plus a startup notification
//! summarizing what was applied and ignored.

mod parse;

use std::path::PathBuf;

use rust_i18n::t;

pub use parse::{
    KeymapLoadError, KeymapReport, KeymapWarning, KeymapWarningReason, load_keymap_file,
    resolve_str,
};

use crate::config::AppConfig;
use crate::input::keymap::KeymapOverlay;

/// `dirs::config_dir()/helix/config.toml` — the one path this feature ever
/// reads. Not user-configurable (see M2 in the design decision record):
/// `use_helix_keymap` is a plain `bool`, not a path, so this trainer never
/// reads an arbitrary file on the user's behalf.
///
/// Returns `None` when `dirs::config_dir()` can't be determined (`HOME`/
/// `XDG_CONFIG_HOME` unset). Falling back to a CWD-relative `./helix/config.toml`
/// in that case would turn the "fixed path" guarantee into one an attacker
/// could plant a file at by controlling the working directory the trainer
/// is launched from — treated as "no config" instead, matching a missing
/// file.
fn default_helix_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("helix");
    path.push("config.toml");
    Some(path)
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

    #[test]
    fn disabled_config_returns_empty_overlay_and_report_together() {
        let (overlay, report, message) = resolve_startup_overlay(&AppConfig::default());
        assert!(overlay.is_empty());
        assert_eq!(report.applied, 0);
        assert!(report.ignored.is_empty());
        assert!(message.is_none());
    }
}
