//! Select (extend) mode: `v` toggles it, motions extend the selection and
//! the editing commands of Helix leave it.

use super::execute_normal_mode_command_internal;
use crate::helix::commands::{
    CMD_ALIGN_SELECTIONS, CMD_DEDENT, CMD_DELETE_SELECTION, CMD_DELETE_SELECTION_NOYANK,
    CMD_ESCAPE, CMD_GOTO_FILE_START, CMD_GOTO_FIRST_NONWHITESPACE, CMD_GOTO_LAST_LINE,
    CMD_GOTO_LINE, CMD_GOTO_LINE_END, CMD_GOTO_LINE_START, CMD_HALF_PAGE_DOWN, CMD_HALF_PAGE_UP,
    CMD_INDENT, CMD_MATCH_BRACKETS, CMD_MOVE_DOWN, CMD_MOVE_LEFT, CMD_MOVE_LONG_WORD_BACKWARD,
    CMD_MOVE_LONG_WORD_END, CMD_MOVE_LONG_WORD_FORWARD, CMD_MOVE_RIGHT, CMD_MOVE_UP,
    CMD_MOVE_WORD_BACKWARD, CMD_MOVE_WORD_END, CMD_MOVE_WORD_FORWARD, CMD_PASTE_AFTER,
    CMD_PASTE_BEFORE, CMD_REPLACE_WITH_YANKED, CMD_SWITCH_CASE, CMD_SWITCH_CASE_ALT,
    CMD_SWITCH_TO_UPPERCASE, CMD_TOGGLE_COMMENTS, CMD_YANK,
};
use crate::helix::registry::parse_counted_command;
use crate::helix::simulator::{EditorMode, HelixSimulator, NormalMode};
use crate::security::UserError;
use helix_core::{Selection, ropey::RopeSlice};

/// Toggle Select mode (`v` command)
///
/// # Errors
///
/// Never fails: always returns `Ok(())`. The `Result` keeps the signature uniform
/// with the fallible commands that share the registry's function type.
pub fn toggle<M: EditorMode>(sim: &mut HelixSimulator<M>) -> Result<(), UserError> {
    sim.select_mode = !sim.select_mode;
    Ok(())
}

/// Run a Normal-mode command, applying the Select-mode rules around it
///
/// In Select mode a motion keeps the old anchor and moves only the head, as
/// the `extend_*` commands of `hx` do (`v l l` covers the characters moved
/// over). Commands that call `exit_select_mode` in `hx` return to Normal mode.
pub(super) fn execute(sim: &mut HelixSimulator<NormalMode>, cmd: &str) -> Result<(), UserError> {
    if !sim.select_mode {
        return execute_normal_mode_command_internal(sim, cmd);
    }

    let base = parse_counted_command(cmd).map_or(cmd, |(_, base)| base);
    let before = is_extending_motion(base).then(|| sim.selection.clone());
    let result = execute_normal_mode_command_internal(sim, cmd);
    if result.is_ok() {
        if let Some(before) = before {
            extend_from(sim, &before);
        } else if leaves_select_mode(base) {
            sim.select_mode = false;
        }
    }
    result
}

fn extend_from(sim: &mut HelixSimulator<NormalMode>, before: &Selection) {
    if before.len() != sim.selection.len() {
        return;
    }
    let slice: RopeSlice = sim.doc.slice(..);
    let ranges = before
        .ranges()
        .iter()
        .zip(sim.selection.ranges())
        .map(|(old, moved)| old.put_cursor(slice, moved.cursor(slice), true))
        .collect();
    sim.selection = Selection::new(ranges, sim.selection.primary_index());
}

fn is_extending_motion(base: &str) -> bool {
    matches!(
        base,
        CMD_MOVE_LEFT
            | CMD_MOVE_DOWN
            | CMD_MOVE_UP
            | CMD_MOVE_RIGHT
            | CMD_MOVE_WORD_FORWARD
            | CMD_MOVE_WORD_BACKWARD
            | CMD_MOVE_WORD_END
            | CMD_MOVE_LONG_WORD_FORWARD
            | CMD_MOVE_LONG_WORD_BACKWARD
            | CMD_MOVE_LONG_WORD_END
            | CMD_GOTO_FILE_START
            | CMD_GOTO_LAST_LINE
            | CMD_GOTO_LINE_START
            | CMD_GOTO_LINE_END
            | CMD_GOTO_FIRST_NONWHITESPACE
            | CMD_GOTO_LINE
            | CMD_HALF_PAGE_UP
            | CMD_HALF_PAGE_DOWN
            | CMD_MATCH_BRACKETS
    ) || is_find_command(base)
}

fn is_find_command(base: &str) -> bool {
    let mut chars = base.chars();
    matches!(chars.next(), Some('f' | 't' | 'F' | 'T')) && chars.count() == 1
}

fn leaves_select_mode(base: &str) -> bool {
    let command = crate::helix::commands::normalize_command_id(base);
    matches!(
        command.as_ref(),
        CMD_ESCAPE
            | CMD_DELETE_SELECTION
            | CMD_DELETE_SELECTION_NOYANK
            | CMD_YANK
            | CMD_PASTE_AFTER
            | CMD_PASTE_BEFORE
            | CMD_REPLACE_WITH_YANKED
            | CMD_INDENT
            | CMD_DEDENT
            | CMD_TOGGLE_COMMENTS
            | CMD_ALIGN_SELECTIONS
            | CMD_SWITCH_CASE
            | CMD_SWITCH_CASE_ALT
            | CMD_SWITCH_TO_UPPERCASE
            | "\"d"
            | "\"y"
            | "\"p"
            | "\"P"
            | "\"R"
    ) || is_replace_char(base)
        || is_surround_command(base)
}

fn is_replace_char(base: &str) -> bool {
    base.starts_with('r') && base.chars().count() == 2
}

fn is_surround_command(base: &str) -> bool {
    ["ms", "md", "mr"].iter().any(|prefix| {
        base.strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty())
    })
}
