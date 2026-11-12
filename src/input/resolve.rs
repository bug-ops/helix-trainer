//! One key resolution for the Task and Arcade screens
//!
//! A screen receives a [`GameplayKey`] (typed Insert-mode text, or the key
//! after keymap-overlay translation) and must decide what it means for the player's input state:
//! typed text in Insert mode, a command to run, a prefix that waits for more
//! keys, or nothing. [`resolve_keys`] is that decision, shared so both
//! screens interpret keys identically; each screen keeps its own key-history
//! and last-command bookkeeping around it.

use std::borrow::Cow;

use super::keymap::GameplayKey;
use super::typestate::{HandlerResult, InputStateMachine};
use crate::helix::ModeKind;
use crate::helix::normal_command::NormalCommand;

/// Where a resolved command came from, which screens use to pick what the key
/// history shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFrom {
    /// One keystroke (or one atomic keymap expansion) resolved it from the
    /// Base state, so the physically-typed key says it best.
    Base,
    /// The last key of a multi-keystroke sequence (`gg`, `fa`) completed it,
    /// so the resolved command says it best.
    Pending,
}

/// What a key means in the current input state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolution {
    /// Insert mode: the key is typed text or an Insert-mode key, never
    /// translated and never seen by the state machine.
    Insert(Cow<'static, str>),
    /// A command to run, with its count prefix still attached.
    Execute {
        /// The typed command.
        command: NormalCommand,
        /// How the command was reached.
        from: ResolvedFrom,
    },
    /// The state machine waits for more keys.
    Pending,
    /// Cancelled, unknown or unparsable: nothing to run.
    Discarded,
}

/// Resolve `key` against `input` in `mode`.
///
/// A single-key sequence, the overwhelmingly common case, is dispatched
/// directly. A multi-key `keys` (a keymap remap whose target spans several
/// canonical keys, e.g. `G` -> `"ge"`) is applied atomically through
/// [`InputStateMachine::apply_canonical_expansion`], which leaves `input`
/// untouched when it does not resolve cleanly. Typed text outside Insert
/// mode is a stale message (the mode changed after it was produced) and is
/// [`Resolution::Discarded`].
pub(crate) fn resolve_keys(
    input: &mut InputStateMachine,
    mode: ModeKind,
    key: &GameplayKey,
) -> Resolution {
    let keys = match (mode, key) {
        (ModeKind::Insert, GameplayKey::Typed(text)) => return Resolution::Insert(text.clone()),
        (ModeKind::Insert, GameplayKey::Keys(keys)) => return Resolution::Insert(keys.to_cow()),
        (_, GameplayKey::Typed(text)) => {
            tracing::debug!(%text, "typed text outside Insert mode; discarding");
            return Resolution::Discarded;
        }
        (_, GameplayKey::Keys(keys)) => keys,
    };

    if let &[key_event] = keys.events() {
        let from = if input.state().is_base() {
            ResolvedFrom::Base
        } else {
            ResolvedFrom::Pending
        };
        return match input.process_key(key_event) {
            HandlerResult::Execute(command) => Resolution::Execute { command, from },
            HandlerResult::Transition(_) => Resolution::Pending,
            HandlerResult::Cancel | HandlerResult::Stay => Resolution::Discarded,
        };
    }

    if let Some(command) = input.apply_canonical_expansion(keys.events()) {
        return Resolution::Execute {
            command,
            from: ResolvedFrom::Base,
        };
    }
    tracing::warn!(
        keys = keys.as_str(),
        "keymap expansion did not resolve; discarding"
    );
    Resolution::Discarded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(input: &mut InputStateMachine, mode: ModeKind, keys: &'static str) -> Resolution {
        resolve_keys(input, mode, &GameplayKey::Keys(crate::testing::keys(keys)))
    }

    fn execute(command: &'static str, from: ResolvedFrom) -> Resolution {
        Resolution::Execute {
            command: crate::testing::normal(command),
            from,
        }
    }

    #[test]
    fn insert_mode_passes_the_key_through_without_touching_the_machine() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Insert, "g"),
            Resolution::Insert(Cow::Borrowed("g"))
        );
        assert!(input.state().is_base());
    }

    #[test]
    fn a_base_key_executes_from_base() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "x"),
            execute("x", ResolvedFrom::Base)
        );
    }

    #[test]
    fn select_mode_resolves_like_normal_mode() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Select, "x"),
            execute("x", ResolvedFrom::Base)
        );
    }

    #[test]
    fn a_prefix_waits_and_its_last_key_executes_from_pending() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "g"),
            Resolution::Pending
        );
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "g"),
            execute("gg", ResolvedFrom::Pending)
        );
    }

    #[test]
    fn a_cancelling_or_unknown_key_is_discarded() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "g"),
            Resolution::Pending
        );
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "Escape"),
            Resolution::Discarded
        );
        assert!(input.state().is_base());
    }

    #[test]
    fn typed_text_is_inserted_as_is_in_insert_mode() {
        let mut input = InputStateMachine::new();
        for text in [" ", "-", "<", "A", "é", "Enter"] {
            assert_eq!(
                resolve_keys(
                    &mut input,
                    ModeKind::Insert,
                    &GameplayKey::Typed(Cow::Borrowed(text))
                ),
                Resolution::Insert(Cow::Borrowed(text))
            );
        }
    }

    #[test]
    fn typed_text_outside_insert_mode_is_discarded() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve_keys(
                &mut input,
                ModeKind::Normal,
                &GameplayKey::Typed(Cow::Borrowed("x"))
            ),
            Resolution::Discarded
        );
        assert!(input.state().is_base());
    }

    #[test]
    fn a_multi_token_expansion_executes_atomically_from_base() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "ge"),
            execute("ge", ResolvedFrom::Base)
        );
        assert!(input.state().is_base());
    }

    #[test]
    fn a_multi_token_expansion_that_does_not_resolve_leaves_the_machine_alone() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, "gz"),
            Resolution::Discarded
        );
        assert!(input.state().is_base());
    }
}
