//! Parsing and resolving a user's Helix `config.toml` keymap into a
//! [`KeymapOverlay`].
//!
//! Supported subset (see the design decision record for the full
//! rationale): `[keys.normal]` top-level single-key -> command-name
//! bindings, plus nested `[keys.normal.<p>]` for `p` in `g`, `m`, `z`,
//! `[`, `]`. Everything else (command sequences, `@`-macros, `:`-typable
//! commands, `[keys.select]`/`[keys.insert]`, minor-mode relocation to a
//! different prefix key) is reported as an ignored binding rather than
//! silently dropped.

use std::fs;
use std::path::Path;

use toml::Value;

use crate::helix::registry::{CommandRegistry, normal_registry};
use crate::helix::simulator::NormalMode;
use crate::input::keymap::{CanonicalKeys, KeyContext, KeymapOverlay, PhysicalKey};
use crate::input::typestate::{HandlerResult, InputStateMachine, parse_helix_key_string};
use crate::security::{SecurityError, limits, path_validator};

/// One binding from the user's Helix config that could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapWarning {
    /// Dotted TOML path of the offending entry, e.g. `"keys.normal.j"` or
    /// `"keys.normal.g.x"`, for the status notification and `tracing::warn!`.
    pub path: String,
    /// Why it was ignored.
    pub reason: KeymapWarningReason,
}

/// Why a single keymap binding was not applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeymapWarningReason {
    /// Not a command name this trainer implements. May still be a real
    /// Helix command — the trainer only implements a subset.
    UnknownCommand(String),
    /// The physical key string didn't parse.
    UnparsableKey(String),
    /// The binding's value wasn't a plain command-name string (a key
    /// sequence, `@`-macro, `:`-typable command, or other unsupported form).
    UnsupportedBindingForm,
    /// A minor-mode table this trainer doesn't model (only `g`/`m`/`z`/`[`/`]` are).
    UnsupportedMinorMode,
    /// A minor-mode binding whose target isn't reachable without leaving
    /// that submenu (relocating a command across prefixes is unsupported).
    UnsupportedRelocation,
    /// The resolved canonical sequence didn't cleanly resolve when replayed
    /// from `Base` (rare; usually a registry/tokenizer mismatch).
    DidNotResolve,
}

/// Applied + ignored bindings from resolving one Helix config file.
#[derive(Debug, Clone, Default)]
pub struct KeymapReport {
    /// Number of bindings successfully applied to the overlay.
    pub applied: usize,
    /// Bindings that were present in the config but could not be applied.
    pub ignored: Vec<KeymapWarning>,
}

/// Failure loading the user's Helix config from disk.
#[derive(Debug, thiserror::Error)]
pub enum KeymapLoadError {
    /// No file exists at the resolved path.
    #[error("Helix config file not found")]
    NotFound,
    /// The file exists but is too large, unreadable, or not valid TOML.
    #[error(transparent)]
    Security(#[from] SecurityError),
}

/// Load and resolve the user's Helix `config.toml` at `path`.
///
/// Returns [`KeymapLoadError::NotFound`] if `path` doesn't exist, so a
/// caller can degrade to [`KeymapOverlay::identity`] and notify the user
/// without treating a missing file as a hard error.
///
/// Accepted risk: unlike scenario/quest loading, this intentionally skips
/// `path_validator::validate_path` (no allowed-bases check) and follows
/// symlinks. Defensible only because `path` is always the fixed,
/// non-user-controlled `dirs::config_dir()/helix/config.toml` (see M2 in
/// the design decision record) — never an arbitrary path derived from
/// user input — so there is no traversal surface to validate against.
pub fn load_keymap_file(path: &Path) -> Result<(KeymapOverlay, KeymapReport), KeymapLoadError> {
    if !path.exists() {
        return Err(KeymapLoadError::NotFound);
    }
    path_validator::validate_file_size(path, limits::MAX_KEYMAP_FILE_SIZE)?;
    let content = fs::read_to_string(path).map_err(|_| SecurityError::InvalidPath)?;
    Ok(resolve_str(&content)?)
}

/// Parse and resolve keymap TOML content directly (no filesystem access).
///
/// # Examples
///
/// ```
/// use helix_trainer::config::keymap::resolve_str;
///
/// let (overlay, report) = resolve_str(r#"
///     [keys.normal]
///     j = "move_char_up"
/// "#).unwrap();
/// assert_eq!(report.applied, 0);
/// assert_eq!(report.ignored.len(), 1); // "move_char_up" isn't a real Helix command
/// assert!(overlay.is_empty());
/// ```
pub fn resolve_str(content: &str) -> Result<(KeymapOverlay, KeymapReport), SecurityError> {
    let root: Value =
        toml::from_str(content).map_err(|e| SecurityError::InvalidToml(e.to_string()))?;
    resolve(&root)
}

/// Accumulates resolved bindings and ignored-binding warnings across one
/// `resolve()` pass. Bundling these together keeps the per-binding helper
/// functions' argument counts down to "the data needed to resolve one
/// binding" rather than also threading three separate output collections.
#[derive(Default)]
struct Resolution {
    entries: Vec<((KeyContext, PhysicalKey), CanonicalKeys)>,
    warnings: Vec<KeymapWarning>,
    applied: usize,
}

impl Resolution {
    fn apply(&mut self, context: KeyContext, key: PhysicalKey, canonical: CanonicalKeys) {
        self.entries.push(((context, key), canonical));
        self.applied += 1;
    }

