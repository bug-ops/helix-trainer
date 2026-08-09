//! Normalized key newtypes for keymap overlay translation
//!
//! [`PhysicalKey`] and [`CanonicalKeys`] keep "what the user physically
//! pressed" and "what canonical Helix key sequence that resolves to"
//! type-distinct, so the rest of the input pipeline can never accidentally
//! feed a raw, unnormalized `KeyEvent` into a `HashMap` lookup.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::typestate::key_mapping::{
    is_named_key, map_macos_composed_char, parse_helix_key_string,
};

/// A normalized, hashable representation of a physically-pressed key.
///
/// This is the *sole* lookup key for [`KeymapOverlay`](super::overlay::KeymapOverlay)
/// translation. It differs from a raw `KeyEvent` in two ways:
///
/// 1. `kind` and `state` are dropped entirely — they're never compared.
///    Terminals that report `KeyEventKind::Repeat` or kitty keyboard
///    protocol `KeyEventState` bits (`CAPS_LOCK`, `NUM_LOCK`, `KEYPAD`)
///    would otherwise silently miss every overlay entry.
/// 2. SHIFT is normalized using **crossterm's** convention, not this
///    crate's [`normalize_key_event`](crate::input::typestate::normalize_key_event):
///    an uppercase ASCII char always carries `SHIFT`, and `SHIFT` on a
///    lowercase char uppercases it. This matches crossterm's own
///    `KeyEvent::normalize_case` (used by its `Hash`/`PartialEq` impls), so
///    `PhysicalKey` and a raw `KeyEvent` never disagree about whether
///    `Char('G')` carries `SHIFT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysicalKey {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl PhysicalKey {
    /// Normalize a raw `KeyEvent` (as delivered by crossterm) into a `PhysicalKey`.
    ///
    /// This is the only constructor that consumes a live `KeyEvent`; keymap
    /// config strings go through [`TryFrom<&str>`](#impl-TryFrom<%26str>-for-PhysicalKey)
    /// instead, which routes through this same normalization so the two
    /// paths can never disagree.
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    /// use helix_trainer::input::keymap::PhysicalKey;
    ///
    /// let lower = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
    /// let shifted = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT));
    /// let upper = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE));
    /// assert_ne!(lower, shifted);
    /// assert_eq!(shifted, upper); // both normalize to Char('G') + SHIFT
    /// ```
    pub fn from_event(event: KeyEvent) -> Self {
        // Destructuring drops `kind` and `state` (point 1 above).
        let KeyEvent {
            code, modifiers, ..
        } = event;
        Self::normalize(code, modifiers)
    }

    fn normalize(code: KeyCode, modifiers: KeyModifiers) -> Self {
        if let KeyCode::Char(ch) = code
            && let Some((base_char, alt_modifier)) = map_macos_composed_char(ch)
        {
            let mut modifiers = modifiers;
            modifiers.insert(alt_modifier);
            return Self::apply_shift_convention(KeyCode::Char(base_char), modifiers);
        }
        Self::apply_shift_convention(code, modifiers)
    }

    /// Replicates crossterm's `KeyEvent::normalize_case` exactly (point 2 above).
    fn apply_shift_convention(mut code: KeyCode, mut modifiers: KeyModifiers) -> Self {
        if let KeyCode::Char(c) = code {
            if c.is_ascii_uppercase() {
                modifiers.insert(KeyModifiers::SHIFT);
            } else if modifiers.contains(KeyModifiers::SHIFT) {
                code = KeyCode::Char(c.to_ascii_uppercase());
            }
        }
        Self { code, modifiers }
    }

    /// Render as a Helix-notation key string for display (e.g. `KeyHistory`
    /// when a remapped key was translated by the overlay).
    ///
    /// This is informational only, not a parser round-trip target like
    /// [`CanonicalKeys`] — it does not need to exactly match any
    /// particular `CMD_*` constant's spelling.
    pub fn label(&self) -> String {
        let mut label = String::new();
        if self.modifiers.contains(KeyModifiers::ALT) {
            label.push_str("Alt-");
        }
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            label.push_str("Ctrl-");
        }
        match self.code {
            KeyCode::Char(c) => label.push(c),
            KeyCode::Esc => label.push_str("Escape"),
            KeyCode::Enter => label.push_str("Enter"),
            KeyCode::Tab => label.push_str("Tab"),
            KeyCode::Backspace => label.push_str("Backspace"),
            KeyCode::Left => label.push_str("Left"),
            KeyCode::Right => label.push_str("Right"),
            KeyCode::Up => label.push_str("Up"),
            KeyCode::Down => label.push_str("Down"),
            other => label.push_str(&format!("{other:?}")),
        }
        label
    }
}

