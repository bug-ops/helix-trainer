//! Command execution and dispatch

pub mod clipboard;
pub mod editing;
pub mod movement;
pub mod search;
pub mod select_mode;
pub mod selection;
pub mod view;

use super::{AnyModeSimulator, HelixSimulator, InsertEntryCommand, InsertMode, Mode, NormalMode};
use crate::grammar::commands::CMD_COMMAND_LINE;
use crate::grammar::find::{FindDirection, FindKind};
use crate::helix::InsertInput;
use crate::helix::error::SimulatorError;
use crate::helix::normal_command::{
    MacroOp, NormalCommand, PromptKind, RegisterOp, SurroundOp, TextObjectScope,
};
use crate::helix::registry::{CountBehavior, normal_registry};
use crate::helix::repeat::{InsertAction, Movement, is_repeatable_command};
use crossterm::event::KeyEvent;

type ArrowStep = fn(&mut HelixSimulator<InsertMode>, usize) -> Result<(), SimulatorError>;

/// The cursor motion of an Insert-mode arrow key.
fn arrow_step(movement: Movement) -> ArrowStep {
    match movement {
        Movement::Left => movement::move_left,
        Movement::Right => movement::move_right,
        Movement::Up => movement::move_up,
        Movement::Down => movement::move_down,
    }
}

fn execute_insert_mode_command_internal(
    sim: &mut HelixSimulator<InsertMode>,
    input: &InsertInput,
) -> Result<(), SimulatorError> {
    match input {
        InsertInput::Escape => {
            if !sim.is_repeating {
                let action = sim.repeat_buffer.insert_recorder_mut().finish();
                sim.repeat_buffer.set_last_action(action);
            }
            Ok(())
        }
        InsertInput::Key(InsertAction::Move(movement)) => {
            let result = arrow_step(*movement)(sim, 1);
            if result.is_ok() {
                sim.session.drop_entry_ranges(&sim.selection);
                if !sim.is_repeating {
                    sim.repeat_buffer
                        .insert_recorder_mut()
                        .record_movement(*movement);
                }
            }
            result
        }
        InsertInput::Key(InsertAction::Edit(command)) => {
            let result = sim.apply_insert_command(*command);
            if result.is_ok() && !sim.is_repeating {
                sim.repeat_buffer
                    .insert_recorder_mut()
                    .record_edit(*command);
            }
            result
        }
        InsertInput::Text(text) => {
            let result = sim.insert_text(text);
            if result.is_ok() && !sim.is_repeating {
                for ch in text.chars() {
                    sim.repeat_buffer.insert_recorder_mut().record_char(ch);
                }
            }
            result
        }
    }
}

