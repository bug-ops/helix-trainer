//! Command metadata types for introspection and documentation
//!
//! Provides rich metadata for each command including category,
//! documentation, and mode transition information.

use crate::helix::simulator::{FindDirection, FindType};

/// Command category for grouping and documentation of registry commands
///
/// Independent of [`ScenarioCategory`](crate::config::ScenarioCategory): that
/// enum classifies scenarios for the menu, this one classifies commands for
/// the help screen. The names overlap by coincidence and there is no mapping
/// between the two.
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

/// Whether the `.` command replays a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    /// `.` replays the command (editing commands such as `d`, `p`, `>`).
    Repeatable,
    /// `.` skips the command (movement, selection and mode-exit commands).
    #[default]
    NotRepeatable,
}

/// The name Helix 25.07.1 gives a command in its static command list
/// (for example `move_char_left` for `h`).
///
/// Only upstream names are valid here: the registry resolves a user's Helix
/// keymap configuration by this name, and a name that does not exist upstream
/// must never be reachable that way. Trainer-only bindings use
/// [`Binding::TrainerAlias`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HelixName(&'static str);

impl HelixName {
    /// Wrap an upstream command name.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::registry::metadata::HelixName;
    ///
    /// assert_eq!(HelixName::new("move_char_left").as_str(), "move_char_left");
    /// ```
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The upstream command name.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

/// How a registered key sequence relates to Helix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// A real Helix 25.07.1 command, resolvable by its upstream name.
    Helix(HelixName),
    /// A trainer-internal convenience binding with no upstream command name
    /// (for example `^` or `Ctrl-r`). Excluded from the
    /// `helix_name -> CanonicalKeys` reverse index of
    /// [`CommandRegistry`](super::command_registry::CommandRegistry).
    TrainerAlias,
}

/// Rich metadata for a command
///
/// Contains all documentation and categorization information
/// for a single command. Used for help screens and hint generation.
///
/// Built with [`CommandMetadata::new`] and the `with_*` builders; the summary
/// and help text default to empty and are expected on every registered
/// command (a registry test enforces it).
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::registry::metadata::{
///     Binding, Category, CommandMetadata, HelixName, Repeat,
/// };
///
/// let meta = CommandMetadata::new(
///     Binding::Helix(HelixName::new("delete_selection")),
///     "d",
///     Category::Editing,
/// )
/// .with_summary("Delete selection")
/// .with_help("Delete the current selection.")
/// .with_repeat(Repeat::Repeatable);
///
/// assert_eq!(meta.key(), "d");
/// assert_eq!(meta.repeat(), Repeat::Repeatable);
/// ```
#[derive(Debug, Clone)]
pub struct CommandMetadata {
    binding: Binding,
    key: &'static str,
    summary: &'static str,
    help: &'static str,
    category: Category,
    repeat: Repeat,
    mode_change: Option<ModeTransition>,
    count_behavior: CountBehavior,
}

impl CommandMetadata {
    /// Start metadata for `key`, bound as `binding`, in `category`.
    ///
    /// Every other property defaults to the most common case: no text,
    /// [`Repeat::NotRepeatable`], no mode transition, [`CountBehavior::Repeat`].
    pub const fn new(binding: Binding, key: &'static str, category: Category) -> Self {
        Self {
            binding,
            key,
            summary: "",
            help: "",
            category,
            repeat: Repeat::NotRepeatable,
            mode_change: None,
            count_behavior: CountBehavior::Repeat,
        }
    }

    /// Set the short description shown in hints (e.g. "Move cursor left").
    #[must_use]
    pub const fn with_summary(mut self, summary: &'static str) -> Self {
        self.summary = summary;
        self
    }

    /// Set the detailed help text for the help screen.
    #[must_use]
    pub const fn with_help(mut self, help: &'static str) -> Self {
        self.help = help;
        self
    }

    /// Set whether `.` replays this command.
    #[must_use]
    pub const fn with_repeat(mut self, repeat: Repeat) -> Self {
        self.repeat = repeat;
        self
    }

    /// Set the mode transition this command triggers.
    #[must_use]
    pub const fn with_mode_change(mut self, mode_change: ModeTransition) -> Self {
        self.mode_change = Some(mode_change);
        self
    }

    /// Override how a count prefix applies to this command.
    #[must_use]
    pub const fn with_count_behavior(mut self, count_behavior: CountBehavior) -> Self {
        self.count_behavior = count_behavior;
        self
    }

    /// How the key relates to Helix.
    pub const fn binding(&self) -> Binding {
        self.binding
    }

    /// The upstream Helix command name, or `None` for a trainer alias.
    pub const fn name(&self) -> Option<HelixName> {
        match self.binding {
            Binding::Helix(name) => Some(name),
            Binding::TrainerAlias => None,
        }
    }

    /// Key sequence that triggers this command (e.g. "h", "gg", "fx").
    pub const fn key(&self) -> &'static str {
        self.key
    }

    /// Short description for hints.
    pub const fn summary(&self) -> &'static str {
        self.summary
    }

    /// Detailed help text for the help screen.
    pub const fn help(&self) -> &'static str {
        self.help
    }

    /// Command category.
    pub const fn category(&self) -> Category {
        self.category
    }

    /// Whether `.` replays this command.
    pub const fn repeat(&self) -> Repeat {
        self.repeat
    }

    /// Mode transition triggered by this command.
    pub const fn mode_change(&self) -> Option<ModeTransition> {
        self.mode_change
    }

    /// How a count prefix applies to this command.
    pub const fn count_behavior(&self) -> CountBehavior {
        self.count_behavior
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
    fn test_metadata_defaults() {
        let meta = CommandMetadata::new(
            Binding::Helix(HelixName::new("move_left")),
            "h",
            Category::Movement,
        )
        .with_summary("Move cursor left")
        .with_help("Move the cursor one character to the left.");

        assert_eq!(meta.name().map(HelixName::as_str), Some("move_left"));
        assert_eq!(meta.key(), "h");
        assert_eq!(meta.summary(), "Move cursor left");
        assert_eq!(meta.category(), Category::Movement);
        assert_eq!(meta.repeat(), Repeat::NotRepeatable);
        assert!(meta.mode_change().is_none());
        assert_eq!(meta.count_behavior(), CountBehavior::Repeat);
    }

    #[test]
    fn test_insert_command_metadata() {
        let meta = CommandMetadata::new(
            Binding::Helix(HelixName::new("insert")),
            "i",
            Category::ModeChange,
        )
        .with_repeat(Repeat::Repeatable)
        .with_mode_change(ModeTransition::ToInsert);

        assert_eq!(meta.repeat(), Repeat::Repeatable);
        assert_eq!(meta.mode_change(), Some(ModeTransition::ToInsert));
    }

    #[test]
    fn test_trainer_alias_has_no_helix_name() {
        let meta = CommandMetadata::new(Binding::TrainerAlias, "^", Category::Movement);

        assert_eq!(meta.binding(), Binding::TrainerAlias);
        assert_eq!(meta.name(), None);
    }
}
