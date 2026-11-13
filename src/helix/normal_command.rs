//! The typed Normal-mode command
//!
//! Every Normal-mode command the input layer can produce is one value of the
//! closed enum [`NormalCommand`]. A command string (`3fa`, `"ay`, `:goto 3`)
//! is parsed into it exactly once, by [`NormalCommand::parse`]; its
//! [`Display`](fmt::Display) is the canonical string used for review-card
//! ids, key history and macros, so the string form is a rendering of the
//! value and never a second grammar. The simulator dispatches over the enum
//! exhaustively.

use std::fmt;
use std::num::NonZeroUsize;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::error::{SimulatorError, bound};
use super::registry::{normal_registry, parse_counted_command};
use super::simulator::InsertEntryCommand;
use super::simulator::command_line::CommandLine;
use crate::grammar::commands::{
    CMD_ESCAPE, CMD_GOTO_FILE_START, CMD_GOTO_LINE, CMD_KEEP_MATCHING, CMD_REMOVE_MATCHING,
    CMD_REPEAT, CMD_REPLAY_MACRO, CMD_SEARCH_BACKWARD, CMD_SEARCH_FORWARD, CMD_SELECT_REGEX,
    CMD_SELECT_REGISTER, CMD_SPLIT_SELECTION, CMD_TOGGLE_MACRO_RECORDING,
};
use crate::grammar::find::FindMotion;
use crate::ids::{CommandId, IdError, NormalizedCommand};

/// A command of the Normal-mode registry, by its canonical key.
///
/// Only the registry mints one ([`RegisteredCommand::lookup`]), so a value
/// always names a command that exists.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::normal_command::RegisteredCommand;
///
/// assert_eq!(RegisteredCommand::lookup("w").map(RegisteredCommand::key), Some("w"));
/// assert!(RegisteredCommand::lookup("no-such-command").is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisteredCommand(&'static str);

impl RegisteredCommand {
    /// The registered command with this key, if there is one.
    #[must_use]
    pub fn lookup(key: &str) -> Option<Self> {
        normal_registry().canonical_key(key).map(Self)
    }

    /// The command's key in the registry.
    #[must_use]
    pub fn key(self) -> &'static str {
        self.0
    }
}

/// The `Q` / `q` macro commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroOp {
    /// `Q`: start or stop recording.
    ToggleRecording,
    /// `q`: replay the recorded macro.
    Replay,
}

impl MacroOp {
    /// The key that runs the macro command.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::ToggleRecording => CMD_TOGGLE_MACRO_RECORDING,
            Self::Replay => CMD_REPLAY_MACRO,
        }
    }
}

/// A surround command (`ms`, `md`, `mr`) with its arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurroundOp {
    /// `ms<c>`: surround the selection with `c`.
    Add(char),
    /// `md<c>`: delete the surrounding `c` pair.
    Delete(char),
    /// `mr<from><to>`: replace the `from` pair with `to`.
    Replace {
        /// The pair to replace.
        from: char,
        /// The pair that takes its place.
        to: char,
    },
}

/// Whether a text object includes its delimiters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextObjectScope {
    /// `ma`: around, delimiters included.
    Around,
    /// `mi`: inside, delimiters excluded.
    Inside,
}

impl TextObjectScope {
    fn key(self) -> char {
        match self {
            Self::Around => 'a',
            Self::Inside => 'i',
        }
    }
}

/// What a register prefix (`"<reg>`) applies to. `c` is an Insert entry
/// ([`InsertEntryCommand::Change`]), not listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterOp {
    /// `y`: yank into the register.
    Yank,
    /// `p`: paste after from the register.
    PasteAfter,
    /// `P`: paste before from the register.
    PasteBefore,
    /// `R`: replace the selection with the register.
    ReplaceWithYanked,
    /// `d`: delete into the register.
    Delete,
}

impl RegisterOp {
    /// The operation bound to `key` after a register prefix.
    #[must_use]
    pub fn from_key(key: char) -> Option<Self> {
        match key {
            'y' => Some(Self::Yank),
            'p' => Some(Self::PasteAfter),
            'P' => Some(Self::PasteBefore),
            'R' => Some(Self::ReplaceWithYanked),
            'd' => Some(Self::Delete),
            _ => None,
        }
    }

    fn key(self) -> char {
        match self {
            Self::Yank => 'y',
            Self::PasteAfter => 'p',
            Self::PasteBefore => 'P',
            Self::ReplaceWithYanked => 'R',
            Self::Delete => 'd',
        }
    }
}

