//! Typed review-card identity: which recorded keystrokes are Helix commands worth practicing.
//!
//! A scenario solution records every keystroke, including text typed in Insert mode and
//! command arguments. [`CommandId`] is the only way to mint a review-card id from
//! such a recording, so typed text, `Escape` and argument characters can never become cards.
//! It is separate from [`crate::grammar::commands::normalize_command_id`], whose contract
//! (quests, minigame scorer) is unchanged; see [`super::NormalizedCommand`] for that form.

use super::IdError;
use crate::grammar::find::takes_char_argument;
use crate::limits::MAX_ID_LENGTH;
use serde::Deserialize;

use crate::grammar::commands::{
    CMD_APPEND, CMD_APPEND_LINE_END, CMD_CHANGE, CMD_CHANGE_SELECTION_NOYANK, CMD_ESCAPE,
    CMD_INSERT, CMD_INSERT_LINE_START, CMD_OPEN_ABOVE, CMD_OPEN_BELOW, CMD_SEARCH_BACKWARD,
    CMD_SEARCH_FORWARD, CMD_SELECT_REGISTER, normalize_command_id,
};

const INSERT_ENTRY_COMMANDS: [&str; 8] = [
    CMD_INSERT,
    CMD_APPEND,
    CMD_INSERT_LINE_START,
    CMD_APPEND_LINE_END,
    CMD_OPEN_BELOW,
    CMD_OPEN_ABOVE,
    CMD_CHANGE,
    CMD_CHANGE_SELECTION_NOYANK,
];

/// Single non-alphanumeric characters that are Helix normal-mode commands.
const PUNCTUATION_COMMANDS: &str = "$^%;,.<>~`&_*/?()=";

/// An FSRS review-card id: a Helix command that may become a card.
///
/// The only constructors are [`CommandId::from_action`] (canonical derivation from a raw
/// recorded action) and [`CommandId::from_card_id`] (re-validation of a stored id), so every
/// value is canonical and at most 64 bytes. Deserialization goes through `from_card_id`.
///
/// # Examples
///
/// ```
/// use helix_trainer::ids::CommandId;
///
/// let ids: Vec<String> = CommandId::from_actions(["3h", "f=", "/ sum", "Escape", "!"])
///     .into_iter()
///     .map(CommandId::into_string)
///     .collect();
/// assert_eq!(ids, ["h", "f", "/"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct CommandId(Box<str>);

impl CommandId {
    /// Consume the command, yielding its card id.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into_string()
    }

    /// Mint a card id from one recorded action, ignoring Insert-mode context.
    ///
    /// Returns `None` for blank input, `Escape`, a bare count, and single characters that
    /// are not Helix commands. Strips a count prefix (`3h` -> `h`), drops the argument of
    /// find/till/replace (`f=` -> `f`) and of searches (`/ sum` -> `/`), keeps surround
    /// forms (`ma(`), and applies [`normalize_command_id`] to the rest. Also returns `None`
    /// when the resulting id exceeds 64 bytes (an overlong `:name` invocation), so such a
    /// command never becomes a card.
    #[must_use]
    pub fn from_action(raw: &str) -> Option<Self> {
        Self::mint(raw).filter(|command| command.0.len() <= MAX_ID_LENGTH)
    }

    fn mint(raw: &str) -> Option<Self> {
        let rest = if raw.starts_with(|c: char| ('1'..='9').contains(&c)) {
            raw.trim_start_matches(|c: char| c.is_ascii_digit())
        } else {
            raw
        };
        if rest.trim().is_empty() || rest == CMD_ESCAPE {
            return None;
        }
        if let Some(search) = search_command(rest) {
            return Some(Self(search.into()));
        }
        let mut chars = rest.chars();
        let first = chars.next()?;
        match (chars.next(), takes_char_argument(first)) {
            (Some(_), true) => Some(Self(first.to_string().into())),
            (None, _) if !first.is_alphanumeric() && !PUNCTUATION_COMMANDS.contains(first) => None,
            _ => Some(Self(normalize_command_id(rest).into())),
        }
    }

    /// Accept an id already stored on a card, only if it is in canonical form.
    ///
    /// `3h`, `f=`, `Escape` and `!` are rejected: they are raw recordings that
    /// [`CommandId::from_action`] would map to a different id or to nothing, so a card
    /// keyed by them predates the reviewable-command filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::ids::CommandId;
    ///
    /// assert!(CommandId::from_card_id("h").is_some());
    /// assert!(CommandId::from_card_id("3h").is_none());
    /// assert!(CommandId::from_card_id("Escape").is_none());
    /// ```
    #[must_use]
    pub fn from_card_id(id: &str) -> Option<Self> {
        Self::from_action(id).filter(|command| command.as_str() == id)
    }

    /// Mint card ids from a whole recorded solution, dropping Insert-mode typing.
    ///
    /// Everything between an insert-entering command (`i`, `a`, `o`, `c`, ...) and the next
    /// `Escape` is typed text and yields no card; the entering command itself does.
    #[must_use]
    pub fn from_actions<'a>(actions: impl IntoIterator<Item = &'a str>) -> Vec<Self> {
        let mut in_insert = false;
        let mut commands = Vec::new();
        for raw in actions {
            if in_insert {
                in_insert = raw != CMD_ESCAPE;
                continue;
            }
            let Some(command) = Self::from_action(raw) else {
                continue;
            };
            in_insert = enters_insert(command.as_str());
            commands.push(command);
        }
        commands
    }
}

