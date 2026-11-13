//! Helix editor integration using helix-core library
//!
//! This module provides a Helix editor simulator using the battle-tested
//! helix-core library. It handles text editing operations with proper
//! unicode support, undo/redo, and multi-cursor capabilities.
//!
//! # Examples
//!
//! ```
//! use helix_trainer::helix::{AnyModeSimulator, Pos};
//!
//! let mut sim = AnyModeSimulator::new("hello world".to_string());
//! sim.execute_command("w")?;  // Move to next word
//! assert_eq!(sim.display().cursor_position(), Pos::new(0, 6));
//! # Ok::<(), helix_trainer::error::UserError>(())
//! ```

pub mod error;
#[cfg(any(test, feature = "test-util"))]
pub mod goldens;
pub mod indent;
mod insert_input;
pub mod language;
pub mod macro_recorder;
mod newline_indent;
pub mod normal_command;
mod position;
pub mod registry;
pub mod repeat;
pub mod simulator;

pub use indent::{IndentUnit, TabWidth};
pub use insert_input::InsertInput;
pub use language::{Language, LanguageRules, SmartTab};
pub use position::{CursorRange, Pos};
pub use repeat::{InsertAction, InsertKey, Movement, RecordedInsert, RepeatBuffer};
pub use simulator::{
    AnyModeSimulator, EditorDisplay, EditorMode, EditorSnapshot, HelixSimulator, InsertMode, Mode,
    ModeKind, NormalMode, SelectionBounds, SerializableRange,
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
/// * `cursor` - Cursor position
/// * `bracket_char` - The bracket type to find (e.g., '[', '(', '{', '"')
///
/// # Returns
///
/// Option containing the (`open`, `close`) bracket positions if found
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::{Pos, find_surrounding_brackets};
///
/// let content = "fn test() { println!(\"hello\"); }";
/// // Cursor inside the parentheses of println
/// if let Some((open, close)) = find_surrounding_brackets(content, Pos::new(0, 22), '(') {
///     assert_eq!(open.col(), 20); // Opening paren position
///     assert_eq!(close.col(), 28); // Closing paren position
/// }
/// ```
pub fn find_surrounding_brackets(
    content: &str,
    cursor: Pos,
    bracket_char: char,
) -> Option<(Pos, Pos)> {
    use helix_core::{Rope, Selection, surround::find_nth_pairs_pos};

    let (open_char, _close_char) = get_bracket_pair(bracket_char);

    // Convert row/col to absolute position
    let rope = Rope::from(content);
    let row = cursor.row().min(rope.len_lines().saturating_sub(1));
    let line_start = rope.line_to_char(row);
    let line = rope.line(row);
    let cursor_abs = line_start + cursor.col().min(line.len_chars().saturating_sub(1));

    // Create a selection at cursor position
    let selection = Selection::point(cursor_abs);
    let range = selection.primary();

    // Use helix-core to find the pair
    let (open_pos, close_pos) = find_nth_pairs_pos(rope.slice(..), open_char, range, 1).ok()?;

    // Convert absolute positions back to row/col
    let to_pos = |abs: usize| {
        let row = rope.char_to_line(abs);
        Pos::new(row, abs - rope.line_to_char(row))
    };

    Some((to_pos(open_pos), to_pos(close_pos)))
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
        let result = find_surrounding_brackets(content, Pos::new(0, 8), '(');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, Pos::new(0, 7)); // Position of '('
        assert_eq!(close, Pos::new(0, 11)); // Position of ')'
    }

    #[test]
    fn test_find_brackets_empty_content() {
        let content = "";
        let result = find_surrounding_brackets(content, Pos::new(0, 0), '(');
        assert!(result.is_none());
    }

    #[test]
    fn test_find_brackets_no_match() {
        let content = "hello world";
        // No brackets in content
        let result = find_surrounding_brackets(content, Pos::new(0, 5), '(');
        assert!(result.is_none());
    }

    #[test]
    fn test_find_brackets_cursor_outside() {
        let content = "before (inside) after";
        // Cursor at position 0 (before the brackets)
        let result = find_surrounding_brackets(content, Pos::new(0, 0), '(');
        assert!(result.is_none());
    }

    #[test]
    fn test_find_brackets_nested() {
        let content = "fn test((inner)) { }";
        // Cursor inside inner parentheses at position 9 (on 'i')
        let result = find_surrounding_brackets(content, Pos::new(0, 9), '(');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        // Should find the innermost pair
        assert_eq!(open, Pos::new(0, 8)); // Inner '('
        assert_eq!(close, Pos::new(0, 14)); // Inner ')'
    }

    #[test]
    fn test_find_brackets_multiline() {
        let content = "fn test(\n  arg1,\n  arg2\n)";
        // Cursor on line 1, col 2 (on 'a' of arg1)
        let result = find_surrounding_brackets(content, Pos::new(1, 2), '(');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, Pos::new(0, 7)); // '(' on line 0
        assert_eq!(close, Pos::new(3, 0)); // ')' on line 3
    }

    #[test]
    fn test_find_brackets_curly_braces() {
        let content = "if true { body }";
        // Cursor inside braces at position 10 (on 'b')
        let result = find_surrounding_brackets(content, Pos::new(0, 10), '{');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, Pos::new(0, 8)); // Position of '{'
        assert_eq!(close, Pos::new(0, 15)); // Position of '}'
    }

    #[test]
    fn test_find_brackets_square_brackets() {
        let content = "let arr = [1, 2, 3];";
        // Cursor inside brackets at position 11 (on '1')
        let result = find_surrounding_brackets(content, Pos::new(0, 11), '[');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, Pos::new(0, 10)); // Position of '['
        assert_eq!(close, Pos::new(0, 18)); // Position of ']'
    }

    #[test]
    fn test_find_brackets_angle_brackets() {
        let content = "Vec<String>";
        // Cursor inside angle brackets at position 4 (on 'S')
        let result = find_surrounding_brackets(content, Pos::new(0, 4), '<');
        assert!(result.is_some());
        let (open, close) = result.unwrap();
        assert_eq!(open, Pos::new(0, 3)); // Position of '<'
        assert_eq!(close, Pos::new(0, 10)); // Position of '>'
    }

    #[test]
    fn test_find_brackets_cursor_on_bracket() {
        let content = "(hello)";
        // Cursor on the opening bracket
        let result = find_surrounding_brackets(content, Pos::new(0, 0), '(');
        // helix-core should find the pair when cursor is on bracket
        assert!(result.is_some());
    }
}
