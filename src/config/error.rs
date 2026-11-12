//! The typed failure of the scenario and quest loaders.

use crate::config::quests::QuestTemplateError;
use crate::config::scenarios::ScenarioValidationError;
use crate::security::SecurityError;

/// Why a scenario or quest file could not be loaded.
///
/// Carries the precise cause for logs; the error hub converts it to the sanitized
/// [`UserError`](crate::error::UserError).
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Path, size, count, encoding or TOML validation failed.
    #[error(transparent)]
    Security(#[from] SecurityError),

    /// A quest template failed validation.
    #[error("Invalid quest template: {0}")]
    QuestTemplate(#[from] QuestTemplateError),

    /// A scenario's cursor setup or target failed validation.
    #[error("Invalid scenario: {0}")]
    Scenario(#[from] ScenarioValidationError),

    /// Reading a scenario directory or one of its entries failed.
    #[error("Failed to read the scenario directory: {0}")]
    Io(#[source] std::io::Error),

    /// No embedded or on-disk source exists for the request.
    #[error("No scenario or quest source found")]
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_template_error_keeps_its_message() {
        let err = ConfigError::from(QuestTemplateError::XpRewardTooHigh { max: 1, actual: 2 });
        assert!(err.to_string().contains("Invalid quest template"));
    }

    #[test]
    fn io_error_keeps_its_cause_as_the_source() {
        let err = ConfigError::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        assert!(std::error::Error::source(&err).is_some());
        assert!(!err.to_string().contains("No scenario"));
    }
}
