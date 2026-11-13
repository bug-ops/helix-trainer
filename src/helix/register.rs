//! Typed register addressing for the Helix simulator
//!
//! A register is the character typed after `"`. [`Register`] parses that
//! character once, at the input boundary, into a closed set: the unnamed
//! default, the blackhole, the read-only registers Helix computes from the
//! editor (`%`, `#`, `.`) and every other general-purpose named register.
//! Downstream code matches on the case instead of comparing characters, so
//! "no register given" and `""` are one value and a write to a read-only
//! register is rejected by [`Register::write_target`] before anything changes.

use std::fmt;

use super::error::SimulatorError;

/// Display name of the buffer Helix opens without a file argument; what `%` reads.
pub(crate) const SCRATCH_BUFFER_NAME: &str = "[scratch]";

/// A general-purpose register (any character that names no special register).
///
/// Only [`Register::from`] builds one, so a named register can never alias the
/// unnamed, blackhole or read-only registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NamedRegister(char);

impl NamedRegister {
    /// The character that names the register.
    #[must_use]
    pub fn name(self) -> char {
        self.0
    }
}

/// A register Helix computes from the editor on read and rejects writes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadOnlyRegister {
    /// `%`: the display name of the current buffer.
    FileName,
    /// `#`: the 1-based index of each selection range.
    SelectionIndices,
    /// `.`: the text of each selection range.
    SelectionContents,
}

impl ReadOnlyRegister {
    /// The character that names the register.
    #[must_use]
    pub fn name(self) -> char {
        match self {
            Self::FileName => '%',
            Self::SelectionIndices => '#',
            Self::SelectionContents => '.',
        }
    }
}

impl fmt::Display for ReadOnlyRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A register addressed by `"<char>`, or none given.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::{Register, ReadOnlyRegister};
///
/// assert_eq!(Register::from('"'), Register::Unnamed);
/// assert_eq!(Register::from('_'), Register::Blackhole);
/// assert_eq!(Register::from('%'), Register::ReadOnly(ReadOnlyRegister::FileName));
/// assert_eq!(Register::from('a').to_string(), "a");
/// assert!(Register::from('%').write_target().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    /// `"`, and the register of every command typed without a prefix.
    Unnamed,
    /// `_`: writes are discarded and reads find nothing.
    Blackhole,
    /// `%`, `#` or `.`: computed on read, never stored.
    ReadOnly(ReadOnlyRegister),
    /// Any other character.
    Named(NamedRegister),
}

impl From<char> for Register {
    fn from(name: char) -> Self {
        match name {
            '"' => Self::Unnamed,
            '_' => Self::Blackhole,
            '%' => Self::ReadOnly(ReadOnlyRegister::FileName),
            '#' => Self::ReadOnly(ReadOnlyRegister::SelectionIndices),
            '.' => Self::ReadOnly(ReadOnlyRegister::SelectionContents),
            other => Self::Named(NamedRegister(other)),
        }
    }
}

impl Register {
    /// The character that addresses the register after `"`.
    #[must_use]
    pub fn name(self) -> char {
        match self {
            Self::Unnamed => '"',
            Self::Blackhole => '_',
            Self::ReadOnly(register) => register.name(),
            Self::Named(register) => register.name(),
        }
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A register a command may write to: [`Register`] without the read-only cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteTarget {
    /// The unnamed register.
    Unnamed,
    /// A named register.
    Named(NamedRegister),
    /// The blackhole: the write is discarded.
    Blackhole,
}

impl Register {
    /// The register as a write destination.
    ///
    /// # Errors
    ///
    /// [`SimulatorError::ReadOnlyRegister`] for `%`, `#` and `.`. Callers
    /// resolve it before changing anything, so a rejected write leaves the
    /// document, selection and mode as they were.
    pub fn write_target(self) -> Result<WriteTarget, SimulatorError> {
        match self {
            Self::Unnamed => Ok(WriteTarget::Unnamed),
            Self::Blackhole => Ok(WriteTarget::Blackhole),
            Self::Named(register) => Ok(WriteTarget::Named(register)),
            Self::ReadOnly(register) => Err(SimulatorError::ReadOnlyRegister { register }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_prints_the_typed_character() {
        for name in ['"', '_', '%', '#', '.', 'a', 'Z', '*', 'é'] {
            assert_eq!(Register::from(name).to_string(), name.to_string());
        }
    }

    #[test]
    fn only_read_only_registers_reject_writes() {
        for (name, expected) in [
            ('%', ReadOnlyRegister::FileName),
            ('#', ReadOnlyRegister::SelectionIndices),
            ('.', ReadOnlyRegister::SelectionContents),
        ] {
            assert!(matches!(
                Register::from(name).write_target(),
                Err(SimulatorError::ReadOnlyRegister { register }) if register == expected
            ));
        }
        for name in ['"', '_', 'a', '*', '/'] {
            assert!(Register::from(name).write_target().is_ok());
        }
    }
}
