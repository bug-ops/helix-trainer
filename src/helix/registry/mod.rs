//! Command Registry Module
//!
//! Provides O(1) command dispatch with rich metadata for introspection.
//!
//! # Architecture
//!
//! ```text
//! execute_command(cmd)
//!        │
//!        ▼
//!    KeyTrie (resolves multi-key sequences)
//!        │
//!        ▼
//!    CommandRegistry (HashMap O(1) lookup)
//!        │
//!        ▼
//!    Handler functions
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use helix_trainer::helix::registry::{NORMAL_REGISTRY, KeyMatch};
//!
//! // Get command metadata
//! if let Some(meta) = NORMAL_REGISTRY.get_metadata("h") {
//!     println!("{}: {}", meta.name, meta.description);
//! }
//!
//! // List all movement commands
//! for key in NORMAL_REGISTRY.commands_in_category(Category::Movement) {
//!     println!("  {}", key);
//! }
//! ```

pub mod command_registry;
pub mod definitions;
pub mod keytrie;
pub mod metadata;

use std::sync::OnceLock;

use crate::helix::simulator::NormalMode;

pub use command_registry::{Command, CommandHandler, CommandRegistry};
pub use keytrie::{KeyMatch, KeyTrie, MAX_COUNT_PREFIX, split_count_prefix};
pub use metadata::{Category, CommandMetadata, ModeTransition};

/// Global normal mode command registry
///
/// Lazily initialized on first access. Contains all registered
/// normal mode commands with O(1) lookup.
pub fn normal_registry() -> &'static CommandRegistry<NormalMode> {
    static REGISTRY: OnceLock<CommandRegistry<NormalMode>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CommandRegistry::new();
        definitions::register_all(&mut registry);
        registry
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_registry_initialized() {
        let registry = normal_registry();
        assert!(!registry.is_empty());
        assert!(registry.len() >= 40);
    }

    #[test]
    fn test_normal_registry_contains_basic_commands() {
        let registry = normal_registry();

        // Movement
        assert!(registry.contains("h"));
        assert!(registry.contains("j"));
        assert!(registry.contains("k"));
        assert!(registry.contains("l"));

        // Word movement
        assert!(registry.contains("w"));
        assert!(registry.contains("b"));
        assert!(registry.contains("e"));

        // Editing
        assert!(registry.contains("d"));
        assert!(registry.contains("i"));
        assert!(registry.contains("a"));

        // Clipboard
        assert!(registry.contains("y"));
        assert!(registry.contains("p"));

        // Multi-key
        assert!(registry.contains("gg"));
        assert!(registry.contains("gh"));
    }

    #[test]
    fn test_normal_registry_metadata() {
        let registry = normal_registry();

        let h_meta = registry.get_metadata("h").unwrap();
        assert_eq!(h_meta.name, "move_left");
        assert_eq!(h_meta.category, Category::Movement);
        assert!(!h_meta.repeatable);

        let d_meta = registry.get_metadata("d").unwrap();
        assert_eq!(d_meta.name, "delete_selection");
        assert_eq!(d_meta.category, Category::Editing);
        assert!(d_meta.repeatable);
    }

    #[test]
    fn test_normal_registry_categories() {
        let registry = normal_registry();

        let movement = registry.commands_in_category(Category::Movement);
        assert!(!movement.is_empty());
        assert!(movement.contains(&"h"));
        assert!(movement.contains(&"j"));

        let editing = registry.commands_in_category(Category::Editing);
        assert!(!editing.is_empty());
        assert!(editing.contains(&"d"));

        let clipboard = registry.commands_in_category(Category::Clipboard);
        assert!(!clipboard.is_empty());
        assert!(clipboard.contains(&"y"));
    }

    #[test]
    fn test_keytrie_resolution() {
        let registry = normal_registry();
        let trie = registry.key_trie();

        // Single key
        assert_eq!(trie.resolve("h"), KeyMatch::Complete("h".to_string()));

        // Partial (waiting for more)
        assert_eq!(trie.resolve("g"), KeyMatch::Partial);
        assert_eq!(trie.resolve("f"), KeyMatch::Partial);

        // Multi-key complete
        assert_eq!(trie.resolve("gg"), KeyMatch::Complete("gg".to_string()));

        // Parametric complete
        assert_eq!(trie.resolve("fa"), KeyMatch::Complete("fa".to_string()));
    }

    #[test]
    fn test_all_commands_iterator() {
        let registry = normal_registry();
        let all: Vec<_> = registry.all_commands().collect();

        assert!(all.len() >= 40);

        // Check that we have diverse categories
        let has_movement = all.iter().any(|m| m.category == Category::Movement);
        let has_editing = all.iter().any(|m| m.category == Category::Editing);
        let has_clipboard = all.iter().any(|m| m.category == Category::Clipboard);

        assert!(has_movement);
        assert!(has_editing);
        assert!(has_clipboard);
    }
}
