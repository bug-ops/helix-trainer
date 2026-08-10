//! Editing command tests for HelixSimulator
//!
//! Tests for text modification commands including:
//! - Delete operations: d, x+d
//! - Undo/redo: u
//! - Line operations: o, O, J
//! - Indentation: >, <
//! - Character replacement: r
//! - Change command: c

use crate::game::EditorState;
use crate::game::editor_state::{CursorPosition, Selection};
use crate::helix::commands::*;
use crate::helix::simulator::{AnyModeSimulator, Mode};

// ============================================================================
// Delete Tests
// ============================================================================

#[test]
fn test_delete_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "ello");
}

#[test]
fn test_delete_char_in_middle() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // Move to 'e'
    sim.execute_command(CMD_MOVE_RIGHT).unwrap(); // Move to 'l'
    sim.execute_command(CMD_DELETE_SELECTION).unwrap(); // Delete 'l'

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "helo");
}

#[test]
fn test_multiple_line_deletions() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    // Delete first line with x+d
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    // Delete second line with x+d
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "line3\n");
}

// ============================================================================
// Undo/Redo Tests
// ============================================================================

#[test]
fn test_undo() {
    let mut sim = AnyModeSimulator::new("test\n".to_string());

    // Delete line using x (select) + d (delete)
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "test\n");
}

#[test]
fn test_undo_then_redo_round_trip() {
    let mut sim = AnyModeSimulator::new("test\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "test\n");

    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");
}

#[test]
fn test_redo_via_ctrl_r_alias() {
    let mut sim = AnyModeSimulator::new("test\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "test\n");

    sim.execute_command(CMD_CTRL_R).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");
}

#[test]
fn test_multiple_sequential_undo_redo_walks_full_history() {
    let mut sim = AnyModeSimulator::new("aaa\nbbb\nccc\n".to_string());

    // Three edits, each deleting the current (now-first) line
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bbb\nccc\n");

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "ccc\n");

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");

    // Undo 3x should walk back through each intermediate state
    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "ccc\n");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bbb\nccc\n");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "aaa\nbbb\nccc\n");

    // Redo 3x should walk forward and land back at the latest state
    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bbb\nccc\n");

    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "ccc\n");

    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");
}

#[test]
fn test_new_edit_after_undo_clears_redo_stack() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3\n".to_string());

    // Delete line1, then undo it back
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "line1\nline2\nline3\n");

    // A fresh edit on a different line must invalidate the pending redo
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "line1\nline3\n");

    // Redo should now be a no-op: the stale "restore line1 deletion" entry is gone
    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "line1\nline3\n");
}

#[test]
fn test_noop_command_after_undo_does_not_clear_pending_redo() {
    // Critic finding S3: a command that produces a pure-retain (no-op)
    // changeset — e.g. uppercasing a point selection, which has no text to
    // change — must not wipe a pending redo_stack. Exercised through the
    // real command path (Alt-`) rather than a hand-built Transaction.
    let mut sim = AnyModeSimulator::new("test\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "test\n");

    // Point selection (anchor == head) at the start of the doc: no text to
    // uppercase, so this produces an empty changeset.
    sim.execute_command(CMD_SWITCH_TO_UPPERCASE).unwrap();
    assert_eq!(sim.state().unwrap().content(), "test\n");

    // The pending redo from the earlier undo must still be intact.
    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "");
}

#[test]
fn test_interleaved_undo_redo_undo_round_trips() {
    let mut sim = AnyModeSimulator::new("aaa\nbbb\n".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bbb\n");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "aaa\nbbb\n");

    sim.execute_command(CMD_REDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bbb\n");

    sim.execute_command(CMD_UNDO).unwrap();
    assert_eq!(sim.state().unwrap().content(), "aaa\nbbb\n");
}

// ============================================================================
// Open Line Tests (o, O)
// ============================================================================

#[test]
fn test_open_below() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Cursor at start of first line
    assert_eq!(sim.state().unwrap().cursor_position().0, 0);

    // Press 'o' should insert new line below and enter insert mode
    sim.execute_command(CMD_OPEN_BELOW).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().0, 1); // On new empty line
}