/// Which prompt command a pattern was typed for.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::normal_command::PromptKind;
///
/// assert_eq!(PromptKind::SearchBackward.command_id(), "?");
/// assert_eq!(PromptKind::RemoveSelections.command_id(), "Alt-K");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    /// `s` - select all regex matches within the current selection
    SelectRegex,
    /// `S` - split the current selection on a regex delimiter
    SplitSelection,
    /// `/` - select the next regex match after the cursor
    Search,
    /// `?` - select the previous regex match before the cursor
    SearchBackward,
    /// `K` - keep the selections whose text matches a regex
    KeepSelections,
    /// `Alt-K` - remove the selections whose text matches a regex
    RemoveSelections,
}

impl PromptKind {
    const ALL: [Self; 6] = [
        Self::SelectRegex,
        Self::SplitSelection,
        Self::Search,
        Self::SearchBackward,
        Self::KeepSelections,
        Self::RemoveSelections,
    ];

    /// The command id the prompt assembles its `"<id> <pattern>"` command from.
    #[must_use]
    pub fn command_id(self) -> &'static str {
        match self {
            Self::SelectRegex => CMD_SELECT_REGEX,
            Self::SplitSelection => CMD_SPLIT_SELECTION,
            Self::Search => CMD_SEARCH_FORWARD,
            Self::SearchBackward => CMD_SEARCH_BACKWARD,
            Self::KeepSelections => CMD_KEEP_MATCHING,
            Self::RemoveSelections => CMD_REMOVE_MATCHING,
        }
    }

    /// The single prompt character shown in the footer; `Alt-K` shows `!`
    /// (negated keep) to stay distinguishable from `K`.
    #[must_use]
    pub fn prefix(self) -> char {
        match self {
            Self::SelectRegex => 's',
            Self::SplitSelection => 'S',
            Self::Search => '/',
            Self::SearchBackward => '?',
            Self::KeepSelections => 'K',
            Self::RemoveSelections => '!',
        }
    }

    /// Whether Enter forwards an invalid pattern to the simulator so its
    /// `UserError` reaches the user (search prompts); the other prompts cancel.
    #[must_use]
    pub fn forwards_invalid_pattern(self) -> bool {
        matches!(self, Self::Search | Self::SearchBackward)
    }

    fn strip_from(cmd: &str) -> Option<(Self, &str)> {
        Self::ALL.into_iter().find_map(|kind| {
            cmd.strip_prefix(kind.command_id())
                .and_then(|rest| rest.strip_prefix(' '))
                .map(|pattern| (kind, pattern))
        })
    }
}

/// A Normal-mode command, closed over everything the input layer can produce.
///
/// # Examples
///
/// ```
/// use helix_trainer::helix::normal_command::NormalCommand;
///
/// let command = NormalCommand::parse("3fa")?;
/// assert_eq!(command.to_string(), "3fa");
/// assert_eq!(command.count().map(|c| c.get()), Some(3));
/// assert!(NormalCommand::parse("no-such-command").is_err());
/// # Ok::<(), helix_trainer::helix::error::SimulatorError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalCommand {
    /// A registry command, with an optional count prefix.
    Registered {
        /// The command.
        command: RegisteredCommand,
        /// The count prefix, applied by the command's count behavior.
        count: Option<NonZeroUsize>,
    },
    /// A command that enters Insert mode.
    InsertEntry(InsertEntryCommand),
    /// `.`: repeat the last Insert session.
    Repeat,
    /// `Q` or `q`.
    Macro(MacroOp),
    /// `f`, `F`, `t` or `T` with its character.
    Find {
        /// The motion.
        motion: FindMotion,
        /// The character searched for.
        ch: char,
        /// Which occurrence to land on.
        count: Option<NonZeroUsize>,
    },
    /// `G` / `gg` with a line: go to line `count` (bare `G` does nothing).
    GotoLine {
        /// The 1-based line.
        count: Option<NonZeroUsize>,
    },
    /// `r<c>`: replace the selection's characters with `c`.
    Replace(char),
    /// `ms`, `md` or `mr` with their arguments.
    Surround(SurroundOp),
    /// `ma<c>` / `mi<c>`: select a text object.
    TextObject {
        /// Around or inside.
        scope: TextObjectScope,
        /// The object's key (`w`, `(`, ...).
        object: char,
    },
    /// `"<reg><op>`: a register-scoped operation.
    Register {
        /// The register.
        register: char,
        /// The operation.
        op: RegisterOp,
    },
    /// `s`, `S`, `K`, `Alt-K`, `/` or `?` with its typed pattern.
    Prompt {
        /// Which prompt.
        kind: PromptKind,
        /// The pattern, as typed.
        pattern: String,
    },
    /// A `:` command line.
    CommandLine(CommandLine),
    /// `Escape` in Normal mode.
    Escape,
}

