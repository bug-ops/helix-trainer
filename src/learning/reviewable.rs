//! Typed review-card identity: which recorded keystrokes are Helix commands worth practicing.
//!
//! A scenario solution records every keystroke, including text typed in Insert mode and
//! command arguments. [`ReviewableCommand`] is the only way to mint a review-card id from
//! such a recording, so typed text, `Escape` and argument characters can never become cards.
//! It is separate from [`crate::helix::commands::normalize_command_id`], whose contract
//! (quests, minigame scorer) is unchanged.

use crate::helix::commands::{
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

const CHAR_ARGUMENT_COMMANDS: [char; 5] = ['f', 'F', 't', 'T', 'r'];

/// Single non-alphanumeric characters that are Helix normal-mode commands.
const PUNCTUATION_COMMANDS: &str = "$^%;,.<>~`&_*/?()=";

/// A Helix command that may become an FSRS review card.
///
/// # Examples
///
/// ```
/// use helix_trainer::learning::ReviewableCommand;
///
/// let ids: Vec<String> = ReviewableCommand::from_actions(["3h", "f=", "/ sum", "Escape", "!"])
///     .into_iter()
///     .map(ReviewableCommand::into_string)
///     .collect();
/// assert_eq!(ids, ["h", "f", "/"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReviewableCommand(String);

impl ReviewableCommand {
    /// Card id of this command.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the command, yielding its card id.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }

    /// Mint a card id from one recorded action, ignoring Insert-mode context.
    ///
    /// Returns `None` for blank input, `Escape`, a bare count, and single characters that
    /// are not Helix commands. Strips a count prefix (`3h` -> `h`), drops the argument of
    /// find/till/replace (`f=` -> `f`) and of searches (`/ sum` -> `/`), keeps surround
    /// forms (`ma(`), and applies [`normalize_command_id`] to the rest.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        let rest = if raw.starts_with(|c: char| ('1'..='9').contains(&c)) {
            raw.trim_start_matches(|c: char| c.is_ascii_digit())
        } else {
            raw
        };
        if rest.trim().is_empty() || rest == CMD_ESCAPE {
            return None;
        }
        if let Some(search) = search_command(rest) {
            return Some(Self(search.to_string()));
        }
        let mut chars = rest.chars();
        let first = chars.next()?;
        match (chars.next(), CHAR_ARGUMENT_COMMANDS.contains(&first)) {
            (Some(_), true) => Some(Self(first.to_string())),
            (None, _) if !first.is_alphanumeric() && !PUNCTUATION_COMMANDS.contains(first) => None,
            _ => Some(Self(normalize_command_id(rest).into_owned())),
        }
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
            let Some(command) = Self::parse(raw) else {
                continue;
            };
            in_insert = enters_insert(command.as_str());
            commands.push(command);
        }
        commands
    }
}

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
        ReviewableCommand::from_actions(actions.iter().copied())
            .into_iter()
            .map(ReviewableCommand::into_string)
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
    fn multibyte_argument_does_not_panic() {
        assert_eq!(ids(&["fé", "ж"]), ["f", "ж"]);
    }
}
