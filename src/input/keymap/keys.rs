//! Normalized key newtypes for keymap overlay translation
//!
//! [`PhysicalKey`] and [`CanonicalKeys`] keep "what the user physically
//! pressed" and "what canonical Helix key sequence that resolves to"
//! type-distinct, so the rest of the input pipeline can never accidentally
//! feed a raw, unnormalized `KeyEvent` into a `HashMap` lookup.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::typestate::key_mapping::{
    is_named_key, key_to_command_string, map_macos_composed_char, parse_helix_key_string,
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
            other => label = format!("{label}{other:?}"),
        }
        label
    }
}

impl PhysicalKey {
    /// Render as a Helix key string that fully identifies the key: every modifier
    /// as an `S-`, `A-` or `C-` prefix in that order, then the key.
    ///
    /// Unlike [`label`](Self::label) it is stable across crossterm versions, because
    /// it never uses `Debug` output, so it can be persisted (the keymap fingerprint).
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::input::keymap::PhysicalKey;
    ///
    /// assert_eq!(PhysicalKey::try_from("A-c").unwrap().to_helix_string(), "A-c");
    /// assert_eq!(PhysicalKey::try_from("G").unwrap().to_helix_string(), "S-G");
    /// assert_eq!(PhysicalKey::try_from("C-Left").unwrap().to_helix_string(), "C-Left");
    /// ```
    pub fn to_helix_string(&self) -> String {
        let mut out = String::new();
        for (modifier, prefix) in [
            (KeyModifiers::SHIFT, "S-"),
            (KeyModifiers::ALT, "A-"),
            (KeyModifiers::CONTROL, "C-"),
        ] {
            if self.modifiers.contains(modifier) {
                out.push_str(prefix);
            }
        }
        match self.code {
            KeyCode::Char(c) => out.push(c),
            KeyCode::F(n) => {
                out.push('F');
                out.push_str(&n.to_string());
            }
            KeyCode::Esc => out.push_str("Escape"),
            KeyCode::Enter => out.push_str("Enter"),
            KeyCode::Tab => out.push_str("Tab"),
            KeyCode::Backspace => out.push_str("Backspace"),
            KeyCode::Left => out.push_str("Left"),
            KeyCode::Right => out.push_str("Right"),
            KeyCode::Up => out.push_str("Up"),
            KeyCode::Down => out.push_str("Down"),
            KeyCode::Home => out.push_str("Home"),
            KeyCode::End => out.push_str("End"),
            KeyCode::PageUp => out.push_str("PageUp"),
            KeyCode::PageDown => out.push_str("PageDown"),
            KeyCode::Insert => out.push_str("Insert"),
            KeyCode::Delete => out.push_str("Delete"),
            // The keymap parser produces none of the remaining codes, so no overlay entry carries one.
            _ => out.push_str("Unsupported"),
        }
        out
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
///
/// The key events are parsed once, at construction, and carried as typed
/// values. Equality, hashing and the keymap fingerprint use the source text
/// only, so they stay independent of how the events are represented.
#[derive(Debug, Clone)]
pub struct CanonicalKeys {
    text: Cow<'static, str>,
    keys: Keys,
}

#[derive(Debug, Clone)]
enum Keys {
    One(KeyEvent),
    /// Two or more events; `CanonicalKeys::parse` is the only constructor.
    Many(Box<[KeyEvent]>),
}

impl PartialEq for CanonicalKeys {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl Eq for CanonicalKeys {}

impl std::hash::Hash for CanonicalKeys {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.text.hash(state);
    }
}

impl CanonicalKeys {
    /// Parse a canonical key string (e.g. a `CMD_*` constant) into its key events.
    ///
    /// # Errors
    ///
    /// Returns [`ParsePhysicalKeyError`] when `text` is empty or one of its
    /// [`tokens`](Self::tokens) is not a key.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::input::keymap::CanonicalKeys;
    ///
    /// assert_eq!(CanonicalKeys::parse("gg").unwrap().events().len(), 2);
    /// assert!(CanonicalKeys::parse("Alt-nonsense").is_err());
    /// ```
    pub fn parse(text: impl Into<Cow<'static, str>>) -> Result<Self, ParsePhysicalKeyError> {
        let text = text.into();
        let mut events = tokenize(&text)
            .into_iter()
            .map(|token| {
                parse_helix_key_string(token)
                    .ok_or_else(|| ParsePhysicalKeyError(token.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let keys = match events.len() {
            0 => return Err(ParsePhysicalKeyError(text.into_owned())),
            1 => Keys::One(events.remove(0)),
            _ => Keys::Many(events.into()),
        };
        Ok(Self { text, keys })
    }

    /// The canonical keys a stock Normal-mode key press stands for, or `None`
    /// where the key has no command (it is dropped).
    ///
    /// # Examples
    ///
    /// ```
    /// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    /// use helix_trainer::input::keymap::CanonicalKeys;
    ///
    /// let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    /// assert_eq!(CanonicalKeys::from_event(x).unwrap().as_str(), "x");
    /// let f5 = KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE);
    /// assert!(CanonicalKeys::from_event(f5).is_none());
    /// ```
    pub fn from_event(event: KeyEvent) -> Option<Self> {
        let text = key_to_command_string(event)?;
        Self::parse(text)
            .inspect_err(|error| tracing::warn!(%error, "unparsable key token; dropping"))
            .ok()
    }

    /// The underlying key string.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The underlying key string, cheap to clone for a `'static` text.
    pub fn to_cow(&self) -> Cow<'static, str> {
        self.text.clone()
    }

    /// The key events, in the order the `InputStateMachine` consumes them.
    pub fn events(&self) -> &[KeyEvent] {
        match &self.keys {
            Keys::One(event) => std::slice::from_ref(event),
            Keys::Many(events) => events,
        }
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
    /// assert_eq!(CanonicalKeys::parse("gg").unwrap().tokens(), vec!["g", "g"]);
    /// assert_eq!(CanonicalKeys::parse("mi(").unwrap().tokens(), vec!["m", "i", "("]);
    /// assert_eq!(CanonicalKeys::parse("Alt-*").unwrap().tokens(), vec!["Alt-*"]);
    /// assert_eq!(CanonicalKeys::parse("Escape").unwrap().tokens(), vec!["Escape"]);
    /// ```
    pub fn tokens(&self) -> Vec<&str> {
        tokenize(&self.text)
    }

    /// Returns `true` when the sequence is exactly one key.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::input::keymap::CanonicalKeys;
    ///
    /// assert!(CanonicalKeys::parse("f").unwrap().is_single_token());
    /// assert!(CanonicalKeys::parse("Alt-*").unwrap().is_single_token());
    /// assert!(!CanonicalKeys::parse("gg").unwrap().is_single_token());
    /// ```
    pub fn is_single_token(&self) -> bool {
        matches!(self.keys, Keys::One(_))
    }
}

/// What a gameplay screen receives for one keystroke.
///
/// Insert-mode text is never a canonical key (it can be a space, a non-ASCII
/// character or composed text), so it travels as [`Typed`](Self::Typed) and is
/// never parsed; every other mode receives [`Keys`](Self::Keys).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayKey {
    /// Text typed in Insert mode, inserted as is.
    Typed(Cow<'static, str>),
    /// A canonical key sequence for the Normal- or Select-mode state machine.
    Keys(CanonicalKeys),
}

impl GameplayKey {
    /// The key text: the typed text, or the canonical key string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Typed(text) => text,
            Self::Keys(keys) => keys.as_str(),
        }
    }
}

impl From<CanonicalKeys> for GameplayKey {
    fn from(keys: CanonicalKeys) -> Self {
        Self::Keys(keys)
    }
}

const MODIFIER_PREFIXES: [&str; 6] = ["A-", "Alt-", "C-", "Ctrl-", "S-", "Shift-"];

fn starts_with_modifier_prefix(s: &str) -> bool {
    MODIFIER_PREFIXES.iter().any(|p| {
        s.get(..p.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(p))
    })
}

fn tokenize(s: &str) -> Vec<&str> {
    if s.is_empty() {
        return Vec::new();
    }
    let is_one_token = s.chars().next().is_some_and(|c| c.len_utf8() == s.len())
        || starts_with_modifier_prefix(s)
        || is_named_key(s);
    if is_one_token {
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
    fn helix_string_keeps_every_modifier_and_named_key() {
        for (input, expected) in [
            ("h", "h"),
            ("G", "S-G"),
            ("A-c", "A-c"),
            ("C-A-x", "A-C-x"),
            ("S-Left", "S-Left"),
            ("Escape", "Escape"),
            ("F5", "F5"),
            ("PageDown", "PageDown"),
            ("space", " "),
        ] {
            assert_eq!(
                PhysicalKey::try_from(input).unwrap().to_helix_string(),
                expected,
                "{input}"
            );
        }
    }

    fn keys(text: &'static str) -> CanonicalKeys {
        CanonicalKeys::parse(text).unwrap()
    }

    #[test]
    fn tokens_examples() {
        assert_eq!(keys("gg").tokens(), vec!["g", "g"]);
        assert_eq!(keys("mi(").tokens(), vec!["m", "i", "("]);
        assert_eq!(keys("Alt-*").tokens(), vec!["Alt-*"]);
        assert_eq!(keys("Ctrl-b").tokens(), vec!["Ctrl-b"]);
        assert_eq!(keys("f").tokens(), vec!["f"]);
        assert_eq!(keys("Escape").tokens(), vec!["Escape"]);
    }

    /// Total-function gate (R2): every one of the registry's real key
    /// strings must parse, so the tokenizer and the parser can never
    /// silently drift apart.
    #[test]
    fn parses_every_registry_key_and_command_constant() {
        let registry = normal_registry();
        let mut checked = 0;
        for meta in registry.all_commands() {
            checked += 1;
            let parsed = CanonicalKeys::parse(meta.key())
                .unwrap_or_else(|error| panic!("registry key {:?}: {error}", meta.key()));
            assert_eq!(
                parsed.events().len(),
                parsed.tokens().len(),
                "{:?}",
                meta.key()
            );
        }
        assert!(checked >= 84);
    }

    #[test]
    fn single_and_multi_key_sequences_are_told_apart() {
        assert!(keys("f").is_single_token());
        assert!(keys("Alt-*").is_single_token());
        assert!(keys("é").is_single_token());
        assert!(!keys("gg").is_single_token());
        assert_eq!(keys("mi(").events().len(), 3);
    }

    #[test]
    fn parse_rejects_empty_and_unparsable_text() {
        assert!(CanonicalKeys::parse("").is_err());
        assert!(CanonicalKeys::parse("Alt-").is_err());
    }

    #[test]
    fn equality_and_hash_follow_the_source_text() {
        use std::hash::{BuildHasher, RandomState};
        let hasher = RandomState::new();
        assert_eq!(keys("Escape"), keys("Escape"));
        assert_eq!(hasher.hash_one(keys("gg")), hasher.hash_one(keys("gg")));
        // Same event, different spelling: still two distinct canonical texts.
        assert_ne!(keys("Escape"), keys("esc"));
    }

    #[test]
    fn from_event_matches_the_stock_command_string() {
        let key = |code, modifiers| CanonicalKeys::from_event(KeyEvent::new(code, modifiers));
        assert_eq!(key(KeyCode::Char('x'), KeyModifiers::NONE), Some(keys("x")));
        assert_eq!(
            key(KeyCode::Char('G'), KeyModifiers::SHIFT),
            Some(keys("G"))
        );
        assert_eq!(key(KeyCode::Enter, KeyModifiers::NONE), Some(keys("Enter")));
        assert_eq!(
            key(KeyCode::Char('d'), KeyModifiers::CONTROL),
            Some(keys("Ctrl-d"))
        );
        assert_eq!(key(KeyCode::F(5), KeyModifiers::NONE), None);
        assert_eq!(key(KeyCode::Char('é'), KeyModifiers::NONE), None);
    }

    #[test]
    fn gameplay_key_wraps_canonical_keys() {
        assert_eq!(GameplayKey::from(keys("x")), GameplayKey::Keys(keys("x")));
    }
}
