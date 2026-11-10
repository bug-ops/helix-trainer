//! Command metadata types for introspection and documentation
//!
//! Provides rich metadata for each command including category,
//! documentation, and mode transition information.

use crate::helix::simulator::{FindDirection, FindType};

/// Command category for grouping and documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Movement commands (h, j, k, l, w, b, e, etc.)
    Movement,
    /// Editing commands (d, c, J, etc.)
    Editing,
    /// Selection commands (x, X, %, ;, v, C, K, etc.). `s`/`S` (regex
    /// select/split) are conceptually part of this category but are not
    /// registered here - they take a pattern argument the registry's
    /// fixed `CommandHandler` signature can't carry, so they're dispatched
    /// directly in `execute_normal_mode_command_internal` instead.
    Selection,
    /// Clipboard commands (y, p, P)
    Clipboard,
    /// Mode change commands (i, a, I, A, o, O)
    ModeChange,
    /// Undo/redo commands (u, U)
    Undo,
    /// Search commands (/, ?, n, N, *, Alt-*)
    Search,
    /// View/viewport commands (z, zt, zb, zm, zj, zk)
    View,
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
            Category::Search => "Search",
            Category::View => "View",
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

/// How a numeric count prefix (`3x`, `2G`, `3fa`) applies to a command.
///
/// The simulator is the only place a count is interpreted: it consults this
/// value to decide whether to repeat the command, run it once, or hand the
/// count to the command itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CountBehavior {
    /// Apply the command `count` times (`h`, `j`, `x`, `d`, ...).
    #[default]
    Repeat,
    /// Apply the command once regardless of the count (`J`).
    Ignore,
    /// Pass the count into the command (`G`, `gg`, `f`, `t`, `F`, `T`).
    Native,
}

/// A `Native` counted command that is dispatched outside the registry
/// because it carries an argument or a count of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCounted {
    /// `{n}G` / `{n}gg`: go to line `n`.
    GotoLine,
    /// `{n}f<c>`, `{n}t<c>`, `{n}F<c>`, `{n}T<c>`: the `n`th match of `ch`.
    Find {
        /// Find or till.
        find_type: FindType,
        /// Search direction.
        direction: FindDirection,
        /// Character to search for.
        ch: char,
    },
}

impl NativeCounted {
    /// Parse a command string without its count prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::registry::NativeCounted;
    ///
    /// assert_eq!(NativeCounted::parse("G"), Some(NativeCounted::GotoLine));
    /// assert!(NativeCounted::parse("fa").is_some());
    /// assert!(NativeCounted::parse("x").is_none());
    /// ```
    pub fn parse(base: &str) -> Option<Self> {
        let mut chars = base.chars();
        let find = |find_type, direction, ch| Self::Find {
            find_type,
            direction,
            ch,
        };
        match (chars.next()?, chars.next(), chars.next()) {
            ('G', None, _) | ('g', Some('g'), None) => Some(Self::GotoLine),
            ('f', Some(ch), None) => Some(find(FindType::Find, FindDirection::Forward, ch)),
            ('F', Some(ch), None) => Some(find(FindType::Find, FindDirection::Backward, ch)),
            ('t', Some(ch), None) => Some(find(FindType::Till, FindDirection::Forward, ch)),
            ('T', Some(ch), None) => Some(find(FindType::Till, FindDirection::Backward, ch)),
            _ => None,
        }
    }
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
    /// Whether `name` is a trainer-internal convenience alias rather than a
    /// real Helix 25.07.1 command name.
    ///
    /// Excluded from the `helix_name -> CanonicalKeys` reverse index built by
    /// [`CommandRegistry`](super::command_registry::CommandRegistry), since
    /// resolving a user's Helix keymap config by command name must never
    /// land on a binding that doesn't actually exist upstream.
    pub alias_only: bool,
    /// How a count prefix applies to this command
    pub count_behavior: CountBehavior,
}

impl CommandMetadata {
    /// Create new command metadata for a real Helix 25.07.1 command
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
            alias_only: false,
            count_behavior: CountBehavior::Repeat,
        }
    }

    /// Override how a count prefix applies to this command.
    #[must_use]
    pub const fn with_count_behavior(mut self, count_behavior: CountBehavior) -> Self {
        self.count_behavior = count_behavior;
        self
    }

    /// Create metadata for a trainer-internal convenience binding that has
    /// no corresponding upstream Helix command name (e.g. `^` or `Ctrl-r`).
    ///
    /// Marks [`alias_only`](Self::alias_only) so the binding is excluded
    /// from the `helix_name -> CanonicalKeys` reverse index.
    pub const fn new_alias(
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
            alias_only: true,
            count_behavior: CountBehavior::Repeat,
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
        assert_eq!(Category::Search.display_name(), "Search");
        assert_eq!(Category::View.display_name(), "View");
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