impl NormalCommand {
    /// Parse a command string.
    ///
    /// # Errors
    ///
    /// [`SimulatorError::UnknownCommand`] for a string that names no command,
    /// [`SimulatorError::CountNotAccepted`] for a count on a command that has
    /// none, [`SimulatorError::UnknownRegisterOp`] for a register prefix
    /// followed by an operation that does not use registers, and the
    /// [`CommandLineError`](crate::helix::error::CommandLineError) of a bad
    /// `:` command.
    pub fn parse(cmd: &str) -> Result<Self, SimulatorError> {
        if let Some(entry) = InsertEntryCommand::parse(cmd) {
            return Ok(Self::InsertEntry(entry));
        }
        match parse_counted_command(cmd) {
            Some((count, base)) => Self::parse_counted(count, base),
            None => Self::parse_uncounted(cmd),
        }
    }

    fn parse_counted(count: NonZeroUsize, base: &str) -> Result<Self, SimulatorError> {
        if base == CMD_GOTO_FILE_START {
            return Ok(Self::GotoLine { count: Some(count) });
        }
        match Self::parse_uncounted(base)? {
            Self::Registered { command, .. } => Ok(Self::Registered {
                command,
                count: Some(count),
            }),
            Self::Find { motion, ch, .. } => Ok(Self::Find {
                motion,
                ch,
                count: Some(count),
            }),
            Self::GotoLine { .. } => Ok(Self::GotoLine { count: Some(count) }),
            // Helix ignores the count of these
            command @ (Self::InsertEntry(_) | Self::Macro(_) | Self::Register { .. }) => {
                Ok(command)
            }
            Self::Repeat
            | Self::Replace(_)
            | Self::Surround(_)
            | Self::TextObject { .. }
            | Self::Prompt { .. }
            | Self::CommandLine(_)
            | Self::Escape => Err(SimulatorError::CountNotAccepted),
        }
    }

    fn parse_uncounted(cmd: &str) -> Result<Self, SimulatorError> {
        match cmd {
            CMD_REPEAT => return Ok(Self::Repeat),
            CMD_ESCAPE => return Ok(Self::Escape),
            CMD_TOGGLE_MACRO_RECORDING => return Ok(Self::Macro(MacroOp::ToggleRecording)),
            CMD_REPLAY_MACRO => return Ok(Self::Macro(MacroOp::Replay)),
            CMD_GOTO_LINE => return Ok(Self::GotoLine { count: None }),
            _ => {}
        }
        if let Some(entry) = InsertEntryCommand::parse(cmd) {
            return Ok(Self::InsertEntry(entry));
        }
        let chars: Vec<char> = cmd.chars().collect();
        match chars.as_slice() {
            ['r', ch] => return Ok(Self::Replace(*ch)),
            [key, ch] if let Some(motion) = FindMotion::from_key(*key) => {
                return Ok(Self::Find {
                    motion,
                    ch: *ch,
                    count: None,
                });
            }
            ['m', 's', ch] => return Ok(Self::Surround(SurroundOp::Add(*ch))),
            ['m', 'd', ch] => return Ok(Self::Surround(SurroundOp::Delete(*ch))),
            ['m', 'r', from, to] => {
                return Ok(Self::Surround(SurroundOp::Replace {
                    from: *from,
                    to: *to,
                }));
            }
            ['m', 'a', object] => {
                return Ok(Self::TextObject {
                    scope: TextObjectScope::Around,
                    object: *object,
                });
            }
            ['m', 'i', object] => {
                return Ok(Self::TextObject {
                    scope: TextObjectScope::Inside,
                    object: *object,
                });
            }
            _ => {}
        }
        if cmd.starts_with(CMD_SELECT_REGISTER)
            && let ['"', register, op] = chars.as_slice()
        {
            return RegisterOp::from_key(*op)
                .map(|op| Self::Register {
                    register: *register,
                    op,
                })
                .ok_or(SimulatorError::UnknownRegisterOp { op: *op });
        }
        if cmd.starts_with(':') {
            return Ok(Self::CommandLine(CommandLine::parse(cmd)?));
        }
        if let Some((kind, pattern)) = PromptKind::strip_from(cmd) {
            return Ok(Self::Prompt {
                kind,
                pattern: pattern.to_string(),
            });
        }
        RegisteredCommand::lookup(cmd)
            .map(|command| Self::Registered {
                command,
                count: None,
            })
            .ok_or_else(|| SimulatorError::UnknownCommand {
                command: bound(cmd),
            })
    }

