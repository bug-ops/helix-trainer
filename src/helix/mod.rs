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
//! let state = sim.state()?;
//! assert_eq!(state.cursor_position().col, 6);
//! # Ok::<(), helix_trainer::security::UserError>(())
//! ```

pub mod commands;
pub mod executor;
pub mod macro_recorder;
pub mod registry;
pub mod repeat;
pub mod simulator;

pub use commands::*;
pub use executor::CommandExecutor;
pub use repeat::{Movement, RepeatBuffer, RepeatableAction, is_repeatable_command};
pub use simulator::{
    AnyModeSimulator, EditorDisplay, EditorMode, EditorSnapshot, HelixSimulator, InsertMode, Mode,
    NormalMode, SelectionBounds, SerializableRange,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== get_bracket_pair tests ====================

    #[test]
    fn test_get_bracket_pair_all_types() {
        // Parentheses
        assert_eq!(get_bracket_pair('('), ('(', ')'));
        assert_eq!(get_bracket_pair(')'), ('(', ')'));

        // Square brackets
        assert_eq!(get_bracket_pair('['), ('[', ']'));
        assert_eq!(get_bracket_pair(']'), ('[', ']'));

        // Curly braces
        assert_eq!(get_bracket_pair('{'), ('{', '}'));
        assert_eq!(get_bracket_pair('}'), ('{', '}'));

        // Angle brackets
        assert_eq!(get_bracket_pair('<'), ('<', '>'));
        assert_eq!(get_bracket_pair('>'), ('<', '>'));

        // Quotes and other characters return themselves
        assert_eq!(get_bracket_pair('"'), ('"', '"'));
        assert_eq!(get_bracket_pair('\''), ('\'', '\''));
        assert_eq!(get_bracket_pair('`'), ('`', '`'));
        assert_eq!(get_bracket_pair('x'), ('x', 'x'));
    }

    // ==================== find_surrounding_brackets tests ====================

    #[test]
    fn test_find_brackets_parentheses() {
        let content = "fn test(arg) { }";
        // Cursor inside parentheses at position 8 (on 'a')
        let result = find_surrounding_brackets(content, 0, 8, '(');
        assert!(result.is_some());
        let (open_row, open_col, close_row, close_col) = result.unwrap();
        assert_eq!(open_row, 0);
        assert_eq!(open_col, 7); // Position of '('
        assert_eq!(close_row, 0);
        assert_eq!(close_col, 11); // Position of ')'
    }

    #[test]
    fn test_find_brackets_empty_content() {
        let content = "";
        let result = find_surrounding_brackets(content, 0, 0, '(');
        assert!(result.is_none());
    }

    #[test]
    fn test_find_brackets_no_match() {
        let content = "hello world";
        // No brackets in content
        let result = find_surrounding_brackets(content, 0, 5, '(');
        assert!(result.is_none());
    }

    #[test]
    fn test_find_brackets_cursor_outside() {
        let content = "before (inside) after";
        // Cursor at position 0 (before the brackets)
        let result = find_surrounding_brackets(content, 0, 0, '(');
        assert!(result.is_none());
    }

    #[test]
    fn test_find_brackets_nested() {
        let content = "fn test((inner)) { }";
        // Cursor inside inner parentheses at position 9 (on 'i')
        let result = find_surrounding_brackets(content, 0, 9, '(');
        assert!(result.is_some());
        let (open_row, open_col, close_row, close_col) = result.unwrap();
        // Should find the innermost pair
        assert_eq!(open_row, 0);
        assert_eq!(open_col, 8); // Inner '('
        assert_eq!(close_row, 0);
        assert_eq!(close_col, 14); // Inner ')'
    }

    #[test]
    fn test_find_brackets_multiline() {
        let content = "fn test(\n  arg1,\n  arg2\n)";
        // Cursor on line 1, col 2 (on 'a' of arg1)
        let result = find_surrounding_brackets(content, 1, 2, '(');
        assert!(result.is_some());
        let (open_row, open_col, close_row, close_col) = result.unwrap();
        assert_eq!(open_row, 0);
        assert_eq!(open_col, 7); // '(' on line 0
        assert_eq!(close_row, 3);
        assert_eq!(close_col, 0); // ')' on line 3
    }

    #[test]
    fn test_find_brackets_curly_braces() {
        let content = "if true { body }";
        // Cursor inside braces at position 10 (on 'b')
        let result = find_surrounding_brackets(content, 0, 10, '{');
        assert!(result.is_some());
        let (open_row, open_col, close_row, close_col) = result.unwrap();
        assert_eq!(open_row, 0);
        assert_eq!(open_col, 8); // Position of '{'
        assert_eq!(close_row, 0);
        assert_eq!(close_col, 15); // Position of '}'
    }

    #[test]
    fn test_find_brackets_square_brackets() {
        let content = "let arr = [1, 2, 3];";
        // Cursor inside brackets at position 11 (on '1')
        let result = find_surrounding_brackets(content, 0, 11, '[');
        assert!(result.is_some());
        let (open_row, open_col, close_row, close_col) = result.unwrap();
        assert_eq!(open_row, 0);
        assert_eq!(open_col, 10); // Position of '['
        assert_eq!(close_row, 0);
        assert_eq!(close_col, 18); // Position of ']'
    }

    #[test]
    fn test_find_brackets_angle_brackets() {
        let content = "Vec<String>";
        // Cursor inside angle brackets at position 4 (on 'S')
        let result = find_surrounding_brackets(content, 0, 4, '<');
        assert!(result.is_some());
        let (open_row, open_col, close_row, close_col) = result.unwrap();
        assert_eq!(open_row, 0);
        assert_eq!(open_col, 3); // Position of '<'
        assert_eq!(close_row, 0);
        assert_eq!(close_col, 10); // Position of '>'
    }

    #[test]
    fn test_find_brackets_cursor_on_bracket() {
        let content = "(hello)";
        // Cursor on the opening bracket
        let result = find_surrounding_brackets(content, 0, 0, '(');
        // helix-core should find the pair when cursor is on bracket
        assert!(result.is_some());
    }
}
