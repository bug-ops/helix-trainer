//! The user-facing error hub
//!
//! [`UserError`] is the sanitized error the UI shows. It sits above every domain module in
//! the layer order, so each domain returns its own typed error ([`SecurityError`],
//! [`GameError`](crate::game::GameError), [`ConfigError`](crate::config::ConfigError),
//! [`SimulatorError`], `GamificationError`,
//! `LearningError`) and the layers above convert it with `?` through the `From` impls here.

use crate::helix::error::{CommandLineError, SimulatorError};
use crate::helix::register::ReadOnlyRegister;
use crate::security::SecurityError;
use thiserror::Error;

/// What a user-facing error is, without its English text.
///
/// The UI resolves the text from the kind (`ui::text::user_error`), so a locale can
/// translate every error and tests match on the variant, never on a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserErrorKind {
    /// An internal invariant was violated.
    InvalidState,
    /// A message arrived for a screen other than the current one.
    WrongScreen,
    /// A scenario file could not be loaded or parsed.
    ScenarioLoad,
    /// A scenario file breaches a size or count limit.
    ScenarioTooComplex,
    /// A generic failure with no actionable detail.
    OperationFailed,
    /// The simulator rejected a command.
    Command(SimulatorErrorKind),
    /// Reading or writing the saved profile or configuration failed.
    Storage,
}

/// Why the simulator rejected a command, as owned data without English text.
///
/// One variant per [`SimulatorError`] and [`CommandLineError`] variant, so a new
/// simulator failure fails the build until it has a kind and a locale key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulatorErrorKind {
    /// The key names no registered command.
    UnknownCommand {
        /// The offending key, bounded and sanitized.
        command: String,
    },
    /// A count was given to a command that does not take one.
    CountNotAccepted,
    /// The pattern is not a valid regex.
    InvalidRegex {
        /// The regex engine's description of the problem.
        detail: String,
    },
    /// The pattern is longer than the allowed maximum.
    PatternTooLong {
        /// The maximum pattern length in characters.
        max: usize,
    },
    /// The bare command needs a pattern that only its prompt supplies.
    PatternRequired,
    /// `align` met a range that spans several lines.
    AlignSpansLines,
    /// The command would create more cursors than the simulator allows.
    TooManyCursors {
        /// The maximum number of cursors.
        max: usize,
    },
    /// The operation would leave no selection.
    NoSelections,
    /// A register prefix was followed by an operation that does not use registers.
    UnknownRegisterOp {
        /// The operation character.
        op: char,
    },
    /// A command tried to write to a register that Helix computes.
    ReadOnlyRegister {
        /// The register that rejected the write.
        register: ReadOnlyRegister,
    },
    /// A `:` command line reached Insert mode.
    CommandLineInInsertMode,
    /// The input did not start with `:`.
    NotACommand {
        /// The offending input.
        input: String,
    },
    /// Nothing followed the `:`.
    EmptyCommandLine,
    /// The command needs an argument and got none.
    MissingArgument {
        /// The command name.
        name: String,
    },
    /// The command got more than one argument.
    TooManyArguments {
        /// The command name.
        name: String,
    },
    /// The argument is not an unsigned number.
    NotANumber {
        /// The command name.
        name: String,
        /// The offending argument.
        argument: String,
    },
    /// The command name is not a supported typable command.
    UnknownCommandLine {
        /// The command name.
        name: String,
    },
}

impl From<&SimulatorError> for SimulatorErrorKind {
    fn from(error: &SimulatorError) -> Self {
        match error {
            SimulatorError::UnknownCommand { command } => Self::UnknownCommand {
                command: command.clone(),
            },
            SimulatorError::CountNotAccepted => Self::CountNotAccepted,
            SimulatorError::InvalidRegex(source) => Self::InvalidRegex {
                detail: source.to_string(),
            },
            SimulatorError::PatternTooLong { max } => Self::PatternTooLong { max: *max },
            SimulatorError::PatternRequired => Self::PatternRequired,
            SimulatorError::AlignSpansLines => Self::AlignSpansLines,
            SimulatorError::TooManyCursors { max } => Self::TooManyCursors { max: *max },
            SimulatorError::NoSelections => Self::NoSelections,
            SimulatorError::UnknownRegisterOp { op } => Self::UnknownRegisterOp { op: *op },
            SimulatorError::ReadOnlyRegister { register } => Self::ReadOnlyRegister {
                register: *register,
            },
            SimulatorError::CommandLineInInsertMode => Self::CommandLineInInsertMode,
            SimulatorError::CommandLine(error) => Self::from(error),
        }
    }
}

