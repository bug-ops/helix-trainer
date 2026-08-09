//! Command execution and dispatch

pub mod clipboard;
pub mod editing;
pub mod movement;
pub mod search;
pub mod selection;
pub mod view;

use super::{AnyModeSimulator, HelixSimulator, InsertMode, Mode, NormalMode};
use crate::helix::commands::*;
use crate::helix::repeat::is_repeatable_command;
use crate::security::UserError;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn cmd_to_key_events(cmd: &str) -> Vec<KeyEvent> {
    if cmd == CMD_GOTO_FILE_START {
        return vec![
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
        ];
    }

    if cmd == CMD_ESCAPE {
        return vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)];
    }
    if cmd == CMD_BACKSPACE {
        return vec![KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_LEFT {
        return vec![KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_RIGHT {
        return vec![KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_UP {
        return vec![KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)];
    }
    if cmd == CMD_ARROW_DOWN {
        return vec![KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)];
    }

    if cmd.starts_with('r') && cmd.len() == 2 {
        let ch = cmd.chars().nth(1).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ];
    }

    if (cmd.starts_with('f')
        || cmd.starts_with('t')
        || cmd.starts_with('F')
        || cmd.starts_with('T'))
        && cmd.len() == 2
    {
        let prefix = cmd.chars().next().unwrap();
        let ch = cmd.chars().nth(1).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char(prefix), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ];
    }

    if cmd.starts_with('g') && cmd.len() == 2 {
        let ch = cmd.chars().nth(1).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ];
    }

    if cmd.starts_with("ms") && cmd.len() == 3 {
        let ch = cmd.chars().nth(2).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ];
    }

    if cmd.starts_with("md") && cmd.len() == 3 {
        let ch = cmd.chars().nth(2).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        ];
    }

    if cmd.starts_with("mr") && cmd.len() == 4 {
        let from_ch = cmd.chars().nth(2).unwrap();
        let to_ch = cmd.chars().nth(3).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(from_ch), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(to_ch), KeyModifiers::NONE),
        ];
    }

    if cmd == CMD_MATCH_BRACKETS {
        return vec![
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
        ];
    }

    if cmd.starts_with("ma") && cmd.len() == 3 {
        let obj = cmd.chars().nth(2).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(obj), KeyModifiers::NONE),
        ];
    }

    if cmd.starts_with("mi") && cmd.len() == 3 {
        let obj = cmd.chars().nth(2).unwrap();
        return vec![
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
            KeyEvent::new(KeyCode::Char(obj), KeyModifiers::NONE),
        ];
    }

    if cmd == CMD_FLIP_SELECTIONS {
        return vec![KeyEvent::new(KeyCode::Char(';'), KeyModifiers::ALT)];
    }

    if cmd.len() == 1
        && let Some(ch) = cmd.chars().next()
    {
        vec![KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)]
    } else {
        Vec::new()
    }
}

fn is_insert_command(cmd: &str) -> bool {
    cmd == CMD_INSERT
        || cmd == CMD_APPEND
        || cmd == CMD_INSERT_LINE_START
        || cmd == CMD_APPEND_LINE_END
        || cmd == CMD_OPEN_BELOW
        || cmd == CMD_OPEN_ABOVE
        || cmd == CMD_CHANGE
}

fn execute_insert_mode_command_internal(
    sim: &mut HelixSimulator<InsertMode>,
    cmd: &str,
) -> Result<(), UserError> {
    if cmd == CMD_ESCAPE {
        if !sim.is_repeating {
            let action = sim.repeat_buffer.insert_recorder_mut().finish();
            sim.repeat_buffer.set_last_action(action);
        }
        Ok(())
    } else if cmd == CMD_BACKSPACE {
        sim.backspace()
    } else if cmd == CMD_ARROW_LEFT {
        let result = movement::move_left(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Left);
        }
        result
    } else if cmd == CMD_ARROW_RIGHT {
        let result = movement::move_right(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Right);
        }
        result
    } else if cmd == CMD_ARROW_UP {
        let result = movement::move_up(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Up);
        }
        result
    } else if cmd == CMD_ARROW_DOWN {
        let result = movement::move_down(sim, 1);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer
                .insert_recorder_mut()
                .record_movement(crate::helix::repeat::Movement::Down);
        }
        result
    } else {
        let result = sim.insert_text(cmd);
        if result.is_ok() && !sim.is_repeating {
            for ch in cmd.chars() {
                sim.repeat_buffer.insert_recorder_mut().record_char(ch);
            }
        }
        result
    }
}

