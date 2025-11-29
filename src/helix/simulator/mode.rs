//! Editor mode typestate markers
//!
//! This module provides zero-cost type-level markers for editor modes,
//! enabling compile-time enforcement of mode-specific operations.
//!
//! # Type Safety
//!
//! The typestate pattern ensures that mode-specific commands are only
//! available in the correct mode:
//!
//! ```ignore
//! // This compiles:
//! let sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
//! sim.delete_char();  // OK - delete is only available in Normal mode
//!
//! // This does NOT compile:
//! let sim: HelixSimulator<InsertMode> = sim.enter_insert_mode();
//! sim.delete_char();  // ERROR - no method `delete_char` for InsertMode
//! ```

/// Normal mode marker (zero-sized type)
///
/// Represents the editor in Normal mode where commands are executed.
/// This is a zero-sized type (ZST) that exists only at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalMode;

/// Insert mode marker (zero-sized type)
///
/// Represents the editor in Insert mode where text is being inserted.
/// This is a zero-sized type (ZST) that exists only at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertMode;

/// Private module for sealing the EditorMode trait
///
/// This prevents external crates from implementing EditorMode,
/// allowing us to add methods in the future without breaking changes.
mod private {
    pub trait Sealed {}
}

/// Sealed trait for editor modes
///
/// This trait can only be implemented by types in this module,
/// ensuring all possible modes are known at compile time.
pub trait EditorMode: private::Sealed {
    /// Get the display name of this mode
    fn name() -> &'static str;
}

// Implement sealed trait for our mode markers
impl private::Sealed for NormalMode {}
impl private::Sealed for InsertMode {}

impl EditorMode for NormalMode {
    fn name() -> &'static str {
        "NORMAL"
    }
}

impl EditorMode for InsertMode {
    fn name() -> &'static str {
        "INSERT"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::marker::PhantomData;

    #[test]
    fn test_mode_names() {
        assert_eq!(NormalMode::name(), "NORMAL");
        assert_eq!(InsertMode::name(), "INSERT");
    }

    #[test]
    fn test_mode_markers_are_zero_sized() {
        // PhantomData is ZST, so our modes should be too
        assert_eq!(std::mem::size_of::<NormalMode>(), 0);
        assert_eq!(std::mem::size_of::<InsertMode>(), 0);
        assert_eq!(std::mem::size_of::<PhantomData<NormalMode>>(), 0);
        assert_eq!(std::mem::size_of::<PhantomData<InsertMode>>(), 0);
    }

    #[test]
    fn test_mode_equality() {
        assert_eq!(NormalMode, NormalMode);
        assert_eq!(InsertMode, InsertMode);
        // Note: Cannot compare NormalMode != InsertMode because they're different types
    }

    #[test]
    fn test_mode_clone() {
        let normal = NormalMode;
        let normal2 = normal;
        assert_eq!(normal, normal2);

        let insert = InsertMode;
        let insert2 = insert;
        assert_eq!(insert, insert2);
    }

    #[test]
    fn test_mode_debug() {
        assert_eq!(format!("{:?}", NormalMode), "NormalMode");
        assert_eq!(format!("{:?}", InsertMode), "InsertMode");
    }
}
