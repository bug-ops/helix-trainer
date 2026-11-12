//! Keymap overlay: translates physically-pressed keys to canonical Helix
//! keys according to the user's Helix `config.toml`.
//!
//! Translation happens once, at the `KeyEvent` boundary in
//! `src/ui/key_dispatch/handlers.rs::handle_gameplay_input`, using [`PhysicalKey`] as
//! the sole lookup key. Everything downstream of that point — the
//! registry, FSRS card ids, quest matching, scenario `solution` — keeps
//! working with canonical key strings and needs no awareness of remapping.
//!
//! Parsing the user's `config.toml` into a [`KeymapOverlay`] lives here too
//! ([`resolve_startup_overlay`], [`load_keymap_file`]).

mod context;
mod keys;
mod load;
mod overlay;
mod parse;

pub use context::KeyContext;
pub use keys::{CanonicalKeys, GameplayKey, ParsePhysicalKeyError, PhysicalKey};
pub use load::{
    KeymapLoadError, KeymapReport, KeymapStatus, KeymapWarning, KeymapWarningReason, StartupKeymap,
    keymap_fingerprint_mismatch_message, load_keymap_file, resolve_startup_overlay, resolve_str,
};
pub use overlay::KeymapOverlay;