#[test]
fn test_open_above() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Move to second line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    assert_eq!(sim.state().unwrap().cursor_position().0, 1);

    // Press 'O' should insert new line above and enter insert mode
    sim.execute_command(CMD_OPEN_ABOVE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "line1\n\nline2");
    assert_eq!(state.cursor_position().0, 1); // On new empty line
}

// ============================================================================
// Join Lines Tests
// ============================================================================

#[test]
fn test_join_lines() {
    let mut sim = AnyModeSimulator::new("line1\nline2\nline3".to_string());

    // Join first two lines
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "line1 line2\nline3");
    assert_eq!(state.cursor_position().0, 0);
}

#[test]
fn test_join_lines_at_last_line() {
    let mut sim = AnyModeSimulator::new("line1\nline2".to_string());

    // Move to last line
    sim.execute_command(CMD_MOVE_DOWN).unwrap();
    assert_eq!(sim.state().unwrap().cursor_position().0, 1);

    // Try to join - should do nothing
    sim.execute_command(CMD_JOIN_LINES).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "line1\nline2");
}

// ============================================================================
// Indentation Tests
// ============================================================================

#[test]
fn test_indent_line() {
    let mut sim = AnyModeSimulator::new("hello\nworld".to_string());

    // Indent first line
    sim.execute_command(CMD_INDENT).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "  hello\nworld");
    // Cursor should move forward by 2
    assert_eq!(state.cursor_position().1, 2);
}

#[test]
fn test_dedent_line() {
    let mut sim = AnyModeSimulator::new("  hello\n    world".to_string());

    // Dedent first line (remove 2 spaces)
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello\n    world");
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_dedent_line_with_one_space() {
    let mut sim = AnyModeSimulator::new(" hello".to_string());

    // Dedent - should remove only 1 space
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello");
    assert_eq!(state.cursor_position().1, 0);
}

#[test]
fn test_dedent_line_no_spaces() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Dedent line with no leading spaces - should do nothing
    sim.execute_command(CMD_DEDENT).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "hello");
}

#[test]
fn test_multiple_indent() {
    let mut sim = AnyModeSimulator::new("code".to_string());

    // Indent twice
    sim.execute_command(CMD_INDENT).unwrap();
    sim.execute_command(CMD_INDENT).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "    code");
    assert_eq!(state.cursor_position().1, 4);
}

// ============================================================================
// Replace Character Tests
// ============================================================================

#[test]
fn test_replace_char() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.state().unwrap().content(), "hello");

    // Press 'r' then 'X' should replace 'h' with 'X'
    sim.execute_command("rX").unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "Xello");
    assert_eq!(sim.mode(), Mode::Normal); // Should stay in normal mode
}

// ============================================================================
// Change Command Tests
// ============================================================================

#[test]
fn test_change_selection() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.state().unwrap().content(), "hello");
    assert_eq!(sim.mode(), Mode::Normal);

    // Press 'c' should delete 'h' and enter insert mode
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "ello");
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().1, 0); // Cursor stays at start
}

#[test]
fn test_change_selection_noyank() {
    let mut sim = AnyModeSimulator::new("hello".to_string());

    // Cursor at start
    assert_eq!(sim.state().unwrap().content(), "hello");
    assert_eq!(sim.mode(), Mode::Normal);

    // Press Alt-c should delete 'h' and enter insert mode, without yanking
    sim.execute_command(CMD_CHANGE_SELECTION_NOYANK).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "ello");
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.cursor_position().1, 0); // Cursor stays at start
}

#[test]
fn test_change_selection_deletes_full_multichar_range() {
    // "world" as a forward range (anchor=6 < head=11) in "hello world"
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(
        state.content(),
        "hello ",
        "'c' must delete the full selection range, not just the head character"
    );
    assert_eq!(sim.mode(), Mode::Insert);
    // Selection collapses to the start of the deleted range before insert mode
    assert_eq!(state.cursor_position().1, 6);
}

#[test]
fn test_delete_selection_populates_register_for_paste() {
    let mut sim = AnyModeSimulator::new("abc".to_string());

    // 'd' deletes 'a' and should write it to the default register
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bc");

    // Move to end and paste - the deleted 'a' should come back
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "bca");
}

#[test]
fn test_change_selection_populates_register_for_paste() {
    let mut sim = AnyModeSimulator::new("abc".to_string());

    // 'c' deletes 'a' and should write it to the default register
    sim.execute_command(CMD_CHANGE).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.state().unwrap().content(), "bc");

    // Move to end and paste - the changed-away 'a' should come back
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "bca");
}