pub(super) fn execute_normal_mode_command_internal(
    sim: &mut HelixSimulator<NormalMode>,
    cmd: &str,
) -> Result<(), UserError> {
    use crate::helix::registry::normal_registry;

    let registry = normal_registry();

    if cmd.len() == 2 {
        let first = cmd.chars().next().unwrap();
        let second = cmd.chars().nth(1).unwrap();

        match first {
            'r' => {
                sim.replace_char(second)?;
                return Ok(());
            }
            'f' => {
                movement::find_next_char(sim, second, 1)?;
                return Ok(());
            }
            'F' => {
                movement::find_prev_char(sim, second, 1)?;
                return Ok(());
            }
            't' => {
                movement::till_next_char(sim, second, 1)?;
                return Ok(());
            }
            'T' => {
                movement::till_prev_char(sim, second, 1)?;
                return Ok(());
            }
            _ => {}
        }
    }

    if cmd.len() == 3 && cmd.starts_with("ms") {
        let surround_char = cmd.chars().nth(2).unwrap();
        editing::surround_selection(sim, surround_char)?;
        return Ok(());
    }

    if cmd.len() == 3 && cmd.starts_with("md") {
        let surround_char = cmd.chars().nth(2).unwrap();
        editing::delete_surround(sim, surround_char)?;
        return Ok(());
    }

    if cmd.len() == 4 && cmd.starts_with("mr") {
        let from_char = cmd.chars().nth(2).unwrap();
        let to_char = cmd.chars().nth(3).unwrap();
        editing::replace_surround(sim, from_char, to_char)?;
        return Ok(());
    }

    if cmd.len() == 3 && cmd.starts_with("ma") {
        let obj = cmd.chars().nth(2).unwrap();
        editing::select_around_textobject(sim, obj)?;
        return Ok(());
    }

    if cmd.len() == 3 && cmd.starts_with("mi") {
        let obj = cmd.chars().nth(2).unwrap();
        editing::select_inside_textobject(sim, obj)?;
        return Ok(());
    }

    if cmd.starts_with(CMD_SELECT_REGISTER) {
        // Destructure via `chars()` rather than `len() == 3` + `.nth(k).unwrap()`:
        // `len()` counts bytes, so a multi-byte register char (e.g. `"é`,
        // 1 char but 2 bytes) would make `len() == 3` true for only 2 chars,
        // and `.nth(2)` would then panic on `None`. This form only matches
        // when there are exactly 3 chars total (quote + register + op).
        let mut chars = cmd.chars();
        chars.next(); // the leading '"', already confirmed by starts_with
        if let (Some(register), Some(op), None) = (chars.next(), chars.next(), chars.next()) {
            return match op {
                'y' => clipboard::yank_to_register(sim, Some(register)),
                'p' => clipboard::paste_after_from_register(sim, Some(register)),
                'P' => clipboard::paste_before_from_register(sim, Some(register)),
                'R' => editing::replace_with_yanked_from_register(sim, Some(register)),
                _ => Err(UserError::command_failed(format!(
                    "unknown register operation '{}'",
                    op
                ))),
            };
        }
    }

    if cmd.starts_with(CMD_COMMAND_LINE) {
        return crate::helix::simulator::command_line::CommandLine::parse(cmd)?.execute(sim);
    }

    if cmd == CMD_ESCAPE {
        return Ok(());
    }

    if cmd == CMD_REPEAT {
        return Ok(());
    }

    // Regex-selection commands ("s <pattern>" / "S <pattern>"), assembled
    // atomically by `RegexPromptPending`. Splitting only the leading space
    // (not `split(' ')`) since patterns can themselves contain spaces. The
    // `s`/`S` prefix alone (no space) cannot collide with the
    // `cmd.len() == 2` f/t/r/F/T block above. Bypasses the registry (whose
    // `CommandHandler` type cannot carry a pattern argument), the same way
    // register-scoped ops and `:`-command-line invocations do above.
    if let Some(pattern) = cmd
        .strip_prefix(CMD_SELECT_REGEX)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        return selection::select_regex(sim, pattern);
    }
    if let Some(pattern) = cmd
        .strip_prefix(CMD_SPLIT_SELECTION)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        return selection::split_selection(sim, pattern);
    }

    registry.execute(sim, cmd)?;

    Ok(())
}

