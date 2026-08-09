//! Keymap overlay: translates physically-pressed keys to canonical Helix
//! keys according to the user's Helix `config.toml`.
//!
//! Translation happens once, at the `KeyEvent` boundary in
//! `src/input/handlers.rs::handle_gameplay_input`, using [`PhysicalKey`] as
//! the sole lookup key. Everything downstream of that point — the
//! registry, FSRS card ids, quest matching, scenario `solution` — keeps
//! working with canonical key strings and needs no awareness of remapping.
//!
//! See `src/config/keymap/` for parsing the user's `config.toml` into a
//! [`KeymapOverlay`].

mod context;
mod keys;
mod overlay;

pub use context::KeyContext;
pub use keys::{CanonicalKeys, ParsePhysicalKeyError, PhysicalKey};
pub use overlay::KeymapOverlay;
