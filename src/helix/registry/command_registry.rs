//! Command registry with O(1) lookup and introspection
//!
//! Provides a central registry for all commands with HashMap-based
//! dispatch and rich metadata for documentation.

use std::collections::HashMap;
use std::marker::PhantomData;

use crate::helix::simulator::{EditorMode, HelixSimulator};
use crate::security::UserError;

use super::keytrie::KeyTrie;
use super::metadata::{Category, CommandMetadata, ModeTransition};

/// Check if a key string represents a modifier key command (Alt-*, Ctrl-*)
fn is_modifier_key(key: &str) -> bool {
    key.starts_with("Alt-") || key.starts_with("Ctrl-")
}

/// Command handler function type
pub type CommandHandler<M> = fn(&mut HelixSimulator<M>) -> Result<(), UserError>;

/// A registered command with metadata and handler
pub struct Command<M: EditorMode> {
    /// Command metadata for documentation
    pub metadata: CommandMetadata,
    /// Handler function
    pub handler: CommandHandler<M>,
    /// Phantom data for mode type
    _mode: PhantomData<M>,
}

impl<M: EditorMode> Command<M> {
    /// Create a new command
    pub fn new(metadata: CommandMetadata, handler: CommandHandler<M>) -> Self {
        Self {
            metadata,
            handler,
            _mode: PhantomData,
        }
    }
}

/// Central command registry with O(1) lookup
///
/// Stores all commands indexed by their key sequence for fast dispatch.
/// Also maintains category groupings for introspection.
pub struct CommandRegistry<M: EditorMode> {
    /// Commands indexed by key (e.g., "h", "gg", "fx")
    commands: HashMap<&'static str, Command<M>>,
    /// Commands grouped by category
    by_category: HashMap<Category, Vec<&'static str>>,
    /// Reverse index: real Helix 25.07.1 command name -> canonical key
    /// sequence that triggers it. Excludes `alias_only` entries, which have
    /// no corresponding upstream Helix command name to index by.
    by_name: HashMap<&'static str, &'static str>,
    /// KeyTrie for multi-key resolution
    key_trie: KeyTrie,
    /// Mode marker
    _mode: PhantomData<M>,
}