/// A Helix key string (e.g. `"A-c"`, `"j"`, `"Escape"`) did not parse into a key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid key string: {0:?}")]
pub struct ParsePhysicalKeyError(String);

impl TryFrom<&str> for PhysicalKey {
    type Error = ParsePhysicalKeyError;

    /// Parse a Helix keymap-config key string (e.g. `"A-c"`, `"G"`) into a
    /// `PhysicalKey`, routing through [`parse_helix_key_string`] and then
    /// the same normalization [`from_event`](Self::from_event) applies, so
    /// a config-sourced key and a live keypress that mean the same thing
    /// always hash equal.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        parse_helix_key_string(s)
            .map(PhysicalKey::from_event)
            .ok_or_else(|| ParsePhysicalKeyError(s.to_string()))
    }
}

/// One or more canonical Helix key tokens (e.g. `"h"`, `"gg"`, `"mi("`).
///
/// "Canonical" means: the exact key-string vocabulary this crate's
/// registry, `InputStateMachine`, and FSRS card ids already use — what the
/// stock keymap would call the command. A `CanonicalKeys` is produced only
/// by [`KeymapOverlay`](super::overlay::KeymapOverlay) resolution or by
/// wrapping an already-canonical single key (the identity/no-remap case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalKeys(Cow<'static, str>);

impl CanonicalKeys {
    /// Wrap a `'static` canonical key string (e.g. a `CMD_*` constant).
    pub const fn from_static(s: &'static str) -> Self {
        Self(Cow::Borrowed(s))
    }

    /// Wrap an owned, dynamically-built canonical key string.
    pub fn from_owned(s: String) -> Self {
        Self(Cow::Owned(s))
    }

    /// The underlying key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume `self`, returning the underlying `Cow`.
    pub fn into_cow(self) -> Cow<'static, str> {
        self.0
    }

    /// Split into the individual keys/tokens the `InputStateMachine` would
    /// consume one at a time to reach this sequence from `Base`.
    ///
    /// Three rules, in order:
    /// 1. The whole string starts with a recognized modifier prefix
    ///    (`A-`/`Alt-`/`C-`/`Ctrl-`/`S-`/`Shift-`, case-insensitive) -> the
    ///    entire string is one token (a modifier always applies to exactly
    ///    one key in this crate's vocabulary).
    /// 2. The whole string is a recognized named key (see
    ///    `is_named_key` in `key_mapping.rs`) -> one token.
    /// 3. Otherwise -> one token per `char`.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::input::keymap::CanonicalKeys;
    ///
    /// assert_eq!(CanonicalKeys::from_static("gg").tokens(), vec!["g", "g"]);
    /// assert_eq!(CanonicalKeys::from_static("mi(").tokens(), vec!["m", "i", "("]);
    /// assert_eq!(CanonicalKeys::from_static("Alt-*").tokens(), vec!["Alt-*"]);
    /// assert_eq!(CanonicalKeys::from_static("Escape").tokens(), vec!["Escape"]);
    /// ```
    pub fn tokens(&self) -> Vec<&str> {
        tokenize(&self.0)
    }

    /// Returns `true` when [`tokens`](Self::tokens) would produce exactly
    /// one token equal to the whole string, without allocating.
    ///
    /// Callers on the keystroke hot path (the overwhelmingly common case is
    /// a single, un-remapped key) should check this first and dispatch on
    /// [`as_str`](Self::as_str) directly, falling back to `tokens()` only
    /// when this returns `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::input::keymap::CanonicalKeys;
    ///
    /// assert!(CanonicalKeys::from_static("f").is_single_token());
    /// assert!(CanonicalKeys::from_static("Alt-*").is_single_token());
    /// assert!(!CanonicalKeys::from_static("gg").is_single_token());
    /// ```
    pub fn is_single_token(&self) -> bool {
        is_single_token(&self.0)
    }
}

impl From<Cow<'static, str>> for CanonicalKeys {
    fn from(s: Cow<'static, str>) -> Self {
        Self(s)
    }
}

const MODIFIER_PREFIXES: [&str; 6] = ["A-", "Alt-", "C-", "Ctrl-", "S-", "Shift-"];

fn starts_with_modifier_prefix(s: &str) -> bool {
    MODIFIER_PREFIXES.iter().any(|p| {
        s.get(..p.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(p))
    })
}

/// Mirrors `tokenize`'s token-count logic (rules 1-3) without allocating a
/// `Vec` - used by [`CanonicalKeys::is_single_token`] on the hot path. The
/// single-`char` check is tried first since it's both the cheapest test and
/// the dominant case (a bare, un-remapped key), ahead of the string-prefix
/// and named-key comparisons rules 1-2 need.
fn is_single_token(s: &str) -> bool {
    !s.is_empty()
        && (s.chars().next().is_some_and(|c| c.len_utf8() == s.len())
            || starts_with_modifier_prefix(s)
            || is_named_key(s))
}