fn record_command_if_needed_normal(
    sim: &mut HelixSimulator<NormalMode>,
    key_events: &[KeyEvent],
    mode_before: Mode,
    entering_insert: bool,
    entry_command: Option<String>,
) {
    let should_record =
        !key_events.is_empty() && !sim.is_repeating && key_events.iter().all(is_repeatable_command);

    if should_record {
        sim.repeat_buffer
            .record_command(key_events.to_vec(), mode_before);
    }

    if entering_insert {
        sim.repeat_buffer.insert_recorder_mut().start(entry_command);
    }
}

pub(super) fn execute_command_any_mode(
    sim: &mut AnyModeSimulator,
    cmd: &str,
) -> Result<(), UserError> {
    if cmd == CMD_REPEAT {
        return sim.execute_repeat();
    }

    // `q`/`Q` only mean "toggle macro recording" / "replay macro" while in
    // Normal mode. The early return sits above the `match old_sim` below
    // (which is what would otherwise tell us the mode), so it tests the
    // mode itself. In Insert mode, both fall through unhandled here and are
    // inserted as literal characters by the Insert arm further down - this
    // is how a single "q"/"Q" command string serves both the live Normal-mode
    // keypress (routed here via `base.rs`) and literal Insert-mode typing
    // (routed here via `handle_insert_mode_input`, which never touches
    // `base.rs`).
    if cmd == CMD_TOGGLE_MACRO_RECORDING && matches!(sim, AnyModeSimulator::Normal(_)) {
        if let AnyModeSimulator::Normal(normal_sim) = sim {
            normal_sim.toggle_macro_recording();
        }
        return Ok(());
    }

    if cmd == CMD_REPLAY_MACRO
        && let AnyModeSimulator::Normal(normal_sim) = sim
    {
        // `Q` while recording is a no-op, not a replay: the recording tap
        // below is skipped while `execute_macro_replay` sets `is_replaying`,
        // so replayed effects would apply to the document while silently
        // NOT being captured into the macro currently being recorded - a
        // training tool producing a macro that doesn't match what the user
        // watched happen on screen. See `macro_recorder.rs` module docs.
        if normal_sim.is_recording_macro() {
            return Ok(());
        }
        return sim.execute_macro_replay();
        // Insert mode: falls through, 'Q' is inserted as literal text.
    }

    let placeholder = AnyModeSimulator::Normal(HelixSimulator::new(String::new()));
    let old_sim = std::mem::replace(sim, placeholder);

    let (result, new_sim) = match old_sim {
        AnyModeSimulator::Normal(mut normal_sim) => {
            let key_events = cmd_to_key_events(cmd);
            let is_insert_cmd = is_insert_command(cmd);
            let should_start_recording = !normal_sim.is_repeating && is_insert_cmd;

            let result = execute_normal_mode_command_internal(&mut normal_sim, cmd);

            if result.is_ok() {
                let entry_cmd = if should_start_recording {
                    Some(cmd.to_string())
                } else {
                    None
                };

                record_command_if_needed_normal(
                    &mut normal_sim,
                    &key_events,
                    Mode::Normal,
                    should_start_recording,
                    entry_cmd,
                );

                if is_insert_cmd {
                    (
                        result,
                        AnyModeSimulator::Insert(normal_sim.enter_insert_mode()),
                    )
                } else {
                    (result, AnyModeSimulator::Normal(normal_sim))
                }
            } else {
                (result, AnyModeSimulator::Normal(normal_sim))
            }
        }
        AnyModeSimulator::Insert(insert_sim)
            if cmd.len() > 1 && cmd.starts_with(CMD_COMMAND_LINE) =>
        {
            // Command-line strings are assembled by `CommandLinePending` in
            // Normal mode only; guarded here too so an *assembled* `:`-prefixed
            // string (e.g. ":goto 3") can never be inserted as literal text.
            // `cmd.len() > 1` matters: insert mode delivers every typed
            // character as its own one-char command, so without this a
            // literal ':' keystroke (len 1) would hit this arm and be
            // rejected instead of inserted.
            let result = Err(UserError::command_failed(
                "command-line is not available in insert mode",
            ));
            (result, AnyModeSimulator::Insert(insert_sim))
        }
        AnyModeSimulator::Insert(mut insert_sim) => {
            let exiting = cmd == CMD_ESCAPE;
            let result = execute_insert_mode_command_internal(&mut insert_sim, cmd);

            if result.is_ok() && exiting {
                (
                    result,
                    AnyModeSimulator::Normal(insert_sim.exit_insert_mode()),
                )
            } else {
                (result, AnyModeSimulator::Insert(insert_sim))
            }
        }
    };

    // Put the new simulator back
    *sim = new_sim;

    // Single tap covering all three dispatch arms above (Normal, Insert,
    // and the insert-mode ':' rejection). Only successful commands are
    // recorded - a macro is a record of what actually happened, and
    // replaying a command that failed would reproduce the failure at best.
    // `record_macro_command` is itself a no-op unless recording is active
    // and this isn't itself part of a replay.
    if result.is_ok() {
        sim.record_macro_command(cmd);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests for is_insert_command()
    mod is_insert_command_tests {
        use super::*;

        #[test]
        fn test_insert_command_insert() {
            assert!(is_insert_command(CMD_INSERT));
        }

        #[test]
        fn test_insert_command_append() {
            assert!(is_insert_command(CMD_APPEND));
        }

        #[test]
        fn test_insert_command_insert_line_start() {
            assert!(is_insert_command(CMD_INSERT_LINE_START));
        }

        #[test]
        fn test_insert_command_append_line_end() {
            assert!(is_insert_command(CMD_APPEND_LINE_END));
        }

        #[test]
        fn test_insert_command_open_below() {
            assert!(is_insert_command(CMD_OPEN_BELOW));
        }

        #[test]
        fn test_insert_command_open_above() {
            assert!(is_insert_command(CMD_OPEN_ABOVE));
        }

        #[test]
        fn test_insert_command_change() {
            assert!(is_insert_command(CMD_CHANGE));
        }

        #[test]
        fn test_insert_command_movement_not_insert() {
            assert!(!is_insert_command(CMD_MOVE_LEFT));
            assert!(!is_insert_command(CMD_MOVE_RIGHT));
            assert!(!is_insert_command(CMD_MOVE_UP));
            assert!(!is_insert_command(CMD_MOVE_DOWN));
        }

        #[test]
        fn test_insert_command_editing_not_insert() {
            assert!(!is_insert_command(CMD_DELETE_SELECTION));
            assert!(!is_insert_command(CMD_YANK));
            assert!(!is_insert_command(CMD_PASTE_AFTER));
        }

        #[test]
        fn test_insert_command_escape_not_insert() {
            assert!(!is_insert_command(CMD_ESCAPE));
        }

        #[test]
        fn test_insert_command_random_string() {
            assert!(!is_insert_command("xyz"));
            assert!(!is_insert_command(""));
        }
    }

    // Unit tests for cmd_to_key_events()
    mod cmd_to_key_events_tests {
        use super::*;

        #[test]
        fn test_cmd_to_key_events_goto_file_start() {
            let events = cmd_to_key_events(CMD_GOTO_FILE_START);
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].code, KeyCode::Char('g'));
            assert_eq!(events[1].code, KeyCode::Char('g'));
        }

        #[test]
        fn test_cmd_to_key_events_escape() {
            let events = cmd_to_key_events(CMD_ESCAPE);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Esc);
        }

        #[test]
        fn test_cmd_to_key_events_backspace() {
            let events = cmd_to_key_events(CMD_BACKSPACE);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Backspace);
        }

        #[test]
        fn test_cmd_to_key_events_arrow_keys() {
            let left = cmd_to_key_events(CMD_ARROW_LEFT);
            assert_eq!(left.len(), 1);
            assert_eq!(left[0].code, KeyCode::Left);

            let right = cmd_to_key_events(CMD_ARROW_RIGHT);
            assert_eq!(right.len(), 1);
            assert_eq!(right[0].code, KeyCode::Right);

            let up = cmd_to_key_events(CMD_ARROW_UP);
            assert_eq!(up.len(), 1);
            assert_eq!(up[0].code, KeyCode::Up);

            let down = cmd_to_key_events(CMD_ARROW_DOWN);
            assert_eq!(down.len(), 1);
            assert_eq!(down[0].code, KeyCode::Down);
        }

        #[test]
        fn test_cmd_to_key_events_replace_command() {
            let events = cmd_to_key_events("rx");
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].code, KeyCode::Char('r'));
            assert_eq!(events[1].code, KeyCode::Char('x'));
        }

        #[test]
        fn test_cmd_to_key_events_single_char() {
            let events = cmd_to_key_events("h");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].code, KeyCode::Char('h'));

            let events2 = cmd_to_key_events("x");
            assert_eq!(events2.len(), 1);
            assert_eq!(events2[0].code, KeyCode::Char('x'));
        }

        #[test]
        fn test_cmd_to_key_events_empty_string() {
            let events = cmd_to_key_events("");
            assert!(events.is_empty());
        }

        #[test]
        fn test_cmd_to_key_events_unknown_multi_char() {
            let events = cmd_to_key_events("xyz");
            assert!(events.is_empty());
        }

        /// Named regression test for S3/Q2: register ops and command-line
        /// invocations are deliberately not `.`-repeatable. `cmd_to_key_events`
        /// has no dedicated arm for either shape, so both fall through to the
        /// generic "unmatched, len != 1" branch and produce no key events -
        /// `record_command_if_needed_normal` then skips recording entirely
        /// (an empty `key_events` slice is never recorded), so `.` after
        /// `"ay` or `:goto 3` is a no-op, matching Helix (`.` repeats changes,
        /// not yanks or navigation).
        #[test]
        fn test_cmd_to_key_events_register_op_and_command_line_are_not_repeatable() {
            assert!(cmd_to_key_events("\"ay").is_empty());
            assert!(cmd_to_key_events(":goto 3").is_empty());
        }

        #[test]
        fn test_cmd_to_key_events_all_modifiers_none() {
            let events = cmd_to_key_events("a");
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].modifiers, KeyModifiers::NONE);
        }

        #[test]
        fn test_cmd_to_key_events_text_object_around_word() {
            let events = cmd_to_key_events("maw");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('a'));
            assert_eq!(events[2].code, KeyCode::Char('w'));
        }

        #[test]
        fn test_cmd_to_key_events_text_object_inside_word() {
            let events = cmd_to_key_events("miw");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('i'));
            assert_eq!(events[2].code, KeyCode::Char('w'));
        }

        #[test]
        fn test_cmd_to_key_events_text_object_around_big_word() {
            let events = cmd_to_key_events("maW");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('a'));
            assert_eq!(events[2].code, KeyCode::Char('W'));
        }

        #[test]
        fn test_cmd_to_key_events_text_object_around_paren() {
            let events = cmd_to_key_events("ma(");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('a'));
            assert_eq!(events[2].code, KeyCode::Char('('));
        }

        #[test]
        fn test_cmd_to_key_events_text_object_inside_bracket() {
            let events = cmd_to_key_events("mi[");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('i'));
            assert_eq!(events[2].code, KeyCode::Char('['));
        }

        #[test]
        fn test_cmd_to_key_events_text_object_around_quote() {
            let events = cmd_to_key_events("ma\"");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('a'));
            assert_eq!(events[2].code, KeyCode::Char('"'));
        }

        #[test]
        fn test_cmd_to_key_events_text_object_inside_paragraph() {
            let events = cmd_to_key_events("mip");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('m'));
            assert_eq!(events[1].code, KeyCode::Char('i'));
            assert_eq!(events[2].code, KeyCode::Char('p'));
        }
    }

    // Register and command-line dispatch regression tests
    mod register_and_command_line_dispatch_tests {
        use super::*;
        use crate::helix::simulator::{AnyModeSimulator, HelixSimulator, NormalMode};

        #[test]
        fn insert_mode_literal_colon_is_inserted_not_rejected() {
            // Regression: the insert-mode ':' guard must only reject an
            // *assembled* command-line string (len > 1), not a single typed
            // ':' character, which insert mode delivers as its own one-char
            // command.
            let mut sim = AnyModeSimulator::Insert(
                HelixSimulator::<NormalMode>::new(String::new()).enter_insert_mode(),
            );
            sim.execute_command(":").unwrap();
            assert_eq!(sim.state().unwrap().content(), ":");
        }

        #[test]
        fn insert_mode_assembled_command_line_is_rejected() {
            let mut sim = AnyModeSimulator::Insert(
                HelixSimulator::<NormalMode>::new(String::new()).enter_insert_mode(),
            );
            assert!(sim.execute_command(":goto 3").is_err());
        }

        /// Named-register round-trip for `P` (paste before), not just `y`/`p`.
        #[test]
        fn register_paste_before_with_explicit_named_register() {
            let mut sim: HelixSimulator<NormalMode> =
                HelixSimulator::new("hello world".to_string());
            sim.selection = helix_core::Selection::single(0, 5); // "hello"

            execute_normal_mode_command_internal(&mut sim, "\"ay").unwrap(); // register a = "hello"

            sim.selection = helix_core::Selection::point(6); // on 'w' in "world"
            execute_normal_mode_command_internal(&mut sim, "\"aP").unwrap();

            assert_eq!(sim.doc.to_string(), "hello helloworld");
        }

        /// Named-register round-trip for `R` (replace selection with
        /// register content), not just `y`/`p`.
        #[test]
        fn register_replace_with_explicit_named_register() {
            let mut sim: HelixSimulator<NormalMode> =
                HelixSimulator::new("hello world".to_string());
            sim.selection = helix_core::Selection::single(0, 5); // "hello"
            execute_normal_mode_command_internal(&mut sim, "\"ay").unwrap(); // register a = "hello"

            sim.selection = helix_core::Selection::single(6, 11); // "world"
            execute_normal_mode_command_internal(&mut sim, "\"aR").unwrap();

            assert_eq!(sim.doc.to_string(), "hello hello");
        }

        #[test]
        fn register_dispatch_handles_non_ascii_register_without_panicking() {
            // Regression: byte-length gating on a char index used to panic
            // for a multi-byte register char (e.g. "é" is 1 char, 2 bytes).
            // A well-formed 3-char command (quote + register + op) with a
            // non-ASCII register succeeds like any other register letter.
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
            assert!(execute_normal_mode_command_internal(&mut sim, "\"éy").is_ok());

            // An unsupported op with the same non-ASCII register is a clean
            // error, not a panic.
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
            assert!(execute_normal_mode_command_internal(&mut sim, "\"éz").is_err());
        }

        #[test]
        fn register_dispatch_does_not_panic_on_malformed_length() {
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
            // Too short: just the prefix, no register/op.
            assert!(execute_normal_mode_command_internal(&mut sim, "\"").is_err());
            // Too short: prefix + register, missing op - also a multi-byte
            // register char (2 chars, 3 bytes), the original panic trigger.
            assert!(execute_normal_mode_command_internal(&mut sim, "\"é").is_err());
            // Too long: register + op + trailing char.
            assert!(execute_normal_mode_command_internal(&mut sim, "\"aay").is_err());
        }
    }

    // Regex-selection ("s <pattern>" / "S <pattern>") dispatch tests
    mod regex_selection_dispatch_tests {
        use super::*;
        use crate::helix::simulator::{HelixSimulator, NormalMode};
        use helix_core::Selection;

        #[test]
        fn dispatches_select_regex() {
            let mut sim: HelixSimulator<NormalMode> =
                HelixSimulator::new("foo bar foo".to_string());
            sim.selection = Selection::single(0, 11);

            execute_normal_mode_command_internal(&mut sim, "s foo").unwrap();

            assert_eq!(sim.selection.len(), 2);
        }

        #[test]
        fn dispatches_split_selection() {
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("a,b,c".to_string());
            sim.selection = Selection::single(0, 5);

            execute_normal_mode_command_internal(&mut sim, "S ,").unwrap();

            assert_eq!(sim.selection.len(), 3);
        }

        /// Pattern containing a literal space must survive intact - this is
        /// exactly why dispatch splits only the leading space rather than
        /// using `split(' ')` or `split_whitespace()`.
        #[test]
        fn pattern_with_space_is_preserved() {
            let mut sim: HelixSimulator<NormalMode> =
                HelixSimulator::new("foo bar foo bar".to_string());
            sim.selection = Selection::single(0, 15);

            execute_normal_mode_command_internal(&mut sim, "s foo bar").unwrap();

            assert_eq!(sim.selection.len(), 2);
        }

        #[test]
        fn invalid_regex_returns_err_not_panic() {
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
            sim.selection = Selection::single(0, 5);

            assert!(execute_normal_mode_command_internal(&mut sim, "s (").is_err());
        }

        /// A bare "s"/"S" (no trailing space - never emitted by
        /// `RegexPromptPending`, which cancels on an empty buffer) falls
        /// through to the registry, which no longer has these registered
        /// (see `registry/definitions/selection.rs`), so it errors instead
        /// of silently no-op'ing like the old stub.
        #[test]
        fn bare_s_without_pattern_is_unknown_command() {
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("hello".to_string());
            assert!(execute_normal_mode_command_internal(&mut sim, "s").is_err());
            assert!(execute_normal_mode_command_internal(&mut sim, "S").is_err());
        }
    }
}