    fn ignore(&mut self, path: String, reason: KeymapWarningReason) {
        self.warnings.push(KeymapWarning { path, reason });
    }

    fn into_overlay_and_report(self) -> (KeymapOverlay, KeymapReport) {
        for warning in &self.warnings {
            tracing::warn!(path = %warning.path, reason = ?warning.reason, "ignored keymap binding");
        }
        (
            KeymapOverlay::from_entries(self.entries),
            KeymapReport {
                applied: self.applied,
                ignored: self.warnings,
            },
        )
    }
}

fn resolve(root: &Value) -> Result<(KeymapOverlay, KeymapReport), SecurityError> {
    let Some(normal) = root
        .get("keys")
        .and_then(|k| k.get("normal"))
        .and_then(Value::as_table)
    else {
        return Ok((KeymapOverlay::identity(), KeymapReport::default()));
    };

    let total_bindings = count_bindings(normal);
    if total_bindings > limits::MAX_KEYMAP_BINDINGS {
        return Err(SecurityError::TooManyKeymapBindings {
            max: limits::MAX_KEYMAP_BINDINGS,
            actual: total_bindings,
        });
    }

    let registry = normal_registry();
    let mut resolution = Resolution::default();

    for (key_str, value) in normal {
        match value {
            Value::String(command_name) => {
                apply_base_binding(registry, key_str, command_name, &mut resolution);
            }
            Value::Table(sub_table) => {
                let Some(context) = KeyContext::from_prefix_char(key_str) else {
                    resolution.ignore(
                        format!("keys.normal.{key_str}"),
                        KeymapWarningReason::UnsupportedMinorMode,
                    );
                    continue;
                };
                for (sub_key, sub_value) in sub_table {
                    let path = format!("keys.normal.{key_str}.{sub_key}");
                    match sub_value {
                        Value::String(command_name) => {
                            apply_minor_mode_binding(
                                registry,
                                context,
                                sub_key,
                                command_name,
                                path,
                                &mut resolution,
                            );
                        }
                        _ => resolution.ignore(path, KeymapWarningReason::UnsupportedBindingForm),
                    }
                }
            }
            _ => resolution.ignore(
                format!("keys.normal.{key_str}"),
                KeymapWarningReason::UnsupportedBindingForm,
            ),
        }
    }

    Ok(resolution.into_overlay_and_report())
}

/// Count top-level plus minor-mode-nested bindings, for the size cap.
/// A minor-mode table that isn't a table at all (malformed) still counts
/// as one binding so it's covered by the cap.
fn count_bindings(normal: &toml::Table) -> usize {
    normal
        .values()
        .map(|v| v.as_table().map_or(1, toml::Table::len))
        .sum()
}

fn apply_base_binding(
    registry: &CommandRegistry<NormalMode>,
    key_str: &str,
    command_name: &str,
    resolution: &mut Resolution,
) {
    let path = format!("keys.normal.{key_str}");
    let Some(canonical_key) = registry.key_for_name(command_name) else {
        resolution.ignore(
            path,
            KeymapWarningReason::UnknownCommand(command_name.to_string()),
        );
        return;
    };
    let Ok(physical) = PhysicalKey::try_from(key_str) else {
        resolution.ignore(
            path,
            KeymapWarningReason::UnparsableKey(key_str.to_string()),
        );
        return;
    };
    let canonical = CanonicalKeys::from_static(canonical_key);
    if !resolves_cleanly_from_base(&canonical) {
        resolution.ignore(path, KeymapWarningReason::DidNotResolve);
        return;
    }
    resolution.apply(KeyContext::Base, physical, canonical);
}

/// A minor-mode remap can only retarget a command reachable *within the
/// same submenu*: its canonical key must be exactly the context's own
/// prefix character followed by one more token. Retargeting a command
/// behind a different prefix (or a top-level command) would require
/// abandoning the pending state and replaying from `Base`, which this v1
/// design doesn't support (reported as [`KeymapWarningReason::UnsupportedRelocation`]).
fn apply_minor_mode_binding(
    registry: &CommandRegistry<NormalMode>,
    context: KeyContext,
    sub_key: &str,
    command_name: &str,
    path: String,
    resolution: &mut Resolution,
) {
    let Some(canonical_key) = registry.key_for_name(command_name) else {
        resolution.ignore(
            path,
            KeymapWarningReason::UnknownCommand(command_name.to_string()),
        );
        return;
    };
    let full = CanonicalKeys::from_static(canonical_key);
    let tokens = full.tokens();
    if tokens.len() != 2 || Some(tokens[0]) != context.prefix_char() {
        resolution.ignore(path, KeymapWarningReason::UnsupportedRelocation);
        return;
    }
    let Ok(physical) = PhysicalKey::try_from(sub_key) else {
        resolution.ignore(
            path,
            KeymapWarningReason::UnparsableKey(sub_key.to_string()),
        );
        return;
    };
    let remainder = CanonicalKeys::from_owned(tokens[1].to_string());
    resolution.apply(context, physical, remainder);
}

/// Replay `canonical`'s tokens through a throwaway `InputStateMachine`
/// starting at `Base`, exactly as it will be dispatched at runtime.
/// Every intermediate token must transition. The final token must
/// execute — *except* for a single-token target (`tokens.len() == 1`),
/// which may instead leave the machine in a pending state (e.g. remapping
/// a key to the bare prefix `find_next_char`, matching un-remapped `f` ->
/// `FindCharPending`).
///
/// The two must agree, not by coincidence but by construction: a
/// multi-token target that ends in `Transition` would be accepted here
/// yet unconditionally rejected at runtime by
/// `InputStateMachine::apply_canonical_expansion`, which requires the
/// final token to execute regardless of token count (that method is only
/// ever invoked for `tokens.len() > 1` — see `handlers.rs`'s
/// `handle_gameplay_input` — so its stricter contract is the one that
/// actually matters for multi-token targets). Today no registered
/// multi-token key's final token is itself a prefix (enforced by
/// `no_registered_multi_token_key_ends_in_a_prefix` below), so this only
/// guards against a *future* registry change silently producing a binding
/// that's reported as "applied" but dead at runtime.
fn resolves_cleanly_from_base(canonical: &CanonicalKeys) -> bool {
    let tokens = canonical.tokens();
    let Some(last) = tokens.len().checked_sub(1) else {
        return false;
    };
    let mut machine = InputStateMachine::new();
    for (i, token) in tokens.iter().enumerate() {
        let Some(key_event) = parse_helix_key_string(token) else {
            return false;
        };
        let result = machine.process_key(key_event);
        if i == last {
            return matches!(result, HandlerResult::Execute(_))
                || (tokens.len() == 1 && matches!(result, HandlerResult::Transition(_)));
        }
        if !matches!(result, HandlerResult::Transition(_)) {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M1: `resolves_cleanly_from_base`'s "final token may transition"
    /// exception is scoped to `tokens.len() == 1` because *today*, by
    /// coincidence rather than by construction, no registered multi-token
    /// canonical key ends in a further-prefix token. This test makes that
    /// assumption load-bearing: if a future registry change (e.g.
    /// registering `surround_add` under `ms`) ever violates it, this
    /// fails loudly instead of silently producing a binding that's
    /// reported as "applied" but dead at runtime (see the doc comment on
    /// `resolves_cleanly_from_base`).
    #[test]
    fn no_registered_multi_token_key_ends_in_a_prefix() {
        let registry = normal_registry();
        for meta in registry.all_commands() {
            let canonical = CanonicalKeys::from_static(meta.key);
            let tokens = canonical.tokens();
            if tokens.len() < 2 {
                continue;
            }
            let mut machine = InputStateMachine::new();
            let mut result = None;
            for token in &tokens {
                let key_event = parse_helix_key_string(token).unwrap_or_else(|| {
                    panic!("registry key {:?} token {:?} unparsable", meta.key, token)
                });
                result = Some(machine.process_key(key_event));
            }
            assert!(
                matches!(result, Some(HandlerResult::Execute(_))),
                "registry key {:?} (name {:?}) is multi-token but its final token doesn't execute: {:?}",
                meta.key,
                meta.name,
                result
            );
        }
    }

    #[test]
    fn empty_config_yields_identity_overlay() {
        let (overlay, report) = resolve_str("").unwrap();
        assert!(overlay.is_empty());
        assert_eq!(report.applied, 0);
        assert!(report.ignored.is_empty());
    }

    #[test]
    fn simple_top_level_remap_applies() {
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal]
            j = "move_char_left"
            "#,
        )
        .unwrap();
        assert_eq!(report.applied, 1);
        assert!(report.ignored.is_empty());
        let key = PhysicalKey::try_from("j").unwrap();
        assert_eq!(
            overlay.lookup(KeyContext::Base, key),
            Some(&CanonicalKeys::from_static("h"))
        );
    }

    #[test]
    fn multi_token_target_resolves_and_applies() {
        // G -> goto_last_line, canonical "ge" (2 tokens), applied only from Base.
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal]
            G = "goto_last_line"
            "#,
        )
        .unwrap();
        assert_eq!(report.applied, 1);
        let key = PhysicalKey::try_from("G").unwrap();
        assert_eq!(
            overlay.lookup(KeyContext::Base, key),
            Some(&CanonicalKeys::from_static("ge"))
        );
    }

    #[test]
    fn unknown_command_is_ignored_with_reason() {
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal]
            j = "totally_not_a_command"
            "#,
        )
        .unwrap();
        assert!(overlay.is_empty());
        assert_eq!(report.applied, 0);
        assert_eq!(report.ignored.len(), 1);
        assert!(matches!(
            report.ignored[0].reason,
            KeymapWarningReason::UnknownCommand(_)
        ));
    }

    #[test]
    fn command_sequence_array_is_unsupported_binding_form() {
        let (_, report) = resolve_str(
            r#"
            [keys.normal]
            j = ["move_char_left", "move_char_left"]
            "#,
        )
        .unwrap();
        assert_eq!(report.applied, 0);
        assert_eq!(
            report.ignored[0].reason,
            KeymapWarningReason::UnsupportedBindingForm
        );
    }

    #[test]
    fn minor_mode_within_submenu_relabel_applies() {
        // Swap which key in the g-submenu triggers goto_last_line.
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal.g]
            x = "goto_last_line"
            "#,
        )
        .unwrap();
        assert_eq!(report.applied, 1);
        let key = PhysicalKey::try_from("x").unwrap();
        assert_eq!(
            overlay.lookup(KeyContext::Goto, key),
            Some(&CanonicalKeys::from_static("e"))
        );
    }

    #[test]
    fn minor_mode_cross_prefix_relocation_is_unsupported() {
        // Target's canonical ("y") doesn't start with "g" -> relocation unsupported.
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal.g]
            x = "yank"
            "#,
        )
        .unwrap();
        assert!(overlay.is_empty());
        assert_eq!(
            report.ignored[0].reason,
            KeymapWarningReason::UnsupportedRelocation
        );
    }

    #[test]
    fn unmodeled_minor_mode_table_is_ignored() {
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal.space]
            f = "file_picker"
            "#,
        )
        .unwrap();
        assert!(overlay.is_empty());
        assert_eq!(
            report.ignored[0].reason,
            KeymapWarningReason::UnsupportedMinorMode
        );
    }

    #[test]
    fn select_and_insert_tables_are_silently_ignored() {
        // Out of scope by design (no select mode; insert mode has no dispatch) -
        // not even worth a warning, since they're not part of our supported subset.
        let (overlay, report) = resolve_str(
            r#"
            [keys.select]
            j = "extend_char_left"

            [keys.insert]
            "C-x" = "completion"
            "#,
        )
        .unwrap();
        assert!(overlay.is_empty());
        assert!(report.ignored.is_empty());
    }

    #[test]
    fn unparsable_key_string_is_ignored() {
        let (overlay, report) = resolve_str(
            r#"
            [keys.normal]
            "not-a-key" = "move_char_left"
            "#,
        )
        .unwrap();
        assert!(overlay.is_empty());
        assert!(matches!(
            report.ignored[0].reason,
            KeymapWarningReason::UnparsableKey(_)
        ));
    }

    #[test]
    fn malformed_toml_is_a_security_error() {
        assert!(resolve_str("not valid toml {{{").is_err());
    }

    #[test]
    fn too_many_bindings_is_a_security_error() {
        let mut toml = String::from("[keys.normal]\n");
        for i in 0..(limits::MAX_KEYMAP_BINDINGS + 1) {
            toml.push_str(&format!("k{i} = \"move_char_left\"\n"));
        }
        let err = resolve_str(&toml).unwrap_err();
        assert!(matches!(err, SecurityError::TooManyKeymapBindings { .. }));
    }

    #[test]
    fn missing_file_is_not_found_not_a_hard_error() {
        let err = load_keymap_file(Path::new("/nonexistent/helix/config.toml")).unwrap_err();
        assert!(matches!(err, KeymapLoadError::NotFound));
    }
}
