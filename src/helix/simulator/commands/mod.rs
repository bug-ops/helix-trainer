//! Command execution and dispatch

pub mod clipboard;
pub mod editing;
pub mod movement;
pub mod search;
pub mod select_mode;
pub mod selection;
pub mod view;

use super::{
    AnyModeSimulator, FindDirection, FindType, HelixSimulator, InsertCommand, InsertEntryCommand,
    InsertMode, Mode, NormalMode,
};
use crate::helix::commands::{
    CMD_ARROW_DOWN, CMD_ARROW_LEFT, CMD_ARROW_RIGHT, CMD_ARROW_UP, CMD_BACKSPACE, CMD_COMMAND_LINE,
    CMD_ESCAPE, CMD_FLIP_SELECTIONS, CMD_GOTO_FILE_START, CMD_GOTO_LINE, CMD_KEEP_MATCHING,
    CMD_MATCH_BRACKETS, CMD_REMOVE_MATCHING, CMD_REPEAT, CMD_REPLAY_MACRO, CMD_SEARCH_BACKWARD,
    CMD_SEARCH_FORWARD, CMD_SELECT_REGEX, CMD_SELECT_REGISTER, CMD_SPLIT_SELECTION,
    CMD_TOGGLE_MACRO_RECORDING,
};
use crate::helix::error::SimulatorError;
use crate::helix::registry::{
    CountBehavior, NativeCounted, normal_registry, parse_counted_command,
};
use crate::helix::repeat::is_repeatable_command;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::num::NonZeroUsize;

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

    let chars: Vec<char> = cmd.chars().collect();
    let plain = |ch: char| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);

    if cmd == CMD_MATCH_BRACKETS {
        return vec![plain('m'), plain('m')];
    }

    if cmd == CMD_FLIP_SELECTIONS {
        return vec![KeyEvent::new(KeyCode::Char(';'), KeyModifiers::ALT)];
    }

    // Named register prefix ("<reg><op>, e.g. `"_d`, `"ay`) and every other
    // argument-taking command: matched on chars, never bytes, so non-ASCII
    // arguments (`fé`, `r😀`) produce key events too.
    match chars.as_slice() {
        [prefix @ ('r' | 'f' | 't' | 'F' | 'T' | 'g'), arg] => vec![plain(*prefix), plain(*arg)],
        ['m', kind @ ('s' | 'd' | 'a' | 'i'), arg] => vec![plain('m'), plain(*kind), plain(*arg)],
        ['m', 'r', from, to] => vec![plain('m'), plain('r'), plain(*from), plain(*to)],
        ['"', register, op] => vec![plain('"'), plain(*register), plain(*op)],
        [ch] => vec![plain(*ch)],
        _ => Vec::new(),
    }
}