#[test]
fn test_change_selection_noyank_does_not_clobber_existing_register() {
    // Pre-populate the register with text DISTINCT from what Alt-c deletes
    // ('y' vs 'a'), so a later paste can only succeed if Alt-c left the
    // pre-existing register content alone (a fresh/empty register would
    // make a no-op paste pass trivially, which is what the old, weaker
    // version of this test did).
    let mut sim = AnyModeSimulator::new("y abc".to_string());

    // Yank 'y' into the default register
    sim.execute_command(CMD_YANK).unwrap();

    // Move onto 'a' and Alt-c it away - must not touch the register
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_CHANGE_SELECTION_NOYANK).unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.state().unwrap().content(), "y bc");

    // Paste before at document start must produce the ORIGINAL 'y', not
    // the 'a' that Alt-c just deleted
    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(
        state.content(),
        "yy bc",
        "Alt-c must leave a pre-existing register ('y') untouched"
    );
}

#[test]
fn test_delete_and_change_register_round_trip_exact_multichar_text() {
    // "world" as a forward range (anchor=6 < head=11) in "hello world"
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();

    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();
    assert_eq!(sim.state().unwrap().content(), "hello ");

    // Paste-after restores the exact multi-char deleted text, not just a
    // single character or some non-empty placeholder
    sim.execute_command(CMD_PASTE_AFTER).unwrap();
    assert_eq!(
        sim.state().unwrap().content(),
        "hello world",
        "'d' must round-trip the EXACT deleted multi-char text through 'p'"
    );

    // Same check for 'c', on a fresh selection
    let cursor = CursorPosition::new(0, 11).unwrap();
    let sel = Selection::new(CursorPosition::new(0, 6).unwrap(), cursor);
    let state = EditorState::new("hello world".to_string(), cursor, Some(sel)).unwrap();
    let mut sim = AnyModeSimulator::from_editor_state(&state);
    sim.execute_command(CMD_CHANGE).unwrap();
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.state().unwrap().content(), "hello ");

    sim.execute_command(CMD_PASTE_AFTER).unwrap();
    assert_eq!(
        sim.state().unwrap().content(),
        "hello world",
        "'c' must round-trip the EXACT deleted multi-char text through 'p'"
    );
}

#[test]
fn test_delete_selection_register_round_trips_through_paste_before() {
    // Same as the 'p' round trip, but exercising 'P' (paste_before), which
    // is a distinct code path (inserts at range.from() instead of head+1).
    let mut sim = AnyModeSimulator::new("abc".to_string());

    sim.execute_command(CMD_DELETE_SELECTION).unwrap(); // deletes 'a'
    assert_eq!(sim.state().unwrap().content(), "bc");

    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(state.content(), "abc");
}

#[test]
fn test_change_selection_multi_range_maps_cursor_through_earlier_deletions() {
    // Repro: doc "ab\ncd", cursor 0, 'C' (copy_selection_next_line) adds a
    // second cursor on the next line and makes it primary. 'c' must delete
    // BOTH single-char ranges and map the (still-primary) second cursor to
    // its correct post-deletion position - not a stale pre-transaction
    // offset that lands past the end of the document once the first
    // range's deletion has shifted everything after it left by one.
    //
    // Note: `enter_insert_mode` (pre-existing, unrelated to this fix, and
    // shared by every insert-entry command such as `a`/`i`/`o`/`O`)
    // collapses the selection to its primary range before Insert mode
    // starts, so only ONE cursor - the correctly mapped primary - survives
    // here. Fixing that shared single-cursor-insert-mode limitation is a
    // separate, larger change out of scope for #395/#396.
    let mut sim = AnyModeSimulator::new("ab\ncd".to_string());

    sim.execute_command(CMD_COPY_SELECTION_NEXT).unwrap();
    sim.execute_command(CMD_CHANGE).unwrap();

    let snapshot = sim.to_snapshot();
    assert_eq!(snapshot.content, "b\nd");
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(
        snapshot.selections.len(),
        1,
        "insert mode carries a single cursor (enter_insert_mode collapses to primary)"
    );
    assert_eq!(
        snapshot.selections[0].head, 2,
        "primary cursor must map through the FIRST range's deletion too, \
         landing on 'd' (position 2), not the stale pre-transaction offset 3"
    );

    // Typing at the (correctly mapped) cursor must land between the
    // newline and 'd', not after 'd'
    sim.execute_command("X").unwrap();
    assert_eq!(sim.state().unwrap().content(), "b\nXd");
}

