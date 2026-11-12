//! The user-facing error hub
//!
//! [`UserError`] is the sanitized error the UI shows. It sits above every domain module in
//! the layer order, so each domain returns its own typed error ([`SecurityError`],
//! [`GameError`](crate::game::GameError), [`ConfigError`](crate::config::ConfigError),
//! [`SimulatorError`](crate::helix::error::SimulatorError), `GamificationError`,
//! `LearningError`) and the layers above convert it with `?` through the `From` impls here.

use crate::security::SecurityError;
use thiserror::Error;

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
    Simulator(#[from] crate::helix::error::SimulatorError),

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

    /// A Helix command failed to execute in the simulator.
    #[error("Command execution failed: {context}")]
    CommandFailed {
        /// What was being executed, for debugging.
        context: String,
        /// Original failure, if any.
        #[source]
        cause: Option<ErrorCause>,
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

    /// Create a `CommandFailed` error with context for debugging
    pub fn command_failed(context: impl Into<String>) -> Self {
        Self::CommandFailed {
            context: context.into(),
            cause: None,
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
            | SecurityError::InvalidToml(_)) => UserError::ScenarioLoadError {
                cause: Some(e.into()),
            },

            e @ (SecurityError::FileTooLarge { .. }
            | SecurityError::TooManyScenarios { .. }
            | SecurityError::ContentTooLarge { .. }
            | SecurityError::TooManyHints { .. }) => UserError::ScenarioTooComplex {
                cause: Some(e.into()),
            },

            SecurityError::InvalidState(msg) => UserError::InvalidState {
                message: msg.clone(),
                cause: Some(SecurityError::InvalidState(msg).into()),
            },

            e @ (SecurityError::InvalidCursorPosition
            | SecurityError::TooManyActions
            | SecurityError::ScoreOverflow
            | SecurityError::CommandSequenceTooLong { .. }
            | SecurityError::InvalidContent
            | SecurityError::InvalidEncoding
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
            e @ GamificationError::InvalidLevel(_) => UserError::InvalidState {
                message: "Invalid level".to_string(),
                cause: Some(e.into()),
            },
            e @ (GamificationError::QuestNotFound(_)
            | GamificationError::StreakFreezeUnavailable
            | GamificationError::StreakFreezeNothingToProtect
            | GamificationError::StreakFreezeGapOutOfRange { .. }
            | GamificationError::AchievementAlreadyUnlocked(_)) => UserError::OperationFailed {
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
            e @ LearningError::CommandNotFound(_) => UserError::CommandFailed {
                context: "Command not found in registry".to_string(),
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
            e @ (ConfigError::QuestTemplate(_) | ConfigError::Scenario(_)) => {
                Self::OperationFailed {
                    cause: Some(e.into()),
                }
            }
        }
    }
}

impl From<crate::helix::error::SimulatorError> for UserError {
    fn from(err: crate::helix::error::SimulatorError) -> Self {
        Self::CommandFailed {
            context: err.to_string(),
            cause: Some(err.into()),
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
    use crate::helix::error::SimulatorError;
    use crate::security::error_chain;
    use std::assert_matches;

    #[test]
    fn test_error_conversion() {
        let err = SecurityError::PathTraversal;
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_invalid_path() {
        let err = SecurityError::InvalidPath;
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_suspicious_path() {
        let err = SecurityError::SuspiciousPath;
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_invalid_toml() {
        let err = SecurityError::InvalidToml("parse error".to_string());
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "Failed to load scenario file. Please check the file path and format."
        );
    }

    #[test]
    fn test_error_conversion_file_too_large() {
        let err = SecurityError::FileTooLarge {
            max: 1000,
            actual: 2000,
        };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_too_many_scenarios() {
        let err = SecurityError::TooManyScenarios {
            max: 100,
            actual: 200,
        };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_content_too_large() {
        let err = SecurityError::ContentTooLarge {
            max: 1000,
            actual: 2000,
        };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_too_many_hints() {
        let err = SecurityError::TooManyHints { max: 10 };
        let user_err = UserError::from(err);
        assert_eq!(
            user_err.to_string(),
            "The scenario file is too large or complex. Please use a smaller file."
        );
    }

    #[test]
    fn test_error_conversion_invalid_state() {
        let err = SecurityError::InvalidState("bad state".to_string());
        let user_err = UserError::from(err);
        assert!(user_err.to_string().contains("Invalid application state"));
    }

    #[test]
    fn test_error_conversion_others_map_to_operation_failed() {
        let err = SecurityError::InvalidCursorPosition;
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");

        let err = SecurityError::ScoreOverflow;
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");

        let err = SecurityError::TooManyActions;
        let user_err = UserError::from(err);
        assert_eq!(user_err.to_string(), "Operation failed. Please try again.");
    }

    #[test]
    fn test_user_error_invalid_state_constructor() {
        let err = UserError::invalid_state("test message");
        assert!(err.to_string().contains("Invalid application state"));
    }

    #[test]
    fn test_user_error_command_failed_constructor() {
        let err = UserError::command_failed("test context");
        assert!(err.to_string().contains("Command execution failed"));
        assert!(err.to_string().contains("test context"));
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
            UserError::command_failed("test")
                .to_string()
                .contains("Command")
        );
    }

    #[test]
    fn user_error_keeps_typed_cause_for_logs() {
        let user = UserError::from(SecurityError::TooManyActions);
        assert_eq!(user.to_string(), "Operation failed. Please try again.");
        assert_eq!(
            error_chain(&user),
            "Operation failed. Please try again.: Too many actions (max: 1000000)"
        );

        let user = UserError::from(SecurityError::InvalidToml("bad key".to_string()));
        assert!(error_chain(&user).ends_with("Invalid TOML format: bad key"));
        assert!(!user.to_string().contains("bad key"));

        let user = UserError::from(SecurityError::InvalidState("broken".to_string()));
        assert!(error_chain(&user).contains("Invalid state: broken"));

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
        assert_matches!(
            std::error::Error::source(&user).and_then(|e| e.downcast_ref::<ErrorCause>()),
            Some(ErrorCause::Simulator(SimulatorError::NoSelections))
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
            UserError::from(GameError::NoActiveScenario),
            UserError::OperationFailed { cause: None }
        );
        assert_matches!(
            UserError::from(GameError::Simulator(SimulatorError::NoSelections)),
            UserError::CommandFailed { .. }
        );
        assert_matches!(
            UserError::from(GameError::Security(SecurityError::TooManyActions)),
            UserError::OperationFailed { .. }
        );
    }
}
