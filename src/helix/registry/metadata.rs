//! Command metadata types for introspection and documentation
//!
//! Provides rich metadata for each command including category,
//! documentation, and mode transition information.

/// Command category for grouping and documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Movement commands (h, j, k, l, w, b, e, etc.)
    Movement,
    /// Editing commands (d, c, J, etc.)
    Editing,
    /// Selection commands (x, X, %, ;, v)
    Selection,
    /// Clipboard commands (y, p, P)
    Clipboard,
    /// Mode change commands (i, a, I, A, o, O)
    ModeChange,
    /// Undo/redo commands (u, U)
    Undo,
}

impl Category {
    /// Get display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            Category::Movement => "Movement",
            Category::Editing => "Editing",
            Category::Selection => "Selection",
            Category::Clipboard => "Clipboard",
            Category::ModeChange => "Mode Change",
            Category::Undo => "Undo/Redo",
        }
    }
}

/// Mode transition information for insert mode entry commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeTransition {
    /// Transition to insert mode (i, a, I, A, o, O, c)
    ToInsert,
    /// Transition to normal mode (Escape)
    ToNormal,
}

/// Rich metadata for a command
///
/// Contains all documentation and categorization information
/// for a single command. Used for help screens and hint generation.
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    /// Internal name (e.g., "move_left")
    pub name: &'static str,
    /// Key sequence that triggers this command (e.g., "h", "gg", "fx")
    pub key: &'static str,
    /// Short description for hints (e.g., "Move cursor left")
    pub description: &'static str,
    /// Detailed help text for help screen
    pub help: &'static str,
    /// Command category
    pub category: Category,
    /// Whether this command can be repeated with '.'
    pub repeatable: bool,
    /// Mode transition triggered by this command
    pub mode_change: Option<ModeTransition>,
}

impl CommandMetadata {
    /// Create new command metadata
    pub const fn new(
        name: &'static str,
        key: &'static str,
        description: &'static str,
        help: &'static str,
        category: Category,
        repeatable: bool,
        mode_change: Option<ModeTransition>,
    ) -> Self {
        Self {
            name,
            key,
            description,
            help,
            category,
            repeatable,
            mode_change,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_display_name() {
        assert_eq!(Category::Movement.display_name(), "Movement");
        assert_eq!(Category::Editing.display_name(), "Editing");
        assert_eq!(Category::Selection.display_name(), "Selection");
        assert_eq!(Category::Clipboard.display_name(), "Clipboard");
        assert_eq!(Category::ModeChange.display_name(), "Mode Change");
        assert_eq!(Category::Undo.display_name(), "Undo/Redo");
    }

    #[test]
    fn test_metadata_creation() {
        let meta = CommandMetadata::new(
            "move_left",
            "h",
            "Move cursor left",
            "Move the cursor one character to the left.",
            Category::Movement,
            false,
            None,
        );

        assert_eq!(meta.name, "move_left");
        assert_eq!(meta.key, "h");
        assert_eq!(meta.category, Category::Movement);
        assert!(!meta.repeatable);
        assert!(meta.mode_change.is_none());
    }

    #[test]
    fn test_insert_command_metadata() {
        let meta = CommandMetadata::new(
            "insert",
            "i",
            "Enter insert mode",
            "Enter insert mode at cursor position.",
            Category::ModeChange,
            true,
            Some(ModeTransition::ToInsert),
        );

        assert!(meta.repeatable);
        assert_eq!(meta.mode_change, Some(ModeTransition::ToInsert));
    }
}