#[test]
fn test_delete_selection_multi_range_maps_cursor_through_earlier_deletions() {
    // Same repro as above for 'd', which stays in Normal mode.
    let mut sim = AnyModeSimulator::new("ab\ncd".to_string());

    sim.execute_command(CMD_COPY_SELECTION_NEXT).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION).unwrap();

    let snapshot = sim.to_snapshot();
    assert_eq!(snapshot.content, "b\nd");
    assert_eq!(snapshot.selections.len(), 2);
    assert_eq!(snapshot.selections[0].head, 0);
    assert_eq!(snapshot.selections[1].head, 2);
    assert_eq!(snapshot.primary_idx, 1);
    assert_eq!(sim.mode(), Mode::Normal);
}

// ============================================================================
// Linewise Change Tests (x + c -> Open::Above, Helix 'xc')
// ============================================================================

#[test]
fn test_select_line_then_change_opens_blank_line() {
    let mut sim = AnyModeSimulator::new("one\ntwo\nthree".to_string());

    // 'x' selects the whole first line including its trailing newline
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    // 'c' on a linewise selection opens a blank line instead of a plain
    // mid-line insert
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "\ntwo\nthree");
    assert_eq!(state.cursor_position(), (0, 0));

    sim.execute_command("X").unwrap();
    assert_eq!(sim.state().unwrap().content(), "X\ntwo\nthree");
}

#[test]
fn test_select_line_then_change_noyank_also_opens_blank_line() {
    let mut sim = AnyModeSimulator::new("one\ntwo".to_string());

    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_CHANGE_SELECTION_NOYANK).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "\ntwo");
    assert_eq!(state.cursor_position(), (0, 0));
}

#[test]
fn test_select_line_middle_then_change_opens_blank_line_in_place() {
    let mut sim = AnyModeSimulator::new("one\ntwo\nthree".to_string());

    sim.execute_command(CMD_MOVE_DOWN).unwrap(); // onto "two"
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "one\n\nthree");
    assert_eq!(state.cursor_position(), (1, 0));
}

#[test]
fn test_change_non_linewise_selection_does_not_open_blank_line() {
    // A plain point cursor (no active selection) is never linewise - 'c'
    // must fall back to a normal mid-position insert, matching pre-existing
    // behavior (see `test_change_selection`).
    let mut sim = AnyModeSimulator::new("one\ntwo".to_string());

    sim.execute_command(CMD_CHANGE).unwrap();

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "ne\ntwo");
    assert_eq!(state.cursor_position(), (0, 0));
}

// ============================================================================
// Blackhole Register Tests ("_)
// ============================================================================

#[test]
fn test_delete_selection_blackhole_register_discards_text() {
    let mut sim = AnyModeSimulator::new("y abc".to_string());

    // Pre-populate the default register so a no-op paste can't pass trivially
    sim.execute_command(CMD_YANK).unwrap();

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command("\"_d").unwrap(); // blackhole-delete 'a'
    assert_eq!(sim.state().unwrap().content(), "y bc");

    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    assert_eq!(
        sim.state().unwrap().content(),
        "yy bc",
        "\"_d must not write the deleted text anywhere"
    );
}

#[test]
fn test_change_selection_blackhole_register_discards_text() {
    let mut sim = AnyModeSimulator::new("y abc".to_string());

    sim.execute_command(CMD_YANK).unwrap();

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command("\"_c").unwrap(); // blackhole-change 'a'
    assert_eq!(sim.mode(), Mode::Insert);
    sim.execute_command(CMD_ESCAPE).unwrap();
    assert_eq!(sim.state().unwrap().content(), "y bc");

    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    assert_eq!(
        sim.state().unwrap().content(),
        "yy bc",
        "\"_c must not write the deleted text anywhere"
    );
}