fn tokenize(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    if is_single_token(s) {
        return vec![s];
    }
    let mut tokens = Vec::with_capacity(s.len());
    let mut idx = 0;
    for ch in s.chars() {
        let len = ch.len_utf8();
        tokens.push(&s[idx..idx + len]);
        idx += len;
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::registry::normal_registry;
    use crossterm::event::{KeyEventKind, KeyEventState};

    #[test]
    fn from_event_drops_kind_and_state() {
        let a = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        let b = PhysicalKey::from_event(KeyEvent::new_with_kind_and_state(
            KeyCode::Char('h'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
            KeyEventState::CAPS_LOCK | KeyEventState::KEYPAD,
        ));
        assert_eq!(a, b);
    }

    #[test]
    fn shift_convention_collapses_all_spellings_of_shift_g() {
        let spellings = [
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT),
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT),
        ];
        let normalized: Vec<_> = spellings.into_iter().map(PhysicalKey::from_event).collect();
        assert!(normalized.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn shift_convention_leaves_plain_lowercase_alone() {
        let a = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        let b = PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE));
        assert_ne!(a, b);
    }

    #[test]
    fn try_from_str_matches_from_event() {
        let from_str = PhysicalKey::try_from("A-c").unwrap();
        let from_event =
            PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT));
        assert_eq!(from_str, from_event);
    }

    #[test]
    fn try_from_str_rejects_garbage() {
        assert!(PhysicalKey::try_from("not-a-key").is_err());
    }

    #[test]
    fn label_renders_helix_notation() {
        assert_eq!(
            PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)).label(),
            "h"
        );
        assert_eq!(
            PhysicalKey::from_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT)).label(),
            "Alt-c"
        );
        assert_eq!(
            PhysicalKey::from_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).label(),
            "Escape"
        );
    }

    #[test]
    fn tokens_examples() {
        assert_eq!(CanonicalKeys::from_static("gg").tokens(), vec!["g", "g"]);
        assert_eq!(
            CanonicalKeys::from_static("mi(").tokens(),
            vec!["m", "i", "("]
        );
        assert_eq!(CanonicalKeys::from_static("Alt-*").tokens(), vec!["Alt-*"]);
        assert_eq!(
            CanonicalKeys::from_static("Ctrl-b").tokens(),
            vec!["Ctrl-b"]
        );
        assert_eq!(CanonicalKeys::from_static("f").tokens(), vec!["f"]);
        assert_eq!(
            CanonicalKeys::from_static("Escape").tokens(),
            vec!["Escape"]
        );
    }

    /// Total-function gate (R2): every one of the registry's 84 real key
    /// strings must tokenize into tokens that each round-trip through
    /// `parse_helix_key_string`, so the tokenizer and the parser can never
    /// silently drift apart.
    ///
    /// Was 86 before `select_regex`/`split_selection` (`s`/`S`) were pulled
    /// out of the registry — their handlers take a `pattern` argument that
    /// doesn't fit the registry's fixed `CommandHandler` signature, so they
    /// dispatch directly instead of through a registered entry.
    #[test]
    fn tokenizes_all_registry_keys_into_reparsable_tokens() {
        let registry = normal_registry();
        let mut checked = 0;
        for meta in registry.all_commands() {
            checked += 1;
            for token in CanonicalKeys::from_static(meta.key).tokens() {
                assert!(
                    parse_helix_key_string(token).is_some(),
                    "token {:?} of registry key {:?} does not round-trip",
                    token,
                    meta.key
                );
            }
        }
        assert!(checked >= 84);
    }

    /// Equivalence gate: `is_single_token` must never drift from
    /// `tokenize().len() == 1` - it exists purely to answer that question
    /// without allocating, so a mismatch would silently break the
    /// keystroke hot path's fast case.
    #[test]
    fn is_single_token_matches_tokens_len_across_inputs() {
        let mut inputs: Vec<&str> = vec![
            "",       // empty string: zero tokens
            "é",      // single multi-byte char: one token
            "Escape", // named key: one token
            "Alt-*",  // modifier-prefixed: one token
            "gg",     // two single-char tokens
            "mi(",    // three single-char tokens
        ];
        let registry = normal_registry();
        for meta in registry.all_commands() {
            inputs.push(meta.key);
        }
        for s in inputs {
            assert_eq!(
                is_single_token(s),
                tokenize(s).len() == 1,
                "is_single_token/tokenize mismatch for {s:?}"
            );
        }
    }
}
