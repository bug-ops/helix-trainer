//! One key resolution for the Task and Arcade screens
//!
//! A screen receives a [`CanonicalKeys`] (the key after keymap-overlay
//! translation) and must decide what it means for the player's input state:
//! typed text in Insert mode, a command to run, a prefix that waits for more
//! keys, or nothing. [`resolve_keys`] is that decision, shared so both
//! screens interpret keys identically; each screen keeps its own key-history
//! and last-command bookkeeping around it.

use std::borrow::Cow;

use super::keymap::CanonicalKeys;
use super::typestate::{HandlerResult, InputStateMachine, parse_helix_key_string};
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

/// Resolve `keys` against `input` in `mode`.
///
/// Single-token keys, the overwhelmingly common case, are dispatched
/// directly to avoid the `Vec` that [`CanonicalKeys::tokens`] allocates. A
/// multi-token `keys` (a keymap remap whose target spans several canonical
/// keys, e.g. `G` -> `"ge"`) is applied atomically through
/// [`InputStateMachine::apply_canonical_expansion`], which leaves `input`
/// untouched when it does not resolve cleanly. An empty `keys` (the keymap
/// parser rejects empty bindings at load time) is [`Resolution::Discarded`].
pub(crate) fn resolve_keys(
    input: &mut InputStateMachine,
    mode: ModeKind,
    keys: &CanonicalKeys,
) -> Resolution {
    if mode == ModeKind::Insert {
        return Resolution::Insert(keys.clone().into_cow());
    }

    if keys.is_single_token() {
        let Some(key_event) = parse_helix_key_string(keys.as_str()) else {
            tracing::warn!(keys = keys.as_str(), "unparsable key token; dropping");
            return Resolution::Discarded;
        };
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

    let tokens = keys.tokens();
    if let Some(command) = input.apply_canonical_expansion(&tokens) {
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
        resolve_keys(input, mode, &CanonicalKeys::from_static(keys))
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
    fn an_unparsable_token_is_discarded() {
        let mut input = InputStateMachine::new();
        assert_eq!(
            resolve(&mut input, ModeKind::Normal, ""),
            Resolution::Discarded
        );
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