#[test]
fn test_delete_selection_named_register_round_trips_through_paste() {
    let mut sim = AnyModeSimulator::new("abc".to_string());

    sim.execute_command("\"ad").unwrap(); // delete 'a' into register 'a'
    assert_eq!(sim.state().unwrap().content(), "bc");

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command("\"ap").unwrap(); // paste register 'a' after cursor

    assert_eq!(sim.state().unwrap().content(), "bca");
}

#[test]
fn test_blackhole_change_on_linewise_selection_opens_blank_line_without_yanking() {
    // Item 1 (linewise xc -> Open::Above) composed with item 2 (blackhole
    // register): the blank-line-open behavior must still happen, and the
    // deleted line must still not be written to any register.
    let mut sim = AnyModeSimulator::new("y\none\ntwo".to_string());

    sim.execute_command(CMD_YANK).unwrap(); // pre-populate the default register with 'y'

    sim.execute_command(CMD_MOVE_DOWN).unwrap(); // onto "one"
    sim.execute_command(CMD_SELECT_LINE).unwrap();
    sim.execute_command("\"_c").unwrap(); // blackhole-change the whole line

    let state = sim.state().unwrap();
    assert_eq!(sim.mode(), Mode::Insert);
    assert_eq!(state.content(), "y\n\ntwo");
    assert_eq!(state.cursor_position(), (1, 0));

    sim.execute_command(CMD_ESCAPE).unwrap();
    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    assert_eq!(
        sim.state().unwrap().content(),
        "yy\n\ntwo",
        "\"_c must not write the deleted line anywhere"
    );
}

// ============================================================================
// Delete Without Yanking Tests (Alt-d)
// ============================================================================

#[test]
fn test_delete_selection_noyank_does_not_populate_register() {
    let mut sim = AnyModeSimulator::new("y abc".to_string());

    sim.execute_command(CMD_YANK).unwrap(); // register holds 'y'

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_DELETE_SELECTION_NOYANK).unwrap(); // Alt-d deletes 'a'
    assert_eq!(sim.mode(), Mode::Normal);
    assert_eq!(sim.state().unwrap().content(), "y bc");

    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command(CMD_PASTE_BEFORE).unwrap();

    assert_eq!(
        sim.state().unwrap().content(),
        "yy bc",
        "Alt-d must leave a pre-existing register ('y') untouched"
    );
}

// ============================================================================
// Repeat (.) of Register-Scoped Commands
// ============================================================================

#[test]
fn test_repeat_blackhole_delete() {
    // Regression: `.` must replay the exact blackhole-delete, not a stale
    // prior action - `"_d`'s key events were previously dropped entirely by
    // `cmd_to_key_events`, so nothing got recorded for it.
    let mut sim = AnyModeSimulator::new("abcabc".to_string());

    sim.execute_command("\"_d").unwrap(); // delete 'a' at index 0, discard it
    assert_eq!(sim.state().unwrap().content(), "bcabc");

    // "bcabc": b(0) c(1) a(2) b(3) c(4) - two moves lands on the 'a'
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_REPEAT).unwrap(); // '.' must replay "_d, deleting 'a' again

    assert_eq!(sim.state().unwrap().content(), "bcbc");

    // The register must still be empty - the repeat must not have yanked
    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command(CMD_PASTE_AFTER).unwrap();
    assert_eq!(
        sim.state().unwrap().content(),
        "bcbc",
        "repeated \"_d must not have written anything to the default register"
    );
}

#[test]
fn test_repeat_named_register_delete() {
    let mut sim = AnyModeSimulator::new("aXaX".to_string());

    sim.execute_command("\"rd").unwrap(); // delete 'a' into register 'r'
    assert_eq!(sim.state().unwrap().content(), "XaX");

    sim.execute_command(CMD_MOVE_RIGHT).unwrap();
    sim.execute_command(CMD_REPEAT).unwrap(); // '.' must replay "rd on the second 'a'

    assert_eq!(sim.state().unwrap().content(), "XX");

    // Register 'r' must hold the SECOND deleted 'a' (overwritten by the
    // repeat), proving the repeat actually re-executed "rd and not a stale
    // prior action
    sim.execute_command(CMD_GOTO_FILE_START).unwrap();
    sim.execute_command("\"rp").unwrap();
    assert_eq!(sim.state().unwrap().content(), "XaX");
}