fn execute_insert_mode_command_internal(
    sim: &mut HelixSimulator<InsertMode>,
    cmd: &str,
) -> Result<(), SimulatorError> {
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
    } else if let Some(command) = InsertCommand::parse(cmd) {
        let result = sim.apply_insert_command(command);
        if result.is_ok() && !sim.is_repeating {
            sim.repeat_buffer.insert_recorder_mut().record_edit(command);
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

/// Apply `base` `count` times according to its [`CountBehavior`].
///
/// The single place a count prefix is interpreted: `Repeat` runs the command
/// `count` times, `Ignore` once, and `Native` hands the count to the command
/// as a `{count}{base}` string (`3fa`, `2G`, `2gg`).
pub(super) fn execute_counted_any_mode(
    sim: &mut AnyModeSimulator,
    base: &str,
    count: NonZeroUsize,
) -> Result<(), SimulatorError> {
    match normal_registry().count_behavior(base) {
        CountBehavior::Repeat => {
            for _ in 0..count.get() {
                execute_command_any_mode(sim, base)?;
                // A command that entered Insert mode must not run again as typed text.
                if matches!(sim, AnyModeSimulator::Insert(_)) {
                    break;
                }
            }
            Ok(())
        }
        CountBehavior::Ignore => execute_command_any_mode(sim, base),
        CountBehavior::Native => execute_command_any_mode(sim, &format!("{count}{base}")),
    }
}

/// Run a `Native` counted command (`3fa`, `2G`, `2gg`) in Normal mode.
fn execute_native_counted(
    sim: &mut HelixSimulator<NormalMode>,
    count: NonZeroUsize,
    base: &str,
) -> Result<(), SimulatorError> {
    match NativeCounted::parse(base) {
        Some(NativeCounted::GotoLine) => {
            movement::goto_line(sim, count);
            Ok(())
        }
        Some(NativeCounted::Find {
            find_type,
            direction,
            ch,
        }) => {
            let count = count.get();
            match (find_type, direction) {
                (FindType::Find, FindDirection::Forward) => {
                    movement::find_next_char(sim, ch, count)
                }
                (FindType::Find, FindDirection::Backward) => {
                    movement::find_prev_char(sim, ch, count)
                }
                (FindType::Till, FindDirection::Forward) => {
                    movement::till_next_char(sim, ch, count)
                }
                (FindType::Till, FindDirection::Backward) => {
                    movement::till_prev_char(sim, ch, count)
                }
            }
        }
        None => Err(SimulatorError::CountNotAccepted),
    }
}

pub(super) fn execute_normal_mode_command_internal(
    sim: &mut HelixSimulator<NormalMode>,
    cmd: &str,
) -> Result<(), SimulatorError> {
    let registry = normal_registry();

    if let Some((count, base)) = parse_counted_command(cmd) {
        return execute_native_counted(sim, count, base);
    }

    // Bare `G` is `goto_line`, a no-op without a count in helix-term 25.07.1.
    if cmd == CMD_GOTO_LINE {
        return Ok(());
    }

    let chars: Vec<char> = cmd.chars().collect();
    match chars.as_slice() {
        ['r', ch] => return sim.replace_char(*ch),
        ['f', ch] => return movement::find_next_char(sim, *ch, 1),
        ['F', ch] => return movement::find_prev_char(sim, *ch, 1),
        ['t', ch] => return movement::till_next_char(sim, *ch, 1),
        ['T', ch] => return movement::till_prev_char(sim, *ch, 1),
        ['m', 's', ch] => return editing::surround_selection(sim, *ch),
        ['m', 'd', ch] => return editing::delete_surround(sim, *ch),
        ['m', 'r', from, to] => return editing::replace_surround(sim, *from, *to),
        ['m', 'a', obj] => return editing::select_around_textobject(sim, *obj),
        ['m', 'i', obj] => return editing::select_inside_textobject(sim, *obj),
        _ => {}
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
                'd' => editing::delete_selection_from_register(sim, Some(register)),
                // 'c' deletes into the given register and prepares Insert
                // mode the same way plain 'c' does; the actual mode
                // transition happens in `execute_command_any_mode`, gated by
                // `is_insert_command` recognizing this register-scoped form.
                'c' => sim.change_selection_from_register(Some(register)),
                _ => Err(SimulatorError::UnknownRegisterOp { op }),
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
    if let Some(pattern) = cmd
        .strip_prefix(CMD_KEEP_MATCHING)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        return selection::keep_selections_regex(sim, pattern);
    }
    if let Some(pattern) = cmd
        .strip_prefix(CMD_REMOVE_MATCHING)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        return selection::remove_selections_regex(sim, pattern);
    }
    // Search-prompt invocation ("/ <pattern>"); bare "/" stays in the registry.
    if let Some(pattern) = cmd
        .strip_prefix(CMD_SEARCH_FORWARD)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        return search::search_pattern_forward(sim, pattern);
    }
    if let Some(pattern) = cmd
        .strip_prefix(CMD_SEARCH_BACKWARD)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        return search::search_pattern_backward(sim, pattern);
    }

    registry.execute(sim, cmd)?;

    Ok(())
}

fn record_command_if_needed_normal(
    sim: &mut HelixSimulator<NormalMode>,
    key_events: &[KeyEvent],
    mode_before: Mode,
) {
    let should_record =
        !key_events.is_empty() && !sim.is_repeating && key_events.iter().all(is_repeatable_command);

    if should_record {
        sim.repeat_buffer
            .record_command(key_events.to_vec(), mode_before);
    }
}

/// Record a successfully run Insert-entering command for `.`: the command
/// itself, then the start of the Insert recording that `Escape` finishes.
/// Nothing is recorded while a `.` replay is running.
pub(super) fn record_insert_entry(sim: &mut HelixSimulator<NormalMode>, command: &str) {
    record_command_if_needed_normal(sim, &cmd_to_key_events(command), Mode::Normal);
    if !sim.is_repeating {
        sim.repeat_buffer
            .insert_recorder_mut()
            .start(Some(command.to_string()));
    }
}

