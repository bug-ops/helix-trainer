//! Helix editor integration using helix-core library
//!
//! This module provides a Helix editor simulator using the battle-tested
//! helix-core library. It handles text editing operations with proper
//! unicode support, undo/redo, and multi-cursor capabilities.
//!
//! # Examples
//!
//! ```ignore
//! use helix_trainer::helix::HelixSimulator;
//!
//! let mut sim = HelixSimulator::new("hello world".to_string());
//! sim.execute_command("w")?;  // Move to next word
//! let state = sim.get_state()?;
//! assert_eq!(state.cursor_position().col, 6);
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```

pub mod commands;
pub mod executor;
pub mod registry;
pub mod repeat;
pub mod simulator;

pub use commands::*;
pub use executor::CommandExecutor;
pub use repeat::{Movement, RepeatBuffer, RepeatableAction, is_repeatable_command};
pub use simulator::{AnyModeSimulator, EditorMode, HelixSimulator, InsertMode, Mode, NormalMode};

/// Get the open/close pair for a bracket or quote character
///
/// Returns a tuple of (open, close) characters for bracket types,
/// or (char, char) for quotes and other characters.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::get_bracket_pair;
///
/// assert_eq!(get_bracket_pair('['), ('[', ']'));
/// assert_eq!(get_bracket_pair(')'), ('(', ')'));
/// assert_eq!(get_bracket_pair('"'), ('"', '"'));
/// ```
#[inline]
pub fn get_bracket_pair(ch: char) -> (char, char) {
    match ch {
        '(' | ')' => ('(', ')'),
        '[' | ']' => ('[', ']'),
        '{' | '}' => ('{', '}'),
        '<' | '>' => ('<', '>'),
        _ => (ch, ch),
    }
}

/// Find surrounding bracket positions in text content
///
/// Uses helix-core's `find_nth_pairs_pos` function to locate matching
/// bracket pairs around the cursor position.
///
/// # Arguments
///
/// * `content` - Text content to search
/// * `cursor_row` - Cursor row (0-indexed)
/// * `cursor_col` - Cursor column (0-indexed)
/// * `bracket_char` - The bracket type to find (e.g., '[', '(', '{', '"')
///
/// # Returns
///
/// Option containing (open_row, open_col, close_row, close_col) if found
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::find_surrounding_brackets;
///
/// let content = "fn test() { println!(\"hello\"); }";
/// // Cursor inside the parentheses of println
/// if let Some((or, oc, cr, cc)) = find_surrounding_brackets(content, 0, 22, '(') {
///     assert_eq!(oc, 20); // Opening paren position
///     assert_eq!(cc, 28); // Closing paren position
/// }
/// ```
pub fn find_surrounding_brackets(
    content: &str,
    cursor_row: usize,
    cursor_col: usize,
    bracket_char: char,
) -> Option<(usize, usize, usize, usize)> {
    use helix_core::{Rope, Selection, surround::find_nth_pairs_pos};

    let (open_char, _close_char) = get_bracket_pair(bracket_char);

    // Convert row/col to absolute position
    let rope = Rope::from(content);
    let line_start = rope.line_to_char(cursor_row.min(rope.len_lines().saturating_sub(1)));
    let line = rope.line(cursor_row.min(rope.len_lines().saturating_sub(1)));
    let cursor_abs = line_start + cursor_col.min(line.len_chars().saturating_sub(1));

    // Create a selection at cursor position
    let selection = Selection::point(cursor_abs);
    let range = selection.primary();

    // Use helix-core to find the pair
    let (open_pos, close_pos) = find_nth_pairs_pos(rope.slice(..), open_char, range, 1).ok()?;

    // Convert absolute positions back to row/col
    let open_row = rope.char_to_line(open_pos);
    let open_col = open_pos - rope.line_to_char(open_row);

    let close_row = rope.char_to_line(close_pos);
    let close_col = close_pos - rope.line_to_char(close_row);

    Some((open_row, open_col, close_row, close_col))
}