    /// The count prefix, if the command carries one.
    #[must_use]
    pub fn count(&self) -> Option<NonZeroUsize> {
        match self {
            Self::Registered { count, .. }
            | Self::Find { count, .. }
            | Self::GotoLine { count } => *count,
            Self::InsertEntry(
                InsertEntryCommand::OpenBelow { count } | InsertEntryCommand::OpenAbove { count },
            ) => Some(*count),
            Self::InsertEntry(_)
            | Self::Repeat
            | Self::Macro(_)
            | Self::Replace(_)
            | Self::Surround(_)
            | Self::TextObject { .. }
            | Self::Register { .. }
            | Self::Prompt { .. }
            | Self::CommandLine(_)
            | Self::Escape => None,
        }
    }

    /// The same command without its count prefix.
    #[must_use]
    pub fn uncounted(&self) -> Self {
        match self {
            Self::Registered { command, .. } => Self::Registered {
                command: *command,
                count: None,
            },
            Self::Find { motion, ch, .. } => Self::Find {
                motion: *motion,
                ch: *ch,
                count: None,
            },
            Self::GotoLine { .. } => Self::GotoLine { count: None },
            Self::InsertEntry(entry) => Self::InsertEntry(entry.uncounted()),
            other => other.clone(),
        }
    }

    /// The review-card id of this command, or `None` when it makes no card
    /// (`Escape`).
    ///
    /// Counts, find arguments and prompt patterns are not part of the id.
    /// Every arm finishes through [`CommandId::from_action`], so the id
    /// grammar and its length cap stay in `ids`; the arms only pick the key
    /// that action sees.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::normal_command::NormalCommand;
    ///
    /// let id = |cmd: &str| NormalCommand::parse(cmd).unwrap().command_id();
    /// assert_eq!(id("3fa").map(|id| id.into_string()), Some("f".to_string()));
    /// assert_eq!(id("/ foo").map(|id| id.into_string()), Some("/".to_string()));
    /// assert_eq!(id("Escape"), None);
    /// ```
    #[must_use]
    pub fn command_id(&self) -> Option<CommandId> {
        match self {
            Self::Registered { command, .. } => CommandId::from_action(command.key()),
            Self::InsertEntry(entry) => CommandId::from_action(&entry.uncounted().command()),
            Self::Repeat => CommandId::from_action(CMD_REPEAT),
            Self::Macro(op) => CommandId::from_action(op.key()),
            Self::Find { motion, .. } => CommandId::from_action(&motion.key().to_string()),
            Self::GotoLine { .. } => CommandId::from_action(CMD_GOTO_LINE),
            Self::Replace(_) => CommandId::from_action("r"),
            // Their ids keep the argument, so the canonical spelling is the key.
            Self::Surround(_) | Self::TextObject { .. } | Self::Register { .. } => {
                CommandId::from_action(&self.to_string())
            }
            Self::Prompt { kind, .. } => CommandId::from_action(&format!("{} ", kind.command_id())),
            Self::CommandLine(CommandLine::Goto(_)) => CommandId::from_action(":goto"),
            Self::Escape => None,
        }
    }