impl From<&CommandLineError> for SimulatorErrorKind {
    fn from(error: &CommandLineError) -> Self {
        match error {
            CommandLineError::NotACommand { input } => Self::NotACommand {
                input: input.clone(),
            },
            CommandLineError::Empty => Self::EmptyCommandLine,
            CommandLineError::MissingArgument { name } => {
                Self::MissingArgument { name: name.clone() }
            }
            CommandLineError::TooManyArguments { name } => {
                Self::TooManyArguments { name: name.clone() }
            }
            CommandLineError::NotANumber { name, argument } => Self::NotANumber {
                name: name.clone(),
                argument: argument.clone(),
            },
            CommandLineError::UnknownCommand { name } => {
                Self::UnknownCommandLine { name: name.clone() }
            }
        }
    }
}

/// Why a command failed to execute.
#[derive(Error, Debug)]
pub enum CommandFailure {
    /// The simulator rejected the command.
    #[error(transparent)]
    Simulator(#[from] SimulatorError),
}

/// Typed internal cause kept below a sanitized [`UserError`].
///
/// Never rendered to the user; it exists so [`error_chain`](crate::security::error_chain) can show the original failure in
/// logs after a domain error was reduced to generic user-facing text.
#[derive(Error, Debug)]
pub enum ErrorCause {
    /// Input validation or limit failure.
    #[error(transparent)]
    Security(#[from] SecurityError),

    /// Gamification (quests, streaks, achievements) failure.
    #[error(transparent)]
    Gamification(#[from] crate::gamification::GamificationError),

    /// Learning (FSRS, scheduling) failure.
    #[error(transparent)]
    Learning(#[from] crate::learning::LearningError),

    /// Helix simulator command failure.
    #[error(transparent)]
    Simulator(#[from] SimulatorError),

    /// Scenario or quest template validation failure.
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
}

/// User-facing error messages (sanitized)
///
/// The `Display` text is path-free and generic. Variants produced from a domain error keep it
/// as a typed [`ErrorCause`] source for logs.
#[derive(Error, Debug)]
pub enum UserError {
    /// An internal invariant was violated; the message is for logs, the
    /// displayed text is generic.
    #[error("Invalid application state. Please restart the application.")]
    InvalidState {
        /// Diagnostic detail that is never shown to the user.
        message: String,
        /// Original failure, if the state error was derived from one.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A message arrived for a screen other than the current one (e.g. a stale
    /// timer message racing a key-driven transition). Not a user-visible fault.
    #[error("That action is not available on the current screen.")]
    WrongScreen,

    /// A scenario file could not be loaded or parsed.
    #[error("Failed to load scenario file. Please check the file path and format.")]
    ScenarioLoadError {
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A scenario file breaches a size or count limit.
    #[error("The scenario file is too large or complex. Please use a smaller file.")]
    ScenarioTooComplex {
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A generic failure with no actionable detail for the user.
    #[error("Operation failed. Please try again.")]
    OperationFailed {
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
    },

    /// A Helix command failed to execute.
    #[error("Command execution failed: {failure}")]
    CommandFailed {
        /// What failed; the source of this error.
        #[source]
        failure: CommandFailure,
    },

    /// Reading or writing the saved profile or configuration failed. The cause is kept as a
    /// source for logs; the displayed text carries no paths or OS messages.
    #[error("Could not read or save your data. Please try again.")]
    Storage(#[source] crate::config::json_store::StorageError),
}

impl UserError {
    /// Create an `InvalidState` error with a descriptive message
    pub fn invalid_state(message: &str) -> Self {
        Self::InvalidState {
            message: message.to_string(),
            cause: None,
        }
    }

    /// What this error is, without its English text.
    #[must_use]
    pub fn kind(&self) -> UserErrorKind {
        match self {
            Self::InvalidState { .. } => UserErrorKind::InvalidState,
            Self::WrongScreen => UserErrorKind::WrongScreen,
            Self::ScenarioLoadError { .. } => UserErrorKind::ScenarioLoad,
            Self::ScenarioTooComplex { .. } => UserErrorKind::ScenarioTooComplex,
            Self::OperationFailed { .. } => UserErrorKind::OperationFailed,
            Self::CommandFailed {
                failure: CommandFailure::Simulator(error),
            } => UserErrorKind::Command(error.into()),
            Self::Storage(_) => UserErrorKind::Storage,
        }
    }

    /// A [`UserError::ScenarioLoadError`] without an underlying cause.
    #[must_use]
    pub fn scenario_load_error() -> Self {
        Self::ScenarioLoadError { cause: None }
    }

    /// A [`UserError::ScenarioTooComplex`] without an underlying cause.
    #[must_use]
    pub fn scenario_too_complex() -> Self {
        Self::ScenarioTooComplex { cause: None }
    }

    /// A [`UserError::OperationFailed`] without an underlying cause.
    #[must_use]
    pub fn operation_failed() -> Self {
        Self::OperationFailed { cause: None }
    }
}

impl From<SecurityError> for UserError {
    fn from(err: SecurityError) -> Self {
        match err {
            e @ (SecurityError::PathTraversal
            | SecurityError::InvalidPath
            | SecurityError::SuspiciousPath
            | SecurityError::Inaccessible(_)
            | SecurityError::InvalidToml(_)) => UserError::ScenarioLoadError {
                cause: Some(e.into()),
            },

            e @ (SecurityError::FileTooLarge { .. }
            | SecurityError::TooManyScenarios { .. }
            | SecurityError::ContentTooLarge { .. }
            | SecurityError::TooManyHints { .. }) => UserError::ScenarioTooComplex {
                cause: Some(e.into()),
            },

            e @ (SecurityError::TooManyActions { .. }
            | SecurityError::CommandSequenceTooLong { .. }
            | SecurityError::InvalidContent
            | SecurityError::InvalidEncoding(_)
            | SecurityError::TooManyKeymapBindings { .. }
            | SecurityError::NotRegularFile
            | SecurityError::ReadFailed(_)) => UserError::OperationFailed {
                cause: Some(e.into()),
            },
        }
    }
}

impl From<crate::gamification::GamificationError> for UserError {
    fn from(err: crate::gamification::GamificationError) -> Self {
        use crate::gamification::GamificationError;

        match err {
            e @ (GamificationError::StreakFreezeUnavailable
            | GamificationError::StreakFreezeNothingToProtect
            | GamificationError::StreakFreezeGapOutOfRange { .. }) => UserError::OperationFailed {
                cause: Some(e.into()),
            },
        }
    }
}

impl From<crate::learning::LearningError> for UserError {
    fn from(err: crate::learning::LearningError) -> Self {
        use crate::learning::LearningError;

        match err {
            e @ (LearningError::InvalidStability(_)
            | LearningError::InvalidDifficulty(_)
            | LearningError::InvalidCounters { .. }) => UserError::InvalidState {
                message: "Invalid learning parameters".to_string(),
                cause: Some(e.into()),
            },
            e @ LearningError::FsrsError(_) => UserError::OperationFailed {
                cause: Some(e.into()),
            },
        }
    }
}

impl From<crate::config::ConfigError> for UserError {
    fn from(err: crate::config::ConfigError) -> Self {
        use crate::config::ConfigError;

        match err {
            ConfigError::Security(e) => e.into(),
            ConfigError::NotFound => Self::scenario_load_error(),
            e @ ConfigError::Io(_) => Self::ScenarioLoadError {
                cause: Some(e.into()),
            },
            e @ (ConfigError::QuestTemplate(_) | ConfigError::Scenario(_)) => {
                Self::OperationFailed {
                    cause: Some(e.into()),
                }
            }
        }
    }
}

impl From<SimulatorError> for UserError {
    fn from(err: SimulatorError) -> Self {
        Self::CommandFailed {
            failure: CommandFailure::Simulator(err),
        }
    }
}

impl From<crate::game::GameError> for UserError {
    fn from(err: crate::game::GameError) -> Self {
        use crate::game::GameError;

        match err {
            GameError::Security(e) => e.into(),
            GameError::Simulator(e) => e.into(),
            GameError::NoActiveScenario => Self::operation_failed(),
        }
    }
}

impl From<crate::config::json_store::StorageError> for UserError {
    fn from(err: crate::config::json_store::StorageError) -> Self {
        Self::Storage(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::error_chain;
    use std::assert_matches;

    #[test]
    fn test_error_conversion_messages() {
        const SCENARIO_FILE: &str =
            "Failed to load scenario file. Please check the file path and format.";
        const TOO_LARGE: &str =
            "The scenario file is too large or complex. Please use a smaller file.";
        let cases = [
            (SecurityError::PathTraversal, SCENARIO_FILE),
            (SecurityError::InvalidPath, SCENARIO_FILE),
            (
                SecurityError::InvalidToml(Box::new(crate::security::sample_toml_error())),
                SCENARIO_FILE,
            ),
            (
                SecurityError::FileTooLarge {
                    max: 1000,
                    actual: 2000,
                },
                TOO_LARGE,
            ),
            (
                SecurityError::TooManyScenarios {
                    max: 100,
                    actual: 200,
                },
                TOO_LARGE,
            ),
            (
                SecurityError::ContentTooLarge {
                    max: 1000,
                    actual: 2000,
                },
                TOO_LARGE,
            ),
            (SecurityError::TooManyHints { max: 10 }, TOO_LARGE),
        ];
        for (err, expected) in cases {
            let label = format!("{err:?}");
            assert_eq!(UserError::from(err).to_string(), expected, "{label}");
        }
    }

    #[test]
    fn test_error_conversion_others_map_to_operation_failed() {
        let err = SecurityError::TooManyActions { max: 1_000_000 };
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");
    }

    #[test]
    fn test_user_error_invalid_state_constructor() {
        let err = UserError::invalid_state("test message");
        assert!(err.to_string().contains("Invalid application state"));
    }

    #[test]
    fn test_user_error_display() {
        assert!(
            UserError::invalid_state("test")
                .to_string()
                .contains("Invalid application state")
        );
        assert!(
            UserError::scenario_load_error()
                .to_string()
                .contains("load scenario")
        );
        assert!(
            UserError::scenario_too_complex()
                .to_string()
                .contains("too large")
        );
        assert!(
            UserError::operation_failed()
                .to_string()
                .contains("Operation failed")
        );
        assert!(
            UserError::from(SimulatorError::NoSelections)
                .to_string()
                .contains("Command")
        );
    }

    #[test]
    fn user_error_keeps_typed_cause_for_logs() {
        let user = UserError::from(SecurityError::TooManyActions { max: 1_000_000 });
        assert_eq!(user.to_string(), "Operation failed. Please try again.");
        assert_eq!(
            error_chain(&user),
            "Operation failed. Please try again.: Too many actions (max: 1000000)"
        );

        let user = UserError::from(SecurityError::InvalidToml(Box::new(
            crate::security::sample_toml_error(),
        )));
        assert!(error_chain(&user).contains("Invalid TOML format: "));
        assert!(error_chain(&user).contains("TOML parse error"));
        assert!(!user.to_string().contains("TOML parse error"));

        let user = UserError::from(crate::gamification::GamificationError::StreakFreezeUnavailable);
        assert_matches!(
            std::error::Error::source(&user).and_then(|e| e.downcast_ref::<ErrorCause>()),
            Some(ErrorCause::Gamification(_))
        );
    }

    #[test]
    fn error_chain_joins_every_cause() {
        let root = std::io::Error::other("disk full");
        let storage = crate::config::json_store::StorageError::Write {
            doc: crate::config::json_store::Document::Profile,
            path: "/secret/profile.json".into(),
            source: root,
        };
        let chain = error_chain(&storage);
        assert_eq!(
            chain,
            "failed to write the profile file /secret/profile.json: disk full"
        );

        let user = UserError::from(storage);
        assert!(!user.to_string().contains("/secret"), "{user}");
        assert_eq!(error_chain(&user).matches("disk full").count(), 1);
    }

    #[test]
    fn simulator_error_converts_to_command_failed_with_the_message() {
        let user = UserError::from(SimulatorError::NoSelections);
        assert_matches!(user, UserError::CommandFailed { .. });
        assert!(user.to_string().contains("no selections remaining"));
        assert_eq!(
            user.kind(),
            UserErrorKind::Command(SimulatorErrorKind::NoSelections)
        );
    }

    #[test]
    fn config_and_game_errors_keep_their_user_facing_kind() {
        use crate::config::ConfigError;
        use crate::game::GameError;

        assert_matches!(
            UserError::from(ConfigError::NotFound),
            UserError::ScenarioLoadError { cause: None }
        );
        assert_matches!(
            UserError::from(ConfigError::Io(std::io::Error::other("denied"))),
            UserError::ScenarioLoadError { cause: Some(_) }
        );
        assert_matches!(
            UserError::from(GameError::NoActiveScenario),
            UserError::OperationFailed { cause: None }
        );
        assert_matches!(
            UserError::from(GameError::Simulator(SimulatorError::NoSelections)),
            UserError::CommandFailed { .. }
        );
        assert_matches!(
            UserError::from(GameError::Security(SecurityError::TooManyActions {
                max: 1_000_000
            })),
            UserError::OperationFailed { .. }
        );
    }
}