impl TryFrom<String> for CommandId {
    type Error = IdError;

    fn try_from(id: String) -> Result<Self, IdError> {
        Self::from_card_id(&id).ok_or(IdError::NotCanonical)
    }
}

str_id_impls!(CommandId);

/// Register-scoped change (`"c`, normalized from `"ac`) also enters Insert mode.
fn enters_insert(card: &str) -> bool {
    INSERT_ENTRY_COMMANDS.contains(&card)
        || card.strip_prefix(CMD_SELECT_REGISTER) == Some(CMD_CHANGE)
}

fn search_command(cmd: &str) -> Option<&'static str> {
    [CMD_SEARCH_FORWARD, CMD_SEARCH_BACKWARD]
        .into_iter()
        .find(|prefix| {
            cmd.strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with(' '))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(actions: &[&str]) -> Vec<String> {
        CommandId::from_actions(actions.iter().copied())
            .into_iter()
            .map(CommandId::into_string)
            .collect()
    }

    #[test]
    fn spec_acceptance_table() {
        let actions = ["!", "Escape", " ", "3h", "f=", "T,", "r_", "/ sum", "ma("];
        assert_eq!(ids(&actions), ["h", "f", "T", "r", "/", "ma("]);
    }

    #[test]
    fn count_prefix_is_stripped_but_zero_is_a_command() {
        assert_eq!(ids(&["3fa", "12w", "0", "3"]), ["f", "w", "0"]);
    }

    #[test]
    fn counted_commands_collapse_to_the_base_command() {
        assert_eq!(ids(&["3d", "3fa", "2G", "10j"]), ["d", "f", "G", "j"]);
    }

    #[test]
    fn insert_mode_typing_creates_no_cards() {
        let actions = [
            "i", "h", "e", "!", "#", " ", "Escape", "x", "o", "y", "Escape",
        ];
        assert_eq!(ids(&actions), ["i", "x", "o"]);
    }

    #[test]
    fn register_scoped_change_enters_insert_mode() {
        assert_eq!(ids(&["\"ac", "h", "Escape", "x"]), ["\"c", "x"]);
        assert_eq!(ids(&["\"ad", "h", "\"ay"]), ["\"d", "h", "\"y"]);
    }

    #[test]
    fn unterminated_insert_swallows_the_tail() {
        assert_eq!(ids(&["A", "a", "b"]), ["A"]);
    }

    #[test]
    fn existing_normalizations_are_preserved() {
        assert_eq!(
            ids(&["\"ay", ":g 3", "s foo", "S ,", "Ctrl-b", "gg"]),
            ["\"y", ":goto", "s", "S", "Ctrl-b", "gg"]
        );
    }

    #[test]
    fn bare_find_and_replace_commands_are_kept() {
        assert_eq!(ids(&["f", "r", ".", "$"]), ["f", "r", ".", "$"]);
    }

    #[test]
    fn card_ids_minted_by_parse_are_canonical() {
        let actions = [
            "h", "dd", "f", "T", "/", "ma(", "\"y", ":goto", "s", "S", "Ctrl-b", "gg", "0", "$",
            ".", "?", "ж",
        ];
        for id in actions {
            assert_eq!(
                CommandId::from_card_id(id).as_ref().map(CommandId::as_str),
                Some(id),
                "{id}"
            );
        }
    }

    #[test]
    fn card_ids_rejects_raw_recordings() {
        for id in ["3h", "f=", "Escape", "!", " ", "", "/ sum", "12w"] {
            assert!(CommandId::from_card_id(id).is_none(), "{id:?}");
        }
    }

    #[test]
    fn overlong_command_line_mints_no_card() {
        let long = format!(":{}", "n".repeat(MAX_ID_LENGTH));
        assert!(CommandId::from_action(&long).is_none());
        assert!(CommandId::from_card_id(&long).is_none());
        let fits = format!(":{}", "n".repeat(MAX_ID_LENGTH - 1));
        assert!(CommandId::from_action(&fits).is_some());
    }

    #[test]
    fn deserialization_requires_canonical_form() {
        assert_eq!(
            serde_json::from_str::<CommandId>("\"gg\"")
                .unwrap()
                .as_str(),
            "gg"
        );
        assert!(serde_json::from_str::<CommandId>("\"3h\"").is_err());
        assert!(serde_json::from_str::<CommandId>("\"Escape\"").is_err());
    }

    #[test]
    fn multibyte_argument_does_not_panic() {
        assert_eq!(ids(&["fé", "ж"]), ["f", "ж"]);
    }

    /// The derivation as it was before the shared grammar replaced its own
    /// list of argument-taking keys; card ids are persisted, so the new one
    /// must agree with it on every input.
    fn legacy_from_action(raw: &str) -> Option<String> {
        const CHAR_ARGUMENT_COMMANDS: [char; 5] = ['f', 'F', 't', 'T', 'r'];
        let rest = if raw.starts_with(|c: char| ('1'..='9').contains(&c)) {
            raw.trim_start_matches(|c: char| c.is_ascii_digit())
        } else {
            raw
        };
        if rest.trim().is_empty() || rest == CMD_ESCAPE {
            return None;
        }
        let minted = if let Some(search) = search_command(rest) {
            search.to_string()
        } else {
            let mut chars = rest.chars();
            let first = chars.next()?;
            match (chars.next(), CHAR_ARGUMENT_COMMANDS.contains(&first)) {
                (Some(_), true) => first.to_string(),
                (None, _) if !first.is_alphanumeric() && !PUNCTUATION_COMMANDS.contains(first) => {
                    return None;
                }
                _ => normalize_command_id(rest).into_owned(),
            }
        };
        (minted.len() <= MAX_ID_LENGTH).then_some(minted)
    }

    fn corpus() -> Vec<String> {
        let alphabet: Vec<char> = "fFtTrgGdwxsSmaiAoOc0123456789\"/?:.,;<>=*&_~`$^%() \u{e9}"
            .chars()
            .collect();
        let mut corpus: Vec<String> = alphabet.iter().map(char::to_string).collect();
        for &a in &alphabet {
            for &b in &alphabet {
                corpus.push(format!("{a}{b}"));
                corpus.push(format!("3{a}{b}"));
            }
        }
        corpus.extend(
            [
                "Escape", "Alt-d", "Ctrl-r", ":goto 3", "s foo", "/ a b", "\"ay", "\"ac", "3\"ad",
            ]
            .map(String::from),
        );
        corpus
    }

    #[test]
    fn card_ids_agree_with_the_derivation_before_the_shared_grammar() {
        for raw in corpus() {
            assert_eq!(
                CommandId::from_action(&raw).map(CommandId::into_string),
                legacy_from_action(&raw),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn card_ids_agree_on_every_bundled_scenario_solution() {
        let scenarios = crate::config::ScenarioLoader::new()
            .load_from_embedded("en")
            .expect("embedded scenarios load");
        assert!(!scenarios.is_empty());
        for scenario in &scenarios {
            for raw in &scenario.solution.commands {
                assert_eq!(
                    CommandId::from_action(raw).map(CommandId::into_string),
                    legacy_from_action(raw),
                    "{}: {raw:?}",
                    scenario.id
                );
            }
        }
    }
}
