//! The typed failure of a play session, shared by training and arcade mode.

use crate::helix::error::SimulatorError;
use crate::security::SecurityError;

/// Why a session could not apply a command or start a scenario.
///
/// Carries the precise cause for logs; the error hub converts it to the sanitized
/// [`UserError`](crate::error::UserError).
#[derive(Debug, thiserror::Error)]
pub enum GameError {
    /// A limit (such as the action cap) or a validation failed.
    #[error(transparent)]
    Security(#[from] SecurityError),

    /// The simulator rejected the command.
    #[error(transparent)]
    Simulator(#[from] SimulatorError),

    /// The session has no scenario to run the command against.
    #[error("No active scenario")]
    NoActiveScenario,
}