    /// The quest-matching form of this command: its canonical spelling, normalized.
    ///
    /// # Errors
    ///
    /// [`IdError`] when the spelling normalizes to nothing or is too long.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::helix::normal_command::NormalCommand;
    ///
    /// let command = NormalCommand::parse("\"ay")?;
    /// assert_eq!(command.normalized()?.as_str(), "\"y");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn normalized(&self) -> Result<NormalizedCommand, IdError> {
        NormalizedCommand::from_raw(&self.to_string())
    }

    /// The keys that type this command, for `.` to replay; empty for a
    /// command `.` does not replay (counted commands, prompts, command lines).
    #[must_use]
    pub fn keys(&self) -> Vec<KeyEvent> {
        let plain = |ch: char| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
        let typed = |text: &str| text.chars().map(plain).collect::<Vec<_>>();
        match self {
            Self::Registered {
                command,
                count: None,
            } => registered_keys(command.key()),
            Self::InsertEntry(entry) => insert_entry_keys(*entry),
            Self::Repeat => typed(CMD_REPEAT),
            Self::Macro(op) => typed(op.key()),
            Self::Escape => vec![KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)],
            Self::Find {
                motion,
                ch,
                count: None,
            } => vec![plain(motion.key()), plain(*ch)],
            Self::GotoLine { count: None } => typed(CMD_GOTO_LINE),
            Self::Replace(ch) => vec![plain('r'), plain(*ch)],
            Self::Surround(SurroundOp::Add(ch)) => vec![plain('m'), plain('s'), plain(*ch)],
            Self::Surround(SurroundOp::Delete(ch)) => vec![plain('m'), plain('d'), plain(*ch)],
            Self::Surround(SurroundOp::Replace { from, to }) => {
                vec![plain('m'), plain('r'), plain(*from), plain(*to)]
            }
            Self::TextObject { scope, object } => {
                vec![plain('m'), plain(scope.key()), plain(*object)]
            }
            Self::Register { register, op } => {
                vec![plain('"'), plain(*register), plain(op.key())]
            }
            Self::Registered { count: Some(_), .. }
            | Self::Find { count: Some(_), .. }
            | Self::GotoLine { count: Some(_) }
            | Self::Prompt { .. }
            | Self::CommandLine(_) => Vec::new(),
        }
    }
}

/// The keys that type an Insert entry: one key, or `"<reg>c`; the counted and
/// modified forms are not replayed.
fn insert_entry_keys(entry: InsertEntryCommand) -> Vec<KeyEvent> {
    let plain = |ch: char| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
    match entry {
        InsertEntryCommand::Insert => vec![plain('i')],
        InsertEntryCommand::Append => vec![plain('a')],
        InsertEntryCommand::InsertLineStart => vec![plain('I')],
        InsertEntryCommand::AppendLineEnd => vec![plain('A')],
        InsertEntryCommand::OpenBelow { count } if count == NonZeroUsize::MIN => vec![plain('o')],
        InsertEntryCommand::OpenAbove { count } if count == NonZeroUsize::MIN => vec![plain('O')],
        InsertEntryCommand::Change { register: None } => vec![plain('c')],
        InsertEntryCommand::Change {
            register: Some(register),
        } => vec![plain('"'), plain(register), plain('c')],
        InsertEntryCommand::OpenBelow { .. }
        | InsertEntryCommand::OpenAbove { .. }
        | InsertEntryCommand::ChangeNoYank => Vec::new(),
    }
}

/// The keys that type a registered command: `gg`, `Alt-;`, `mm`, or a single
/// character; the other multi-key and modified commands are not replayed.
fn registered_keys(key: &str) -> Vec<KeyEvent> {
    let plain = |ch: char| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
    if key == crate::grammar::commands::CMD_FLIP_SELECTIONS {
        return vec![KeyEvent::new(KeyCode::Char(';'), KeyModifiers::ALT)];
    }
    let chars: Vec<char> = key.chars().collect();
    match chars.as_slice() {
        [first, second] if matches!(first, 'g' | 'm') => vec![plain(*first), plain(*second)],
        [ch] => vec![plain(*ch)],
        _ => Vec::new(),
    }
}

impl fmt::Display for NormalCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registered { command, count } => {
                write_count(f, *count)?;
                f.write_str(command.key())
            }
            Self::InsertEntry(entry) => f.write_str(&entry.command()),
            Self::Repeat => f.write_str(CMD_REPEAT),
            Self::Macro(op) => f.write_str(op.key()),
            Self::Find { motion, ch, count } => {
                write_count(f, *count)?;
                write!(f, "{}{ch}", motion.key())
            }
            Self::GotoLine { count } => {
                write_count(f, *count)?;
                f.write_str(CMD_GOTO_LINE)
            }
            Self::Replace(ch) => write!(f, "r{ch}"),
            Self::Surround(SurroundOp::Add(ch)) => write!(f, "ms{ch}"),
            Self::Surround(SurroundOp::Delete(ch)) => write!(f, "md{ch}"),
            Self::Surround(SurroundOp::Replace { from, to }) => write!(f, "mr{from}{to}"),
            Self::TextObject { scope, object } => write!(f, "m{}{object}", scope.key()),
            Self::Register { register, op } => {
                write!(f, "{CMD_SELECT_REGISTER}{register}{}", op.key())
            }
            Self::Prompt { kind, pattern } => write!(f, "{} {pattern}", kind.command_id()),
            Self::CommandLine(CommandLine::Goto(line)) => write!(f, ":goto {line}"),
            Self::Escape => f.write_str(CMD_ESCAPE),
        }
    }
}

