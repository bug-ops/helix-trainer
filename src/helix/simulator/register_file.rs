//! Register storage for yank/paste/replace operations
//!
//! Mirrors Helix's register model for the registers that hold text: the
//! unnamed register (what plain `y`/`p`/`P`/`R` use, and `""`) and the named
//! ones. Callers address them with a [`Register`], so `""y` and `y` are the
//! same operation by construction.

use std::collections::HashMap;

use crate::helix::register::{Register, WriteTarget};

/// Storage for every writable register, including the unnamed default.
///
/// Read-only registers (`%`, `#`, `.`) are computed from the editor, not
/// stored here, and the blackhole never holds anything: [`WriteTarget`] has no
/// way to name a register that cannot be stored.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::Register;
/// use helix_trainer::helix::register::WriteTarget;
/// use helix_trainer::helix::simulator::RegisterFile;
///
/// let mut registers = RegisterFile::new();
/// registers.set(WriteTarget::Unnamed, "hello".to_string());
/// registers.set(Register::from('a').write_target().unwrap(), "world".to_string());
///
/// assert_eq!(registers.get(Register::Unnamed), Some("hello"));
/// assert_eq!(registers.get(Register::from('a')), Some("world"));
/// assert_eq!(registers.get(Register::from('b')), None);
/// ```
#[derive(Debug, Clone, Default)]
pub struct RegisterFile {
    registers: HashMap<Register, Vec<String>>,
}

impl RegisterFile {
    /// Create an empty register file (no register has been written to yet).
    pub fn new() -> Self {
        Self {
            registers: HashMap::new(),
        }
    }

    /// Read the first stored value of a register.
    ///
    /// Callers that must honor every selection range use [`Self::get_values`].
    pub fn get(&self, register: Register) -> Option<&str> {
        self.get_values(register)
            .and_then(<[String]>::first)
            .map(String::as_str)
    }

    /// Read every stored value of a register, one per yanked selection range.
    ///
    /// The blackhole and the read-only registers hold nothing here; the
    /// simulator computes the latter on read.
    pub fn get_values(&self, register: Register) -> Option<&[String]> {
        self.registers.get(&register).map(Vec::as_slice)
    }

    /// Write a single value into a register, overwriting any previous content.
    ///
    /// See [`Self::set_values`].
    pub fn set(&mut self, target: WriteTarget, content: String) {
        self.set_values(target, vec![content]);
    }

    /// Write one value per yanked selection range, overwriting any previous
    /// content. A write to the blackhole is discarded.
    pub fn set_values(&mut self, target: WriteTarget, values: Vec<String>) {
        let register = match target {
            WriteTarget::Unnamed => Register::Unnamed,
            WriteTarget::Named(named) => Register::Named(named),
            WriteTarget::Blackhole => return,
        };
        self.registers.insert(register, values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(name: char) -> WriteTarget {
        Register::from(name)
            .write_target()
            .expect("writable register")
    }

    #[test]
    fn unnamed_register_roundtrip() {
        let mut registers = RegisterFile::new();
        registers.set(WriteTarget::Unnamed, "foo".to_string());
        assert_eq!(registers.get(Register::Unnamed), Some("foo"));
    }

    #[test]
    fn named_register_is_independent_of_unnamed() {
        let mut registers = RegisterFile::new();
        registers.set(WriteTarget::Unnamed, "default".to_string());
        registers.set(target('a'), "named".to_string());

        assert_eq!(registers.get(Register::Unnamed), Some("default"));
        assert_eq!(registers.get(Register::from('a')), Some("named"));
    }

    #[test]
    fn typed_quote_is_the_unnamed_register() {
        let mut registers = RegisterFile::new();
        registers.set(target('"'), "via-explicit-key".to_string());
        assert_eq!(registers.get(Register::Unnamed), Some("via-explicit-key"));
    }

    #[test]
    fn unset_register_is_none() {
        let registers = RegisterFile::new();
        assert_eq!(registers.get(Register::from('z')), None);
        assert_eq!(registers.get(Register::Unnamed), None);
    }

    #[test]
    fn multi_value_register_keeps_each_value_separate() {
        let mut registers = RegisterFile::new();
        registers.set_values(WriteTarget::Unnamed, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            registers.get_values(Register::Unnamed),
            Some(["a".to_string(), "b".to_string()].as_slice())
        );
        assert_eq!(registers.get(Register::Unnamed), Some("a"));
    }

    #[test]
    fn blackhole_discards_writes_and_reads_nothing() {
        let mut registers = RegisterFile::new();
        registers.set(WriteTarget::Blackhole, "discarded".to_string());
        assert_eq!(registers.get(Register::Blackhole), None);
    }

    #[test]
    fn blackhole_write_does_not_disturb_other_registers() {
        let mut registers = RegisterFile::new();
        registers.set(WriteTarget::Unnamed, "unnamed".to_string());
        registers.set(target('a'), "named".to_string());
        registers.set(WriteTarget::Blackhole, "discarded".to_string());

        assert_eq!(registers.get(Register::Unnamed), Some("unnamed"));
        assert_eq!(registers.get(Register::from('a')), Some("named"));
        assert_eq!(registers.get(Register::Blackhole), None);
    }
}