pub(super) fn execute_command_any_mode(
    sim: &mut AnyModeSimulator,
    cmd: &str,
) -> Result<(), SimulatorError> {
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

    if matches!(sim, AnyModeSimulator::Normal(_))
        && let Some((count, base)) = parse_counted_command(cmd)
        && normal_registry().count_behavior(base) != CountBehavior::Native
    {
        return execute_counted_any_mode(sim, base, count);
    }

    let placeholder = AnyModeSimulator::Normal(HelixSimulator::new(String::new()));
    let old_sim = std::mem::replace(sim, placeholder);

    let (result, new_sim) = match old_sim {
        AnyModeSimulator::Normal(mut normal_sim) => {
            if let Some(entry) = InsertEntryCommand::parse(cmd) {
                match normal_sim.begin_insert(entry) {
                    Ok(insert_sim) => (Ok(()), AnyModeSimulator::Insert(insert_sim)),
                    Err(failed) => {
                        let (normal_sim, error) = *failed;
                        (Err(error), AnyModeSimulator::Normal(normal_sim))
                    }
                }
            } else {
                let key_events = cmd_to_key_events(cmd);
                let result = select_mode::execute(&mut normal_sim, cmd);
                if result.is_ok() {
                    record_command_if_needed_normal(&mut normal_sim, &key_events, Mode::Normal);
                }
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
            let result = Err(SimulatorError::CommandLineInInsertMode);
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
    use std::assert_matches;

    #[test]
    fn counted_command_without_native_count_form_is_rejected() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc".to_string());

        assert_matches!(
            execute_native_counted(&mut sim, NonZeroUsize::new(2).unwrap(), "h"),
            Err(SimulatorError::CountNotAccepted)
        );
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

        /// Named regression test for S3/Q2: command-line invocations are
        /// deliberately not `.`-repeatable. `cmd_to_key_events` has no
        /// dedicated arm for `:`-prefixed commands, so they fall through to
        /// the generic "unmatched, len != 1" branch and produce no key
        /// events - `record_command_if_needed_normal` then skips recording
        /// entirely (an empty `key_events` slice is never recorded), so `.`
        /// after `:goto 3` is a no-op.
        ///
        /// Register-scoped ops (`"ay`, `"_d`, ...) are the opposite case -
        /// they DO need key events (see the next test) so `.` after a
        /// register-scoped delete/change/yank replays it, consistent with
        /// their unscoped counterparts (bare `y`/`d`/`c` are all
        /// `.`-repeatable per `is_repeatable_command`). This split from the
        /// register case above was itself a regression (#400): the missing
        /// arm meant `"<reg><op>` was silently unrecorded, so `.` replayed
        /// whatever the *previous* recorded action was instead.
        #[test]
        fn test_cmd_to_key_events_command_line_is_not_repeatable() {
            assert!(cmd_to_key_events(":goto 3").is_empty());
        }

        #[test]
        fn test_cmd_to_key_events_register_op_produces_three_key_events() {
            let events = cmd_to_key_events("\"ay");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('"'));
            assert_eq!(events[1].code, KeyCode::Char('a'));
            assert_eq!(events[2].code, KeyCode::Char('y'));

            let events = cmd_to_key_events("\"_d");
            assert_eq!(events.len(), 3);
            assert_eq!(events[0].code, KeyCode::Char('"'));
            assert_eq!(events[1].code, KeyCode::Char('_'));
            assert_eq!(events[2].code, KeyCode::Char('d'));
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
        fn non_ascii_arguments_produce_key_events() {
            let codes = |cmd: &str| -> Vec<KeyCode> {
                cmd_to_key_events(cmd).iter().map(|key| key.code).collect()
            };
            assert_eq!(
                codes("f\u{e9}"),
                [KeyCode::Char('f'), KeyCode::Char('\u{e9}')]
            );
            assert_eq!(
                codes("r\u{1f600}"),
                [KeyCode::Char('r'), KeyCode::Char('\u{1f600}')]
            );
            assert_eq!(
                codes("ms\u{e9}"),
                [
                    KeyCode::Char('m'),
                    KeyCode::Char('s'),
                    KeyCode::Char('\u{e9}')
                ]
            );
            assert_eq!(
                codes("mr\u{e9}\u{1f600}"),
                [
                    KeyCode::Char('m'),
                    KeyCode::Char('r'),
                    KeyCode::Char('\u{e9}'),
                    KeyCode::Char('\u{1f600}')
                ]
            );
            assert_eq!(codes("\u{e9}"), [KeyCode::Char('\u{e9}')]);
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
            assert_eq!(sim.display().content(), ":");
        }

        #[test]
        fn insert_mode_assembled_command_line_is_rejected() {
            let mut sim = AnyModeSimulator::Insert(
                HelixSimulator::<NormalMode>::new(String::new()).enter_insert_mode(),
            );
            assert_matches!(
                sim.execute_command(":goto 3"),
                Err(SimulatorError::CommandLineInInsertMode)
            );
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
            assert_matches!(
                execute_normal_mode_command_internal(&mut sim, "\"éz"),
                Err(SimulatorError::UnknownRegisterOp { op: 'z' })
            );
        }

        #[test]
        fn non_ascii_arguments_are_accepted_by_argument_taking_commands() {
            let run = |text: &str, cmd: &str| -> HelixSimulator<NormalMode> {
                let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new(text.to_string());
                execute_normal_mode_command_internal(&mut sim, cmd).unwrap();
                sim
            };

            assert_eq!(run("a caf\u{e9} b", "f\u{e9}").selection.primary().head, 6);
            assert_eq!(run("abc", "r\u{1f600}").doc.to_string(), "\u{1f600}bc");
            assert_eq!(run("abc", "ms\u{e9}").doc.to_string(), "\u{e9}\u{e9}abc");
            for cmd in ["md\u{e9}", "mr\u{e9}\u{1f600}", "mi\u{e9}", "ma\u{1f600}"] {
                run("abc", cmd);
            }
        }

        #[test]
        fn bare_non_ascii_character_is_an_unknown_command_not_a_panic() {
            let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc".to_string());
            assert!(execute_normal_mode_command_internal(&mut sim, "\u{e9}").is_err());
            assert!(execute_normal_mode_command_internal(&mut sim, "\u{1f600}").is_err());
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

            assert_matches!(
                execute_normal_mode_command_internal(&mut sim, "s ("),
                Err(SimulatorError::InvalidRegex(_))
            );
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
