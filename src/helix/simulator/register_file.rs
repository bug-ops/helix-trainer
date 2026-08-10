//! Named register storage for yank/paste/replace operations
//!
//! Mirrors Helix's register model: every clipboard-affecting command reads
//! and writes a named register, and the *unnamed* register (bound to `'"'`,
//! matching Helix's own default) is what plain `y`/`p`/`P`/`R` use when no
//! `"<reg>` prefix is given. Keeping a single storage type with one lookup
//! key means `""y` and `y` are the same operation by construction, rather
//! than two code paths that could drift.

use std::collections::HashMap;

/// Register key used when no explicit register is selected.
///
/// Matches Helix, where the unnamed register is itself addressable as `"`
/// (so `""y` and `y` are equivalent).
pub const UNNAMED_REGISTER: char = '"';

/// The blackhole register: writes addressed to it are discarded.
///
/// Matches Helix's `"_`, used to delete/change text without touching any
/// register (e.g. `"_d`, `"_c`).
pub const BLACKHOLE_REGISTER: char = '_';

/// Storage for all named registers, including the unnamed default register.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::simulator::RegisterFile;
///
/// let mut registers = RegisterFile::new();
/// registers.set(None, "hello".to_string());
/// registers.set(Some('a'), "world".to_string());
///
/// assert_eq!(registers.get(None), Some("hello"));
/// assert_eq!(registers.get(Some('a')), Some("world"));
/// assert_eq!(registers.get(Some('b')), None);
/// ```
#[derive(Debug, Clone, Default)]
pub struct RegisterFile {
    registers: HashMap<char, String>,
}

impl RegisterFile {
    /// Create an empty register file (no register has been written to yet).
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
        }
    }

    /// Read the contents of a register.
    ///
    /// `None` addresses the unnamed register, so `get(None)` is what plain
    /// `p`/`P`/`R` read from.
    pub fn get(&self, register: Option<char>) -> Option<&str> {
        let key = register.unwrap_or(UNNAMED_REGISTER);
        self.registers.get(&key).map(String::as_str)
    }

    /// Write content into a register, overwriting any previous content.
    ///
    /// `None` addresses the unnamed register, so `set(None, ...)` is what
    /// plain `y` writes to. Writes to the blackhole register
    /// ([`BLACKHOLE_REGISTER`]) are silently discarded, matching Helix's
    /// `"_`.
    pub fn set(&mut self, register: Option<char>, content: String) {
        let key = register.unwrap_or(UNNAMED_REGISTER);
        if key == BLACKHOLE_REGISTER {
            return;
        }
        self.registers.insert(key, content);
    }

    /// Remove a register's content entirely.
    pub fn clear(&mut self, register: char) {
        self.registers.remove(&register);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unnamed_register_roundtrip() {
        let mut registers = RegisterFile::new();
        registers.set(None, "foo".to_string());
        assert_eq!(registers.get(None), Some("foo"));
    }

    #[test]
    fn named_register_is_independent_of_unnamed() {
        let mut registers = RegisterFile::new();
        registers.set(None, "default".to_string());
        registers.set(Some('a'), "named".to_string());

        assert_eq!(registers.get(None), Some("default"));
        assert_eq!(registers.get(Some('a')), Some("named"));
    }

    #[test]
    fn explicit_unnamed_key_matches_none() {
        let mut registers = RegisterFile::new();
        registers.set(Some(UNNAMED_REGISTER), "via-explicit-key".to_string());
        assert_eq!(registers.get(None), Some("via-explicit-key"));
    }

    #[test]
    fn unset_register_is_none() {
        let registers = RegisterFile::new();
        assert_eq!(registers.get(Some('z')), None);
        assert_eq!(registers.get(None), None);
    }

    #[test]
    fn clear_removes_register_content() {
        let mut registers = RegisterFile::new();
        registers.set(Some('a'), "content".to_string());
        registers.clear('a');
        assert_eq!(registers.get(Some('a')), None);
    }

    #[test]
    fn blackhole_register_discards_writes() {
        let mut registers = RegisterFile::new();
        registers.set(Some(BLACKHOLE_REGISTER), "discarded".to_string());
        assert_eq!(registers.get(Some(BLACKHOLE_REGISTER)), None);
    }

    #[test]
    fn blackhole_register_does_not_affect_other_registers() {
        let mut registers = RegisterFile::new();
        registers.set(None, "unnamed".to_string());
        registers.set(Some('a'), "named".to_string());
        registers.set(Some(BLACKHOLE_REGISTER), "discarded".to_string());

        assert_eq!(registers.get(None), Some("unnamed"));
        assert_eq!(registers.get(Some('a')), Some("named"));
        assert_eq!(registers.get(Some(BLACKHOLE_REGISTER)), None);
    }
}