impl<M: EditorMode> CommandRegistry<M> {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            by_category: HashMap::new(),
            by_name: HashMap::new(),
            key_trie: KeyTrie::new(),
            _mode: PhantomData,
        }
    }

    /// Register a command
    pub fn register(&mut self, cmd: Command<M>) {
        let key = cmd.metadata.key;
        let category = cmd.metadata.category;

        // Add to category grouping
        self.by_category.entry(category).or_default().push(key);

        // Add to the name -> key reverse index, unless this is a
        // trainer-internal convenience binding with no real Helix name.
        if !cmd.metadata.alias_only {
            self.by_name.insert(cmd.metadata.name, key);
        }

        // Register in KeyTrie for single-key commands (including modifier keys)
        // Modifier keys like "Alt-x", "Ctrl-c" are considered single commands
        if key.len() == 1 || is_modifier_key(key) {
            self.key_trie.register_single(key);
        }

        // Store command
        self.commands.insert(key, cmd);
    }

    /// Look up the canonical key sequence for a real Helix 25.07.1 command name.
    ///
    /// Returns `None` for unknown names and for names that only exist as
    /// `alias_only` trainer bindings, which have no upstream Helix command
    /// name to resolve.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::registry::normal_registry;
    ///
    /// let registry = normal_registry();
    /// assert_eq!(registry.key_for_name("goto_last_line"), Some("ge"));
    /// assert_eq!(registry.key_for_name("goto_first_nonblank"), None); // alias_only
    /// ```
    pub fn key_for_name(&self, name: &str) -> Option<&'static str> {
        self.by_name.get(name).copied()
    }

    /// Iterate the `helix_name -> key` reverse index (excludes `alias_only` entries).
    pub fn name_index(&self) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
        self.by_name.iter().map(|(&name, &key)| (name, key))
    }

    /// Execute a command by key (O(1) lookup)
    ///
    /// # Returns
    /// * `Ok(Some(transition))` - Command executed, may trigger mode transition
    /// * `Ok(None)` - Command executed, no mode transition
    /// * `Err(_)` - Command not found or execution failed
    pub fn execute(
        &self,
        sim: &mut HelixSimulator<M>,
        key: &str,
    ) -> Result<Option<ModeTransition>, UserError> {
        if let Some(cmd) = self.commands.get(key) {
            (cmd.handler)(sim)?;
            Ok(cmd.metadata.mode_change)
        } else {
            Err(UserError::command_failed(format!(
                "unknown command '{}'",
                key
            )))
        }
    }

    /// Check if a command exists
    pub fn contains(&self, key: &str) -> bool {
        self.commands.contains_key(key)
    }

    /// Get command metadata
    pub fn metadata(&self, key: &str) -> Option<&CommandMetadata> {
        self.commands.get(key).map(|c| &c.metadata)
    }

    /// Get all command metadata
    pub fn all_commands(&self) -> impl Iterator<Item = &CommandMetadata> {
        self.commands.values().map(|c| &c.metadata)
    }

    /// Get commands in a category
    pub fn commands_in_category(&self, category: Category) -> &[&'static str] {
        self.by_category
            .get(&category)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a command is repeatable
    pub fn is_repeatable(&self, key: &str) -> bool {
        self.commands
            .get(key)
            .map(|c| c.metadata.repeatable)
            .unwrap_or(false)
    }

    /// Get the KeyTrie for multi-key resolution
    pub fn key_trie(&self) -> &KeyTrie {
        &self.key_trie
    }

    /// Get total command count
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl<M: EditorMode> Default for CommandRegistry<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helix::simulator::NormalMode;

    fn create_test_command() -> Command<NormalMode> {
        Command::new(
            CommandMetadata::new(
                "test_move",
                "h",
                "Test move",
                "Test move help",
                Category::Movement,
                false,
                None,
            ),
            |_sim| Ok(()),
        )
    }

    #[test]
    fn test_registry_creation() {
        let registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_command_registration() {
        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        registry.register(create_test_command());

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("h"));
    }

    #[test]
    fn test_metadata() {
        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        registry.register(create_test_command());

        let meta = registry.metadata("h").unwrap();
        assert_eq!(meta.name, "test_move");
        assert_eq!(meta.key, "h");
        assert_eq!(meta.category, Category::Movement);
    }

    #[test]
    fn test_category_grouping() {
        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        registry.register(create_test_command());
        registry.register(Command::new(
            CommandMetadata::new(
                "test_move2",
                "l",
                "Test move 2",
                "Help",
                Category::Movement,
                false,
                None,
            ),
            |_sim| Ok(()),
        ));
        registry.register(Command::new(
            CommandMetadata::new(
                "test_edit",
                "d",
                "Test edit",
                "Help",
                Category::Editing,
                true,
                None,
            ),
            |_sim| Ok(()),
        ));

        let movement = registry.commands_in_category(Category::Movement);
        assert_eq!(movement.len(), 2);
        assert!(movement.contains(&"h"));
        assert!(movement.contains(&"l"));

        let editing = registry.commands_in_category(Category::Editing);
        assert_eq!(editing.len(), 1);
        assert!(editing.contains(&"d"));
    }

    #[test]
    fn test_is_repeatable() {
        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        registry.register(create_test_command()); // Not repeatable
        registry.register(Command::new(
            CommandMetadata::new(
                "test_edit",
                "d",
                "Test edit",
                "Help",
                Category::Editing,
                true, // Repeatable
                None,
            ),
            |_sim| Ok(()),
        ));

        assert!(!registry.is_repeatable("h"));
        assert!(registry.is_repeatable("d"));
        assert!(!registry.is_repeatable("unknown"));
    }

    #[test]
    fn test_all_commands_iterator() {
        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        registry.register(create_test_command());
        registry.register(Command::new(
            CommandMetadata::new(
                "test2",
                "j",
                "Test 2",
                "Help",
                Category::Movement,
                false,
                None,
            ),
            |_sim| Ok(()),
        ));

        let all: Vec<_> = registry.all_commands().collect();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_nonexistent_command() {
        let registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        assert!(!registry.contains("xyz"));
        assert!(registry.metadata("xyz").is_none());
    }

    /// Guards the `helix_name -> key` reverse index used to resolve a
    /// user's Helix keymap config by command name: it must be **total**
    /// (every non-alias command's name resolves back to its own key) and
    /// **injective** (no two distinct non-alias commands share a `name`,
    /// which would make resolution ambiguous). A collision here is a bug
    /// in the registry's `CommandMetadata::name` values, not something a
    /// tie-break rule should paper over — this test is meant to fail the
    /// build.
    #[test]
    fn reverse_index_is_total_and_injective_over_non_alias_names() {
        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        crate::helix::registry::definitions::register_all(&mut registry);

        let non_alias: Vec<_> = registry.all_commands().filter(|m| !m.alias_only).collect();
        assert!(!non_alias.is_empty());

        // Injective: the index has exactly one entry per non-alias command.
        assert_eq!(
            registry.name_index().count(),
            non_alias.len(),
            "reverse index is not injective: two non-alias commands share a `name`"
        );

        // Total: every non-alias command's name resolves back to its own key.
        for meta in &non_alias {
            assert_eq!(
                registry.key_for_name(meta.name),
                Some(meta.key),
                "reverse index missing or wrong entry for `{}`",
                meta.name
            );
        }
    }

    /// Every non-alias `CommandMetadata.name` must be a real upstream Helix
    /// 25.07.1 command name, checked against the reference list scraped
    /// from `helix-term/src/commands.rs`'s `static_commands!` invocation.
    /// `alias_only` entries are trainer-internal by definition and are
    /// exempt.
    #[test]
    fn non_alias_names_are_real_helix_commands() {
        let known: std::collections::HashSet<&str> = include_str!("helix-25.07-commands.txt")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        assert!(known.len() > 200, "reference list failed to load");

        let mut registry: CommandRegistry<NormalMode> = CommandRegistry::new();
        crate::helix::registry::definitions::register_all(&mut registry);

        for meta in registry.all_commands().filter(|m| !m.alias_only) {
            assert!(
                known.contains(meta.name),
                "`{}` is not a real Helix 25.07.1 command name",
                meta.name
            );
        }
    }
}
