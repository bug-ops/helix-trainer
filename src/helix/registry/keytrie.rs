//! KeyTrie for multi-key sequence resolution
//!
//! Handles sequences like `gg`, `gh`, `fx`, `rx`, and count prefixes like `3h`, `10j`
//! by determining if a buffer contains a complete command, partial sequence, or invalid input.

use std::collections::{HashMap, HashSet};

/// Maximum allowed count prefix to prevent abuse
pub const MAX_COUNT_PREFIX: usize = 999;

/// Result of matching a key sequence against the trie
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyMatch {
    /// Complete match - command ready to execute
    Complete(String),
    /// Partial match - more keys needed (g, r, f, F, t, T, or digits)
    Partial,
    /// No match - invalid sequence
    Invalid,
}

/// Split a buffer into count prefix and command parts
///
/// # Examples
/// - "3h" -> (Some(3), "h")
/// - "12j" -> (Some(12), "j")
/// - "h" -> (None, "h")
/// - "3" -> (Some(3), "")
/// - "3gg" -> (Some(3), "gg")
pub fn split_count_prefix(buffer: &str) -> (Option<usize>, &str) {
    let digit_end = buffer
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .map(|c| c.len_utf8())
        .sum();

    if digit_end == 0 {
        return (None, buffer);
    }

    let count_str = &buffer[..digit_end];
    let cmd_str = &buffer[digit_end..];

    match count_str.parse::<usize>() {
        Ok(count) if count <= MAX_COUNT_PREFIX => (Some(count), cmd_str),
        _ => (None, buffer), // Invalid count, treat as invalid
    }
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

        // Register multi-key match mode commands
        trie.multi_key.insert("mm", "mm");

        // Register character-input prefixes
        trie.char_input_prefixes.insert('f');
        trie.char_input_prefixes.insert('F');
        trie.char_input_prefixes.insert('t');
        trie.char_input_prefixes.insert('T');
        trie.char_input_prefixes.insert('r');

        trie
    }

    /// Register a single-key command (including modifier keys like Alt-x, Ctrl-c)
    pub fn register_single(&mut self, key: &'static str) {
        self.single_key.insert(key);
    }

    /// Check if a key string is a registered single-key command (including modifiers)
    pub fn is_registered_single(&self, key: &str) -> bool {
        self.single_key.contains(key)
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

        // Check for modifier key commands first (Alt-*, Ctrl-*)
        // These are registered as single commands but have longer strings
        if self.single_key.contains(buffer) && buffer.len() > 1 {
            return KeyMatch::Complete(buffer.to_string());
        }

        // Check for count prefix (e.g., "3h", "12j", "10w")
        // Note: "0" alone is NOT a command in Helix (use "gh" for goto line start)
        // Count prefixes must start with 1-9, but can contain 0 (e.g., "10j")
        let first_char = buffer.chars().next().unwrap();
        let has_count_prefix =
            first_char.is_ascii_digit() && first_char != '0' && !buffer.is_empty();

        if has_count_prefix {
            let (count, cmd_part) = split_count_prefix(buffer);

            if count.is_some() {
                // We have a count prefix
                if cmd_part.is_empty() {
                    // Just digits so far, waiting for command
                    return KeyMatch::Partial;
                }

                // Validate the command part (only single-key commands with count)
                // Multi-key commands like "3gg" and char-input like "3fx" are not supported
                if cmd_part.len() == 1 {
                    let cmd_char = cmd_part.chars().next().unwrap();
                    // Don't allow count with char-input prefixes (f, t, r, etc.),
                    // goto prefix (g), or match mode prefix (m)
                    if !self.char_input_prefixes.contains(&cmd_char)
                        && cmd_char != 'g'
                        && cmd_char != 'm'
                    {
                        return KeyMatch::Complete(buffer.to_string());
                    }
                }

                // Invalid count+command combination
                return KeyMatch::Invalid;
            }
        }

        // No count prefix - use standard resolution
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

            // Match mode text object prefixes (ma, mi) - waiting for object char
            if buffer == "ma" || buffer == "mi" {
                return KeyMatch::Partial;
            }

            // Match mode surround prefixes (ms, md) - waiting for char
            if buffer == "ms" || buffer == "md" {
                return KeyMatch::Partial;
            }

            // Match mode surround replace prefix (mr) - waiting for from/to chars
            if buffer == "mr" {
                return KeyMatch::Partial;
            }

            // Invalid 2-char sequence
            return KeyMatch::Invalid;
        }

        // Check for 3-char match mode commands
        if len == 3 && first_char == 'm' {
            let second_char = buffer.chars().nth(1).unwrap();
            // Text object commands: ma{obj} or mi{obj}
            if second_char == 'a' || second_char == 'i' {
                return KeyMatch::Complete(buffer.to_string());
            }
            // Surround add/delete: ms{char} or md{char}
            if second_char == 's' || second_char == 'd' {
                return KeyMatch::Complete(buffer.to_string());
            }
            // Surround replace: mr{from} - still waiting for {to}
            if second_char == 'r' {
                return KeyMatch::Partial;
            }
        }

        // Check for 4-char surround replace command: mr{from}{to}
        if len == 4 && buffer.starts_with("mr") {
            return KeyMatch::Complete(buffer.to_string());
        }

        // Single character
        if len == 1 {
            // Check if waiting for char input (f, t, r, etc.)
            if self.char_input_prefixes.contains(&first_char) {
                return KeyMatch::Partial;
            }

            // Check if this is a goto prefix or match mode prefix
            if first_char == 'g' || first_char == 'm' {
                return KeyMatch::Partial;
            }

            // Check if it's a registered single-key command
            if self.single_key.contains(buffer) {
                return KeyMatch::Complete(buffer.to_string());
            }

            // Unknown single character - invalid
            return KeyMatch::Invalid;
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
    use crate::helix::registry::normal_registry;

    #[test]
    fn test_single_key_complete() {
        // Use registry's trie which has registered commands
        let trie = normal_registry().key_trie();
        assert_eq!(trie.resolve("h"), KeyMatch::Complete("h".to_string()));
        assert_eq!(trie.resolve("j"), KeyMatch::Complete("j".to_string()));
        assert_eq!(trie.resolve("d"), KeyMatch::Complete("d".to_string()));
        assert_eq!(trie.resolve("x"), KeyMatch::Complete("x".to_string()));
    }

    #[test]
    fn test_single_key_invalid() {
        // Use registry's trie to check invalid commands
        let trie = normal_registry().key_trie();
        // '0' and '$' are NOT commands in Helix
        assert_eq!(trie.resolve("0"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("$"), KeyMatch::Invalid);
        // 'G' is NOT a command - use 'ge' for goto last line
        assert_eq!(trie.resolve("G"), KeyMatch::Invalid);
    }

    #[test]
    fn test_goto_prefix_partial() {
        let trie = normal_registry().key_trie();
        assert_eq!(trie.resolve("g"), KeyMatch::Partial);
    }

    #[test]
    fn test_goto_commands_complete() {
        let trie = normal_registry().key_trie();
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

    // Count prefix tests
    #[test]
    fn test_split_count_prefix_with_count() {
        assert_eq!(split_count_prefix("3h"), (Some(3), "h"));
        assert_eq!(split_count_prefix("12j"), (Some(12), "j"));
        assert_eq!(split_count_prefix("999k"), (Some(999), "k"));
    }

    #[test]
    fn test_split_count_prefix_no_count() {
        assert_eq!(split_count_prefix("h"), (None, "h"));
        assert_eq!(split_count_prefix("gg"), (None, "gg"));
        assert_eq!(split_count_prefix("fx"), (None, "fx"));
    }

    #[test]
    fn test_split_count_prefix_only_digits() {
        assert_eq!(split_count_prefix("3"), (Some(3), ""));
        assert_eq!(split_count_prefix("123"), (Some(123), ""));
    }

    #[test]
    fn test_split_count_prefix_exceeds_max() {
        // Count > MAX_COUNT_PREFIX should be treated as invalid
        assert_eq!(split_count_prefix("1000h"), (None, "1000h"));
        assert_eq!(split_count_prefix("9999j"), (None, "9999j"));
    }

    #[test]
    fn test_count_prefix_partial() {
        let trie = KeyTrie::new();
        // Just digits should be partial (waiting for command)
        assert_eq!(trie.resolve("3"), KeyMatch::Partial);
        assert_eq!(trie.resolve("12"), KeyMatch::Partial);
        assert_eq!(trie.resolve("999"), KeyMatch::Partial);
    }

    #[test]
    fn test_count_prefix_complete() {
        let trie = KeyTrie::new();
        // Count + single-key command should be complete
        assert_eq!(trie.resolve("3h"), KeyMatch::Complete("3h".to_string()));
        assert_eq!(trie.resolve("3l"), KeyMatch::Complete("3l".to_string()));
        assert_eq!(trie.resolve("5j"), KeyMatch::Complete("5j".to_string()));
        assert_eq!(trie.resolve("5k"), KeyMatch::Complete("5k".to_string()));
        assert_eq!(trie.resolve("10w"), KeyMatch::Complete("10w".to_string()));
        assert_eq!(trie.resolve("99d"), KeyMatch::Complete("99d".to_string()));
    }

    #[test]
    fn test_count_prefix_invalid_combinations() {
        let trie = KeyTrie::new();
        // Count + goto prefix (g) is invalid
        assert_eq!(trie.resolve("3g"), KeyMatch::Invalid);
        // Count + match mode prefix (m) is invalid
        assert_eq!(trie.resolve("3m"), KeyMatch::Invalid);
        // Count + char-input prefix (f, t, r) is invalid
        assert_eq!(trie.resolve("3f"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("3r"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("3t"), KeyMatch::Invalid);
        // Count + multi-key command is invalid
        assert_eq!(trie.resolve("3gg"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("3mm"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("3fx"), KeyMatch::Invalid);
    }

    // Match mode text object tests (Phase 1)

    #[test]
    fn test_match_mode_text_object_prefixes_partial() {
        let trie = KeyTrie::new();
        // ma and mi are partial - waiting for object char
        assert_eq!(trie.resolve("ma"), KeyMatch::Partial);
        assert_eq!(trie.resolve("mi"), KeyMatch::Partial);
    }

    #[test]
    fn test_match_mode_text_object_commands_complete() {
        let trie = KeyTrie::new();
        // 3-char text object commands are complete
        assert_eq!(trie.resolve("miw"), KeyMatch::Complete("miw".to_string()));
        assert_eq!(trie.resolve("maw"), KeyMatch::Complete("maw".to_string()));
        assert_eq!(trie.resolve("mi("), KeyMatch::Complete("mi(".to_string()));
        assert_eq!(trie.resolve("ma("), KeyMatch::Complete("ma(".to_string()));
        assert_eq!(trie.resolve("mi["), KeyMatch::Complete("mi[".to_string()));
        assert_eq!(trie.resolve("ma["), KeyMatch::Complete("ma[".to_string()));
        assert_eq!(trie.resolve("mi{"), KeyMatch::Complete("mi{".to_string()));
        assert_eq!(trie.resolve("ma{"), KeyMatch::Complete("ma{".to_string()));
        assert_eq!(trie.resolve("mip"), KeyMatch::Complete("mip".to_string()));
        assert_eq!(trie.resolve("map"), KeyMatch::Complete("map".to_string()));
        assert_eq!(trie.resolve("mi\""), KeyMatch::Complete("mi\"".to_string()));
        assert_eq!(trie.resolve("ma'"), KeyMatch::Complete("ma'".to_string()));
    }

    #[test]
    fn test_match_mode_surround_prefixes_partial() {
        let trie = KeyTrie::new();
        // ms, md, mr are partial - waiting for char(s)
        assert_eq!(trie.resolve("ms"), KeyMatch::Partial);
        assert_eq!(trie.resolve("md"), KeyMatch::Partial);
        assert_eq!(trie.resolve("mr"), KeyMatch::Partial);
    }

    #[test]
    fn test_match_mode_surround_add_delete_complete() {
        let trie = KeyTrie::new();
        // ms{char} and md{char} are complete
        assert_eq!(trie.resolve("ms("), KeyMatch::Complete("ms(".to_string()));
        assert_eq!(trie.resolve("ms["), KeyMatch::Complete("ms[".to_string()));
        assert_eq!(trie.resolve("md("), KeyMatch::Complete("md(".to_string()));
        assert_eq!(trie.resolve("md{"), KeyMatch::Complete("md{".to_string()));
    }

    #[test]
    fn test_match_mode_surround_replace_partial_then_complete() {
        let trie = KeyTrie::new();
        // mr{from} is partial - waiting for 'to' char
        assert_eq!(trie.resolve("mr("), KeyMatch::Partial);
        assert_eq!(trie.resolve("mr["), KeyMatch::Partial);
        // mr{from}{to} is complete
        assert_eq!(trie.resolve("mr()"), KeyMatch::Complete("mr()".to_string()));
        assert_eq!(trie.resolve("mr(["), KeyMatch::Complete("mr([".to_string()));
        assert_eq!(trie.resolve("mr{("), KeyMatch::Complete("mr{(".to_string()));
    }

    #[test]
    fn test_match_mode_invalid_fourth_char() {
        let trie = KeyTrie::new();
        // 4-char sequences that don't start with 'mr' are invalid
        assert_eq!(trie.resolve("miwa"), KeyMatch::Invalid);
        assert_eq!(trie.resolve("mawx"), KeyMatch::Invalid);
    }
}
