//! Forward-translating keymap overlay

use std::collections::HashMap;
use std::sync::Arc;

use super::context::KeyContext;
use super::fingerprint::{Fnv1a64, KeymapFingerprint};
use super::keys::{CanonicalKeys, PhysicalKey};

/// A forward-only translation table from `(context, physical key)` to the
/// canonical Helix key sequence that should execute instead.
///
/// Built once at startup by `src/config/keymap/parse.rs` from the user's
/// Helix `config.toml`, then consulted on every gameplay keypress in
/// `src/input/handlers.rs::handle_gameplay_input`. The overlay *overrides*,
/// it never removes: a miss simply falls through to the stock keymap, so
/// unbound stock keys keep working exactly as before (this is Helix's own
/// layering behavior).
///
/// Cheap to clone (wraps an `Arc`), so it can be captured by value without
/// threading a reference through every call site.
#[derive(Debug, Clone, Default)]
pub struct KeymapOverlay {
    table: Arc<HashMap<(KeyContext, PhysicalKey), CanonicalKeys>>,
}

impl KeymapOverlay {
    /// The no-op overlay: every lookup misses, so translation is a no-op
    /// and the stock keymap runs unchanged. This is the default when
    /// `use_helix_keymap` is `false` or no config was loaded.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::input::keymap::{KeyContext, KeymapOverlay, PhysicalKey};
    /// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    ///
    /// let overlay = KeymapOverlay::identity();
    /// let key = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    /// assert!(overlay.lookup(KeyContext::Base, key).is_none());
    /// ```
    pub fn identity() -> Self {
        Self::default()
    }

    /// Whether this overlay has no bindings (equivalent to [`identity`](Self::identity)).
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Number of resolved bindings in this overlay.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Look up the canonical key sequence a physical key should translate
    /// to in the given context. Returns `None` on a miss (stock behavior).
    pub fn lookup(&self, context: KeyContext, key: PhysicalKey) -> Option<&CanonicalKeys> {
        self.table.get(&(context, key))
    }

    /// Build an overlay from already-resolved `(context, key) -> canonical`
    /// entries. Used by `src/config/keymap/parse.rs` once every binding has
    /// been validated.
    pub(crate) fn from_entries(
        entries: impl IntoIterator<Item = ((KeyContext, PhysicalKey), CanonicalKeys)>,
    ) -> Self {
        Self {
            table: Arc::new(entries.into_iter().collect()),
        }
    }

    /// Fingerprint of the resolved bindings, independent of `HashMap`
    /// iteration order, the toolchain and dependency versions.
    ///
    /// FNV-1a 64 over the bindings sorted by (context, key); each binding
    /// hashes its context name, Helix key string and bound canonical target
    /// as length-prefixed fields, so remapping a key to another command
    /// changes the value.
    ///
    /// Stored on [`UserProfile::keymap_fingerprint`](crate::gamification::UserProfile::keymap_fingerprint)
    /// so a mismatch at startup can flag that FSRS review history was
    /// recorded under a different mapping.
    pub fn fingerprint(&self) -> KeymapFingerprint {
        let mut entries: Vec<(&'static str, String, &str)> = self
            .table
            .iter()
            .map(|((context, key), canonical)| {
                (context.as_str(), key.to_helix_string(), canonical.as_str())
            })
            .collect();
        entries.sort_unstable();

        let mut hasher = Fnv1a64::new();
        for (context, key, target) in entries {
            hasher.write_field(context);
            hasher.write_field(&key);
            hasher.write_field(target);
        }
        KeymapFingerprint::Fnv1a64V1(hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: char) -> PhysicalKey {
        PhysicalKey::from_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
    }

    #[test]
    fn identity_overlay_always_misses() {
        let overlay = KeymapOverlay::identity();
        assert!(overlay.is_empty());
        assert!(overlay.lookup(KeyContext::Base, key('h')).is_none());
    }

    #[test]
    fn resolved_entry_is_found_by_context_and_key() {
        let overlay = KeymapOverlay::from_entries([(
            (KeyContext::Base, key('j')),
            CanonicalKeys::from_static("k"),
        )]);
        assert_eq!(
            overlay.lookup(KeyContext::Base, key('j')),
            Some(&CanonicalKeys::from_static("k"))
        );
        // Same physical key, different context: miss.
        assert!(overlay.lookup(KeyContext::Goto, key('j')).is_none());
    }

    #[test]
    fn identity_fingerprint_is_stable() {
        assert_eq!(
            KeymapOverlay::identity().fingerprint(),
            KeymapOverlay::identity().fingerprint()
        );
    }

    #[test]
    fn fingerprint_is_order_independent_and_content_sensitive() {
        let a = KeymapOverlay::from_entries([
            (
                (KeyContext::Base, key('j')),
                CanonicalKeys::from_static("k"),
            ),
            (
                (KeyContext::Base, key('k')),
                CanonicalKeys::from_static("j"),
            ),
        ]);
        let b = KeymapOverlay::from_entries([
            (
                (KeyContext::Base, key('k')),
                CanonicalKeys::from_static("j"),
            ),
            (
                (KeyContext::Base, key('j')),
                CanonicalKeys::from_static("k"),
            ),
        ]);
        assert_eq!(a.fingerprint(), b.fingerprint());
        assert_ne!(a.fingerprint(), KeymapOverlay::identity().fingerprint());
    }

    #[test]
    fn remapping_a_key_to_another_command_changes_the_fingerprint() {
        let remap = |target| {
            KeymapOverlay::from_entries([((KeyContext::Base, key('j')), target)]).fingerprint()
        };
        assert_ne!(
            remap(CanonicalKeys::from_static("k")),
            remap(CanonicalKeys::from_static("l"))
        );
    }

    #[test]
    fn context_and_key_are_part_of_the_fingerprint() {
        let at = |context, c| {
            KeymapOverlay::from_entries([((context, key(c)), CanonicalKeys::from_static("k"))])
                .fingerprint()
        };
        assert_ne!(at(KeyContext::Base, 'j'), at(KeyContext::Goto, 'j'));
        assert_ne!(at(KeyContext::Base, 'j'), at(KeyContext::Base, 'l'));
    }

    /// Pins the algorithm: a change here invalidates every stored fingerprint.
    #[test]
    fn fingerprint_value_is_pinned() {
        let overlay = KeymapOverlay::from_entries([(
            (KeyContext::Base, key('j')),
            CanonicalKeys::from_static("k"),
        )]);
        assert_eq!(
            overlay.fingerprint(),
            KeymapFingerprint::Fnv1a64V1(0x05aa_d1ba_399a_3e5d)
        );
    }

    #[test]
    fn clone_is_cheap_and_shares_state() {
        let overlay = KeymapOverlay::from_entries([(
            (KeyContext::Base, key('j')),
            CanonicalKeys::from_static("k"),
        )]);
        let cloned = overlay.clone();
        assert_eq!(cloned.len(), overlay.len());
    }
}