/// Run one parsed Normal-mode command against a Normal-mode simulator.
///
/// Mode transitions, `.`, macros and counts are the business of
/// [`execute_normal_any_mode`]; this is the exhaustive dispatch of what a
/// command does to the document.
pub(super) fn execute_normal_mode_command(
    sim: &mut HelixSimulator<NormalMode>,
    command: &NormalCommand,
) -> Result<(), SimulatorError> {
    match command {
        NormalCommand::Registered {
            command,
            count: None,
        } => normal_registry().execute(sim, command.key()).map(drop),
        NormalCommand::Registered { count: Some(_), .. } => Err(SimulatorError::CountNotAccepted),
        NormalCommand::InsertEntry(InsertEntryCommand::Change {
            register: Some(register),
        }) => sim.change_selection_from_register(Some(*register)),
        NormalCommand::InsertEntry(entry) => normal_registry()
            .execute(sim, &entry.uncounted().command())
            .map(drop),
        NormalCommand::Macro(op) => normal_registry().execute(sim, op.key()).map(drop),
        NormalCommand::Repeat | NormalCommand::Escape => Ok(()),
        NormalCommand::Find { motion, ch, count } => {
            let count = count.map_or(1, std::num::NonZeroUsize::get);
            match (motion.kind(), motion.direction()) {
                (FindKind::Find, FindDirection::Forward) => {
                    movement::find_next_char(sim, *ch, count)
                }
                (FindKind::Find, FindDirection::Backward) => {
                    movement::find_prev_char(sim, *ch, count)
                }
                (FindKind::Till, FindDirection::Forward) => {
                    movement::till_next_char(sim, *ch, count)
                }
                (FindKind::Till, FindDirection::Backward) => {
                    movement::till_prev_char(sim, *ch, count)
                }
            }
        }
        // Bare `G` is `goto_line`, a no-op without a count in helix-term 25.07.1.
        NormalCommand::GotoLine { count } => {
            if let Some(count) = count {
                movement::goto_line(sim, *count);
            }
            Ok(())
        }
        NormalCommand::Replace(ch) => sim.replace_char(*ch),
        NormalCommand::Surround(SurroundOp::Add(ch)) => editing::surround_selection(sim, *ch),
        NormalCommand::Surround(SurroundOp::Delete(ch)) => editing::delete_surround(sim, *ch),
        NormalCommand::Surround(SurroundOp::Replace { from, to }) => {
            editing::replace_surround(sim, *from, *to)
        }
        NormalCommand::TextObject {
            scope: TextObjectScope::Around,
            object,
        } => editing::select_around_textobject(sim, *object),
        NormalCommand::TextObject {
            scope: TextObjectScope::Inside,
            object,
        } => editing::select_inside_textobject(sim, *object),
        NormalCommand::Register { register, op } => {
            let register = Some(*register);
            match op {
                RegisterOp::Yank => clipboard::yank_to_register(sim, register),
                RegisterOp::PasteAfter => clipboard::paste_after_from_register(sim, register),
                RegisterOp::PasteBefore => clipboard::paste_before_from_register(sim, register),
                RegisterOp::ReplaceWithYanked => {
                    editing::replace_with_yanked_from_register(sim, register)
                }
                RegisterOp::Delete => editing::delete_selection_from_register(sim, register),
            }
        }
        // The regex and search prompts assemble the command atomically, so
        // the pattern keeps its spaces.
        NormalCommand::Prompt { kind, pattern } => match kind {
            PromptKind::SelectRegex => selection::select_regex(sim, pattern),
            PromptKind::SplitSelection => selection::split_selection(sim, pattern),
            PromptKind::KeepSelections => selection::keep_selections_regex(sim, pattern),
            PromptKind::RemoveSelections => selection::remove_selections_regex(sim, pattern),
            PromptKind::Search => search::search_pattern_forward(sim, pattern),
            PromptKind::SearchBackward => search::search_pattern_backward(sim, pattern),
        },
        NormalCommand::CommandLine(command_line) => command_line.execute(sim),
    }
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
pub(super) fn record_insert_entry(sim: &mut HelixSimulator<NormalMode>, entry: InsertEntryCommand) {
    let keys = NormalCommand::InsertEntry(entry).keys();
    record_command_if_needed_normal(sim, &keys, Mode::Normal);
    if !sim.is_repeating {
        sim.repeat_buffer
            .insert_recorder_mut()
            .start(Some(entry.command()));
    }
}

/// Hand the simulator to `run` by value, so a command can change its mode.
///
/// The placeholder only lives while `run` does; `begin_insert` and
/// `exit_insert_mode` consume the simulator, which a `&mut` cannot give up.
fn transition<R>(
    sim: &mut AnyModeSimulator,
    run: impl FnOnce(AnyModeSimulator) -> (R, AnyModeSimulator),
) -> R {
    let placeholder = AnyModeSimulator::Normal(HelixSimulator::new(String::new()));
    let (result, next) = run(std::mem::replace(sim, placeholder));
    *sim = next;
    match sim {
        AnyModeSimulator::Normal(normal_sim) => view::ensure_cursor_in_view(normal_sim),
        AnyModeSimulator::Insert(insert_sim) => view::ensure_cursor_in_view(insert_sim),
    }
    result
}

/// Run `run` on a Normal-mode simulator wrapped as an [`AnyModeSimulator`].
fn on_any_mode(
    normal_sim: HelixSimulator<NormalMode>,
    run: impl FnOnce(&mut AnyModeSimulator) -> Result<(), SimulatorError>,
) -> (Result<(), SimulatorError>, AnyModeSimulator) {
    let mut sim = AnyModeSimulator::Normal(normal_sim);
    (run(&mut sim), sim)
}

/// Run a command string: parsed in Normal mode, typed text in Insert mode.
pub(super) fn execute_command_any_mode(
    sim: &mut AnyModeSimulator,
    cmd: &str,
) -> Result<(), SimulatorError> {
    run_input(sim, cmd, || InsertInput::parse(cmd))
}

/// Run an Insert-mode input; a Normal-mode simulator takes the command its text names.
pub(super) fn execute_insert_any_mode(
    sim: &mut AnyModeSimulator,
    input: &InsertInput,
) -> Result<(), SimulatorError> {
    run_input(sim, input.command(), || input.clone())
}

fn run_input(
    sim: &mut AnyModeSimulator,
    cmd: &str,
    insert_input: impl FnOnce() -> InsertInput,
) -> Result<(), SimulatorError> {
    transition(sim, |sim| match sim {
        AnyModeSimulator::Normal(normal_sim) => match NormalCommand::parse(cmd) {
            Ok(command) => run_normal(normal_sim, &command),
            Err(error) => (Err(error), AnyModeSimulator::Normal(normal_sim)),
        },
        AnyModeSimulator::Insert(insert_sim) => run_insert(insert_sim, &insert_input()),
    })
}

/// Run a parsed Normal-mode command, handling counts, `.`, macros and the
/// transition into Insert mode.
///
/// A simulator that is already in Insert mode takes the command as the text
/// it would be typed as.
pub(super) fn execute_normal_any_mode(
    sim: &mut AnyModeSimulator,
    command: &NormalCommand,
) -> Result<(), SimulatorError> {
    transition(sim, |sim| match sim {
        AnyModeSimulator::Normal(normal_sim) => run_normal(normal_sim, command),
        AnyModeSimulator::Insert(insert_sim) => {
            run_insert(insert_sim, &InsertInput::parse(&command.to_string()))
        }
    })
}

fn run_normal(
    mut normal_sim: HelixSimulator<NormalMode>,
    command: &NormalCommand,
) -> (Result<(), SimulatorError>, AnyModeSimulator) {
    match command {
        NormalCommand::Repeat => return on_any_mode(normal_sim, AnyModeSimulator::execute_repeat),
        NormalCommand::Macro(MacroOp::ToggleRecording) => {
            normal_sim.toggle_macro_recording();
            return (Ok(()), AnyModeSimulator::Normal(normal_sim));
        }
        NormalCommand::Macro(MacroOp::Replay) => {
            // `q` while recording is a no-op, not a replay: the recording tap
            // below is skipped while `execute_macro_replay` sets
            // `is_replaying`, so replayed effects would apply to the document
            // while silently NOT being captured into the macro currently being
            // recorded - a training tool producing a macro that doesn't match
            // what the user watched happen on screen. See `macro_recorder.rs`
            // module docs.
            return on_any_mode(normal_sim, |sim| {
                if sim.is_recording_macro() {
                    return Ok(());
                }
                sim.execute_macro_replay()
            });
        }
        NormalCommand::Registered {
            command: registered,
            count: Some(count),
        } => {
            let uncounted = command.uncounted();
            return on_any_mode(normal_sim, |sim| {
                match normal_registry().count_behavior(*registered) {
                    CountBehavior::Repeat => {
                        for _ in 0..count.get() {
                            execute_normal_any_mode(sim, &uncounted)?;
                            // A command that entered Insert mode must not run again as typed text.
                            if matches!(sim, AnyModeSimulator::Insert(_)) {
                                break;
                            }
                        }
                        Ok(())
                    }
                    CountBehavior::Ignore => execute_normal_any_mode(sim, &uncounted),
                    CountBehavior::Native => Err(SimulatorError::CountNotAccepted),
                }
            });
        }
        _ => {}
    }

    let (result, mut new_sim) = if let NormalCommand::InsertEntry(entry) = command {
        match normal_sim.begin_insert(*entry) {
            Ok(insert_sim) => (Ok(()), AnyModeSimulator::Insert(insert_sim)),
            Err(failed) => {
                let (normal_sim, error) = *failed;
                (Err(error), AnyModeSimulator::Normal(normal_sim))
            }
        }
    } else {
        let result = select_mode::execute(&mut normal_sim, command);
        if result.is_ok() {
            record_command_if_needed_normal(&mut normal_sim, &command.keys(), Mode::Normal);
        }
        (result, AnyModeSimulator::Normal(normal_sim))
    };

    // Only successful commands are recorded - a macro is a record of what
    // actually happened, and replaying a command that failed would reproduce
    // the failure at best. `record_macro_command` is itself a no-op unless
    // recording is active and this isn't itself part of a replay.
    if result.is_ok() {
        new_sim.record_macro_command(&command.to_string());
    }
    (result, new_sim)
}

/// Run `input` against an Insert-mode simulator.
pub(super) fn run_insert(
    mut insert_sim: HelixSimulator<InsertMode>,
    input: &InsertInput,
) -> (Result<(), SimulatorError>, AnyModeSimulator) {
    let cmd = input.command();
    let (result, mut new_sim) = if matches!(input, InsertInput::Text(_))
        && cmd.len() > 1
        && cmd.starts_with(CMD_COMMAND_LINE)
    {
        // Command-line strings are assembled by `CommandLinePending` in
        // Normal mode only; guarded here too so an *assembled* `:`-prefixed
        // string (e.g. ":goto 3") can never be inserted as literal text.
        // `cmd.len() > 1` matters: insert mode delivers every typed
        // character as its own one-char command, so without this a
        // literal ':' keystroke (len 1) would hit this arm and be
        // rejected instead of inserted.
        (
            Err(SimulatorError::CommandLineInInsertMode),
            AnyModeSimulator::Insert(insert_sim),
        )
    } else {
        let result = execute_insert_mode_command_internal(&mut insert_sim, input);
        if result.is_ok() && *input == InsertInput::Escape {
            (
                result,
                AnyModeSimulator::Normal(insert_sim.exit_insert_mode()),
            )
        } else {
            (result, AnyModeSimulator::Insert(insert_sim))
        }
    };

    if result.is_ok() {
        new_sim.record_macro_command(cmd);
    }
    (result, new_sim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    /// Parse and run `cmd` against a Normal-mode simulator, without counts.
    fn execute_normal_mode_command_internal(
        sim: &mut HelixSimulator<NormalMode>,
        cmd: &str,
    ) -> Result<(), SimulatorError> {
        execute_normal_mode_command(sim, &NormalCommand::parse(cmd)?)
    }

    #[test]
    fn a_registered_count_is_for_the_any_mode_dispatcher_only() {
        let mut sim: HelixSimulator<NormalMode> = HelixSimulator::new("abc".to_string());

        assert_matches!(
            execute_normal_mode_command_internal(&mut sim, "2h"),
            Err(SimulatorError::CountNotAccepted)
        );
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
