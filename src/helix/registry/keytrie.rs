//! KeyTrie for multi-key sequence resolution
//!
//! Handles sequences like `gg`, `gh`, `fx`, `rx` by determining
//! if a buffer contains a complete command, partial sequence, or invalid input.

use std::collections::{HashMap, HashSet};

/// Result of matching a key sequence against the trie
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyMatch {
    /// Complete match - command ready to execute
    Complete(String),
    /// Partial match - more keys needed (g, r, f, F, t, T)
    Partial,
    /// No match - invalid sequence
    Invalid,
}

/// KeyTrie for resolving multi-key command sequences
///
/// Handles three types of sequences:
/// 1. Single-key commands: "h", "j", "d", etc.
/// 2. Multi-key commands: "gg", "gh", "gl", "gs", "ge"
/// 3. Character-input commands: "fx", "Fx", "tx", "Tx", "rx"
#[derive(Debug)]
pub struct KeyTrie {
    /// Multi-key command sequences (e.g., "gg" -> "gg")
    multi_key: HashMap<&'static str, &'static str>,
    /// Prefixes that expect arbitrary character input
    char_input_prefixes: HashSet<char>,
    /// Known single-key commands
    single_key: HashSet<&'static str>,
}

impl KeyTrie {
    /// Create a new KeyTrie with standard Helix sequences
    pub fn new() -> Self {
        let mut trie = Self {
            multi_key: HashMap::new(),
            char_input_prefixes: HashSet::new(),
            single_key: HashSet::new(),
        };

        // Register multi-key goto commands
        trie.multi_key.insert("gg", "gg");
        trie.multi_key.insert("gh", "gh");
        trie.multi_key.insert("gl", "gl");
        trie.multi_key.insert("gs", "gs");
        trie.multi_key.insert("ge", "ge");

        // Register character-input prefixes
        trie.char_input_prefixes.insert('f');
        trie.char_input_prefixes.insert('F');
        trie.char_input_prefixes.insert('t');
        trie.char_input_prefixes.insert('T');
        trie.char_input_prefixes.insert('r');

        trie
    }

    /// Register a single-key command
    pub fn register_single(&mut self, key: &'static str) {
        self.single_key.insert(key);
    }

    /// Resolve a command buffer to a KeyMatch
    ///
    /// # Arguments
    /// * `buffer` - The accumulated key sequence
    ///
    /// # Returns
    /// * `Complete(cmd)` - Buffer contains a complete command
    /// * `Partial` - Buffer is a valid prefix, waiting for more input
    /// * `Invalid` - Buffer is not a valid command or prefix
    pub fn resolve(&self, buffer: &str) -> KeyMatch {
        if buffer.is_empty() {
            return KeyMatch::Invalid;
        }

        let len = buffer.len();
        let first_char = buffer.chars().next().unwrap();

        // Check for multi-key command
        if len == 2 {
            // Multi-key goto commands
            if let Some(&cmd) = self.multi_key.get(buffer) {
                return KeyMatch::Complete(cmd.to_string());
            }

            // Character-input commands (fx, tx, rx, etc.)
            if self.char_input_prefixes.contains(&first_char) {
                return KeyMatch::Complete(buffer.to_string());
            }

            // Invalid 2-char sequence
            return KeyMatch::Invalid;
        }

        // Single character
        if len == 1 {
            // Check if waiting for char input (f, t, r, etc.)
            if self.char_input_prefixes.contains(&first_char) {
                return KeyMatch::Partial;
            }

            // Check if this is a goto prefix
            if first_char == 'g' {
                return KeyMatch::Partial;
            }

            // Single-key commands are complete
            return KeyMatch::Complete(buffer.to_string());
        }

        // Longer sequences are invalid
        KeyMatch::Invalid
    }

    /// Check if a character is a char-input prefix
    pub fn is_char_input_prefix(&self, ch: char) -> bool {
        self.char_input_prefixes.contains(&ch)
    }

    /// Check if a character starts a multi-key sequence
    pub fn is_multi_key_prefix(&self, ch: char) -> bool {
        ch == 'g' || self.char_input_prefixes.contains(&ch)
    }
}

impl Default for KeyTrie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_key_complete() {
        let trie = KeyTrie::new();
        assert_eq!(trie.resolve("h"), KeyMatch::Complete("h".to_string()));
        assert_eq!(trie.resolve("j"), KeyMatch::Complete("j".to_string()));
        assert_eq!(trie.resolve("d"), KeyMatch::Complete("d".to_string()));
        assert_eq!(trie.resolve("x"), KeyMatch::Complete("x".to_string()));
    }

    #[test]
    fn test_goto_prefix_partial() {
        let trie = KeyTrie::new();
        assert_eq!(trie.resolve("g"), KeyMatch::Partial);
    }

    #[test]
    fn test_goto_commands_complete() {
        let trie = KeyTrie::new();
        assert_eq!(trie.resolve("gg"), KeyMatch::Complete("gg".to_string()));
        assert_eq!(trie.resolve("gh"), KeyMatch::Complete("gh".to_string()));
        assert_eq!(trie.resolve("gl"), KeyMatch::Complete("gl".to_string()));
        assert_eq!(trie.resolve("gs"), KeyMatch::Complete("gs".to_string()));
        assert_eq!(trie.resolve("ge"), KeyMatch::Complete("ge".to_string()));
    }

    #[test]
    fn test_char_input_prefix_partial() {
        let trie = KeyTrie::new();
        assert_eq!(trie.resolve("f"), KeyMatch::Partial);
        assert_eq!(trie.resolve("F"), KeyMatch::Partial);
        assert_eq!(trie.resolve("t"), KeyMatch::Partial);
        assert_eq!(trie.resolve("T"), KeyMatch::Partial);
        assert_eq!(trie.resolve("r"), KeyMatch::Partial);
    }

    #[test]
    fn test_char_input_commands_complete() {
        let trie = KeyTrie::new();
        assert_eq!(trie.resolve("fa"), KeyMatch::Complete("fa".to_string()));
        assert_eq!(trie.resolve("Fx"), KeyMatch::Complete("Fx".to_string()));
        assert_eq!(trie.resolve("t9"), KeyMatch::Complete("t9".to_string()));
        assert_eq!(trie.resolve("T!"), KeyMatch::Complete("T!".to_string()));
        assert_eq!(trie.resolve("rz"), KeyMatch::Complete("rz".to_string()));
    }

    #[test]
    fn test_invalid_sequences() {
        let trie = KeyTrie::new();
        assert_eq!(trie.resolve(""), KeyMatch::Invalid);
        assert_eq!(trie.resolve("xyz"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("ab"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("gx"), KeyMatch::Invalid); // Not a valid goto
    }

    #[test]
    fn test_is_char_input_prefix() {
        let trie = KeyTrie::new();
        assert!(trie.is_char_input_prefix('f'));
        assert!(trie.is_char_input_prefix('F'));
        assert!(trie.is_char_input_prefix('t'));
        assert!(trie.is_char_input_prefix('T'));
        assert!(trie.is_char_input_prefix('r'));
        assert!(!trie.is_char_input_prefix('g'));
        assert!(!trie.is_char_input_prefix('h'));
    }

    #[test]
    fn test_is_multi_key_prefix() {
        let trie = KeyTrie::new();
        assert!(trie.is_multi_key_prefix('g'));
        assert!(trie.is_multi_key_prefix('f'));
        assert!(trie.is_multi_key_prefix('r'));
        assert!(!trie.is_multi_key_prefix('h'));
        assert!(!trie.is_multi_key_prefix('d'));
    }
}
