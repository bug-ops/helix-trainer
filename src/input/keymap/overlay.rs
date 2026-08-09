//! Forward-translating keymap overlay

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::context::KeyContext;
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

    /// Hash over the sorted resolved bindings, order-independent so the
    /// same keymap always fingerprints the same way regardless of
    /// `HashMap` iteration order.
    ///
    /// Stored on [`UserProfile::keymap_fingerprint`](crate::gamification::UserProfile::keymap_fingerprint)
    /// so a mismatch at startup can flag that FSRS review history was
    /// recorded under a different mapping.
    pub fn fingerprint(&self) -> u64 {
        let mut entries: Vec<(String, &str)> = self
            .table
            .iter()
            .map(|((context, key), canonical)| (format!("{context:?}|{key:?}"), canonical.as_str()))
            .collect();
        entries.sort_unstable();

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        entries.hash(&mut hasher);
        hasher.finish()
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
    fn clone_is_cheap_and_shares_state() {
        let overlay = KeymapOverlay::from_entries([(
            (KeyContext::Base, key('j')),
            CanonicalKeys::from_static("k"),
        )]);
        let cloned = overlay.clone();
        assert_eq!(cloned.len(), 1);
    }
}
