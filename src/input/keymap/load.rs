//! Loading and resolving the user's Helix `config.toml` keymap.
//!
//! Opt-in via `AppConfig::use_helix_keymap`; when enabled,
//! [`resolve_startup_overlay`] resolves a fixed path
//! (Helix's own `config.toml` location: `$XDG_CONFIG_HOME/helix/` or
//! `~/.config/helix/` on Unix, `%AppData%\helix\` on Windows — no arbitrary
//! user-supplied path) and calls [`load_keymap_file`] to produce a
//! [`KeymapOverlay`] plus a [`KeymapStatus`] for the `ui` layer to report.

use std::path::PathBuf;

use rust_i18n::t;

pub use super::parse::{
    KeymapLoadError, KeymapReport, KeymapWarning, KeymapWarningReason, load_keymap_file,
    resolve_str,
};

use super::KeymapOverlay;
use crate::config::AppConfig;
#[cfg(not(windows))]
use crate::config::data_dir::xdg_config_base_from_env;

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
    let base = xdg_config_base_from_env();
    Some(base?.join("helix").join("config.toml"))
}

/// How the startup keymap resolution ended; the `ui` layer renders the matching message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeymapStatus {
    /// `use_helix_keymap` is off: nothing was loaded and nothing is worth telling the user.
    Disabled,
    /// The Helix config was loaded; [`StartupKeymap::report`] counts what was applied and ignored.
    Applied,
    /// The Helix config file (or the directory it lives in) was not found.
    NotFound,
    /// The Helix config file could not be loaded (invalid TOML, too many bindings, ...).
    Invalid,
}

/// The result of resolving the keymap at startup.
#[derive(Debug, Clone)]
pub struct StartupKeymap {
    /// The gameplay overlay; the identity overlay unless [`status`](Self::status) is `Applied`.
    pub overlay: KeymapOverlay,
    /// What was applied and ignored; empty unless [`status`](Self::status) is `Applied`.
    pub report: KeymapReport,
    /// How the resolution ended.
    pub status: KeymapStatus,
}

impl StartupKeymap {
    fn fallback(status: KeymapStatus) -> Self {
        Self {
            overlay: KeymapOverlay::identity(),
            report: KeymapReport::default(),
            status,
        }
    }
}

/// Resolve the gameplay keymap overlay for a freshly-loaded [`AppConfig`].
///
/// Returns [`KeymapOverlay::identity`] (no translation) whenever
/// `config.use_helix_keymap` is `false`, or when loading fails for any
/// reason — a missing file, invalid TOML, or too many bindings never
/// blocks startup or falls back to a partially-applied overlay.
///
/// Alongside the overlay, returns the [`KeymapReport`] (empty when the
/// feature is off or loading failed) and the [`KeymapStatus`] the `ui` layer
/// turns into a message. The caller is expected to store the report on
/// `ConfigState` — a startup toast alone is gone within seconds, and
/// `tracing` output isn't visible in the TUI, so the report is what keeps
/// this information reachable for as long as the session runs (e.g. a future
/// status screen render).
///
/// # Examples
///
/// ```
/// use helix_trainer::config::AppConfig;
/// use helix_trainer::input::keymap::{KeymapStatus, resolve_startup_overlay};
///
/// let keymap = resolve_startup_overlay(&AppConfig::default());
/// assert!(keymap.overlay.is_empty());
/// assert_eq!(keymap.report.applied, 0);
/// assert_eq!(keymap.status, KeymapStatus::Disabled); // use_helix_keymap defaults to false
/// ```
pub fn resolve_startup_overlay(config: &AppConfig) -> StartupKeymap {
    if !config.use_helix_keymap {
        return StartupKeymap::fallback(KeymapStatus::Disabled);
    }

    let Some(path) = default_helix_config_path() else {
        tracing::warn!("Helix keymap enabled but the OS config directory could not be determined");
        return StartupKeymap::fallback(KeymapStatus::NotFound);
    };
    match load_keymap_file(&path) {
        Ok((overlay, report)) => {
            tracing::info!(
                applied = report.applied,
                ignored = report.ignored.len(),
                path = %path.display(),
                "Helix keymap loaded"
            );
            StartupKeymap {
                overlay,
                report,
                status: KeymapStatus::Applied,
            }
        }
        Err(KeymapLoadError::NotFound) => {
            tracing::warn!(
                path = %path.display(),
                "Helix keymap enabled but config file not found"
            );
            StartupKeymap::fallback(KeymapStatus::NotFound)
        }
        Err(KeymapLoadError::Security(e)) => {
            tracing::warn!(error = %e, path = %path.display(), "Failed to load Helix keymap config");
            StartupKeymap::fallback(KeymapStatus::Invalid)
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
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn disabled_config_returns_empty_overlay_and_report_together() {
        let keymap = resolve_startup_overlay(&AppConfig::default());
        assert!(keymap.overlay.is_empty());
        assert_eq!(keymap.report.applied, 0);
        assert!(keymap.report.ignored.is_empty());
        assert_eq!(keymap.status, KeymapStatus::Disabled);
    }
}