fn write_count(f: &mut fmt::Formatter<'_>, count: Option<NonZeroUsize>) -> fmt::Result {
    match count {
        Some(count) => write!(f, "{count}"),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::assert_matches;

    fn nz(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).unwrap()
    }

    fn registered(key: &str, count: Option<usize>) -> NormalCommand {
        NormalCommand::Registered {
            command: RegisteredCommand::lookup(key).expect("registered key"),
            count: count.map(nz),
        }
    }

    #[test]
    fn canonical_strings_round_trip() {
        for cmd in [
            "h", "3h", "gg", "3G", "G", "fa", "3fa", "Tz", "rx", "r\n", "msx", "mdx", "mr()",
            "maw", "mi(", "\"ay", "\"_d", "s foo", "/ a b", "Alt-K x", ":goto 3", "Escape", ".",
            "q", "Q", "o", "2o", "3O", "\"ac", "Alt-d", "Ctrl-r", "i", "A",
        ] {
            let command = NormalCommand::parse(cmd).unwrap_or_else(|e| panic!("{cmd}: {e}"));
            assert_eq!(command.to_string(), cmd, "{cmd}");
            assert_eq!(NormalCommand::parse(&command.to_string()).unwrap(), command);
        }
    }

    #[test]
    fn non_canonical_spellings_parse_to_the_canonical_command() {
        assert_eq!(NormalCommand::parse("3gg").unwrap().to_string(), "3G");
        assert_eq!(NormalCommand::parse(":g 3").unwrap().to_string(), ":goto 3");
        assert_eq!(NormalCommand::parse("3i").unwrap().to_string(), "i");
    }

    #[test]
    fn parse_rejects_what_the_simulator_rejected() {
        assert_matches!(
            NormalCommand::parse("no-such"),
            Err(SimulatorError::UnknownCommand { .. })
        );
        assert_matches!(
            NormalCommand::parse("\"az"),
            Err(SimulatorError::UnknownRegisterOp { op: 'z' })
        );
        assert_matches!(
            NormalCommand::parse("3rx"),
            Err(SimulatorError::CountNotAccepted)
        );
        assert!(NormalCommand::parse("\u{e9}").is_err());
        assert!(NormalCommand::parse("\"").is_err());
        assert!(NormalCommand::parse("\"aay").is_err());
        assert!(NormalCommand::parse(":nope").is_err());
        assert!(NormalCommand::parse("s").is_err());
    }

    #[test]
    fn keys_match_the_keystrokes_dot_replays() {
        let codes = |command: &NormalCommand| -> Vec<KeyCode> {
            command.keys().iter().map(|key| key.code).collect()
        };
        assert_eq!(
            codes(&NormalCommand::parse("gg").unwrap()),
            [KeyCode::Char('g'), KeyCode::Char('g')]
        );
        assert_eq!(
            codes(&NormalCommand::parse("\"ay").unwrap()),
            [KeyCode::Char('"'), KeyCode::Char('a'), KeyCode::Char('y')]
        );
        assert_eq!(
            codes(&NormalCommand::parse("mr\u{e9}\u{1f600}").unwrap()),
            [
                KeyCode::Char('m'),
                KeyCode::Char('r'),
                KeyCode::Char('\u{e9}'),
                KeyCode::Char('\u{1f600}')
            ]
        );
        assert_eq!(
            codes(&NormalCommand::parse("Escape").unwrap()),
            [KeyCode::Esc]
        );
        let flip = NormalCommand::parse("Alt-;").unwrap().keys();
        assert_eq!(flip.len(), 1);
        assert_eq!(flip[0].modifiers, KeyModifiers::ALT);
        for not_replayed in ["3h", "3fa", ":goto 3", "s foo", "2o", "Alt-d"] {
            assert!(
                NormalCommand::parse(not_replayed)
                    .unwrap()
                    .keys()
                    .is_empty(),
                "{not_replayed}"
            );
        }
    }

    #[test]
    fn counts_are_exposed_and_removed() {
        let counted = registered("h", Some(3));
        assert_eq!(counted.count(), Some(nz(3)));
        assert_eq!(counted.uncounted(), registered("h", None));
        assert_eq!(NormalCommand::parse("2o").unwrap().count(), Some(nz(2)));
    }

    fn argument() -> impl Strategy<Value = char> {
        prop_oneof![
            prop::char::range('a', 'z'),
            Just('('),
            Just('"'),
            Just('\u{e9}'),
            Just('\u{1f600}'),
            Just(' '),
            Just('\n'),
        ]
    }

    fn command() -> impl Strategy<Value = NormalCommand> {
        let count = prop::option::of((1usize..=999).prop_map(nz));
        let motion = prop::sample::select(vec!['f', 'F', 't', 'T']);
        let key = prop::sample::select(vec!["h", "j", "w", "x", "d", "u", "Alt-d", "gh", "mm"]);
        let register_op = prop::sample::select(vec![
            RegisterOp::Yank,
            RegisterOp::PasteAfter,
            RegisterOp::PasteBefore,
            RegisterOp::ReplaceWithYanked,
            RegisterOp::Delete,
        ]);
        let kind = prop::sample::select(PromptKind::ALL.to_vec());
        prop_oneof![
            (key, count.clone()).prop_map(|(key, count)| NormalCommand::Registered {
                command: RegisteredCommand::lookup(key).unwrap(),
                count,
            }),
            (motion, argument(), count.clone()).prop_map(|(key, ch, count)| {
                NormalCommand::Find {
                    motion: FindMotion::from_key(key).unwrap(),
                    ch,
                    count,
                }
            }),
            count.prop_map(|count| NormalCommand::GotoLine { count }),
            argument().prop_map(NormalCommand::Replace),
            argument().prop_map(|ch| NormalCommand::Surround(SurroundOp::Add(ch))),
            (argument(), argument())
                .prop_map(|(from, to)| NormalCommand::Surround(SurroundOp::Replace { from, to })),
            (any::<bool>(), argument()).prop_map(|(around, object)| NormalCommand::TextObject {
                scope: if around {
                    TextObjectScope::Around
                } else {
                    TextObjectScope::Inside
                },
                object,
            }),
            (argument(), register_op)
                .prop_map(|(register, op)| NormalCommand::Register { register, op }),
            (kind, "[a-z ()]{1,8}")
                .prop_map(|(kind, pattern)| NormalCommand::Prompt { kind, pattern }),
            (1usize..=500).prop_map(|n| NormalCommand::CommandLine(CommandLine::Goto(n))),
        ]
    }

    fn replayable_forms() -> impl Strategy<Value = NormalCommand> {
        let fixed = prop::sample::select(vec![
            "i", "a", "I", "A", "o", "O", "c", "\"ac", "\"_c", ".", "q", "Q", "Alt-;", "Escape",
        ]);
        prop_oneof![
            command(),
            fixed.prop_map(|text| NormalCommand::parse(text).unwrap())
        ]
    }

    fn oracle_counts() -> [Option<NonZeroUsize>; 4] {
        [None, Some(nz(1)), Some(nz(2)), Some(nz(12))]
    }

    /// Every command shape the input layer builds, over the arguments that
    /// matter to the id grammar.
    fn exhaustive_commands() -> Vec<NormalCommand> {
        let mut all = vec![
            NormalCommand::Repeat,
            NormalCommand::Escape,
            NormalCommand::Macro(MacroOp::ToggleRecording),
            NormalCommand::Macro(MacroOp::Replay),
        ];
        for count in oracle_counts() {
            all.extend(
                normal_registry()
                    .all_commands()
                    .map(|meta| NormalCommand::Registered {
                        command: RegisteredCommand::lookup(meta.key()).unwrap(),
                        count,
                    }),
            );
            all.push(NormalCommand::GotoLine { count });
            for motion in ['f', 'F', 't', 'T'] {
                for ch in ['a', '=', ' '] {
                    all.push(NormalCommand::Find {
                        motion: FindMotion::from_key(motion).unwrap(),
                        ch,
                        count,
                    });
                }
            }
        }
        for count in [nz(1), nz(2), nz(12)] {
            all.push(NormalCommand::InsertEntry(InsertEntryCommand::OpenBelow {
                count,
            }));
            all.push(NormalCommand::InsertEntry(InsertEntryCommand::OpenAbove {
                count,
            }));
        }
        for entry in [
            InsertEntryCommand::Insert,
            InsertEntryCommand::Append,
            InsertEntryCommand::InsertLineStart,
            InsertEntryCommand::AppendLineEnd,
            InsertEntryCommand::ChangeNoYank,
            InsertEntryCommand::Change { register: None },
        ] {
            all.push(NormalCommand::InsertEntry(entry));
        }
        for register in ['_', 'a', '%'] {
            all.push(NormalCommand::InsertEntry(InsertEntryCommand::Change {
                register: Some(register),
            }));
        }
        for ch in ['x', ' ', '\u{e9}'] {
            all.push(NormalCommand::Replace(ch));
            all.push(NormalCommand::Surround(SurroundOp::Add(ch)));
            all.push(NormalCommand::Surround(SurroundOp::Delete(ch)));
            all.push(NormalCommand::Surround(SurroundOp::Replace {
                from: ch,
                to: '(',
            }));
        }
        for scope in [TextObjectScope::Around, TextObjectScope::Inside] {
            for object in ['w', '(', 'm', '"'] {
                all.push(NormalCommand::TextObject { scope, object });
            }
        }
        for op in [
            RegisterOp::Yank,
            RegisterOp::PasteAfter,
            RegisterOp::PasteBefore,
            RegisterOp::ReplaceWithYanked,
            RegisterOp::Delete,
        ] {
            for register in ['a', '_', '"', '%'] {
                all.push(NormalCommand::Register { register, op });
            }
        }
        for kind in PromptKind::ALL {
            for pattern in [String::new(), "foo".to_string(), "x".repeat(70)] {
                all.push(NormalCommand::Prompt { kind, pattern });
            }
        }
        for line in [1, 3, 500] {
            all.push(NormalCommand::CommandLine(CommandLine::Goto(line)));
        }
        all
    }

    #[test]
    fn typed_ids_equal_the_string_derivation_for_every_command() {
        for command in exhaustive_commands() {
            let text = command.to_string();
            assert_eq!(
                command.command_id(),
                CommandId::from_action(&text),
                "{text:?}"
            );
            assert_eq!(
                command.normalized(),
                NormalizedCommand::from_raw(&text),
                "{text:?}"
            );
        }
    }

    #[test]
    fn typed_ids_equal_the_string_derivation_for_every_bundled_solution_token() {
        let scenarios = crate::config::ScenarioLoader::new()
            .load_from_embedded("en")
            .expect("embedded scenarios load");
        let mut checked = 0;
        for token in scenarios.iter().flat_map(|s| &s.solution.commands) {
            if let Ok(command) = NormalCommand::parse(token) {
                assert_eq!(
                    command.command_id(),
                    CommandId::from_action(token),
                    "{token:?}"
                );
                assert_eq!(
                    command.normalized(),
                    NormalizedCommand::from_raw(&command.to_string()),
                    "{token:?}"
                );
                checked += 1;
            }
        }
        assert!(checked > 0);
    }

    proptest! {
        /// Every replayed key is a plain character, so a command that `.`
        /// replays types its own canonical spelling.
        #[test]
        fn plain_keys_spell_the_display_text(command in replayable_forms()) {
            let keys = command.keys();
            let plain: Option<String> = keys
                .iter()
                .map(|key| match (key.code, key.modifiers) {
                    (KeyCode::Char(ch), KeyModifiers::NONE) => Some(ch),
                    _ => None,
                })
                .collect();
            if let Some(typed) = plain.filter(|typed| !typed.is_empty()) {
                prop_assert_eq!(typed, command.to_string());
            }
        }

        /// Canonical spellings only (`:g 3` or `3gg` are not canonical): the
        /// string is a rendering of the value and parses back to it.
        #[test]
        fn typed_ids_equal_the_string_derivation(command in replayable_forms()) {
            let text = command.to_string();
            prop_assert_eq!(command.command_id(), CommandId::from_action(&text), "{}", text);
            prop_assert_eq!(command.normalized(), NormalizedCommand::from_raw(&text), "{}", text);
        }

        #[test]
        fn display_then_parse_is_the_identity(command in command()) {
            let text = command.to_string();
            prop_assert_eq!(NormalCommand::parse(&text).unwrap(), command, "{}", text);
        }
    }
}
