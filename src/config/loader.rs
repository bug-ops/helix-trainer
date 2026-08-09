//! Shared TOML parse-and-validate pipeline for scenario/quest files.

use crate::security::SecurityError;

/// TOML file wrapper carrying a list of domain items.
pub(crate) trait ItemsFile: serde::de::DeserializeOwned {
    type Item;
    fn into_items(self) -> Vec<Self::Item>;
}

/// Parses `content` as `F`, unwraps its items, enforces `max_count`, and
/// runs `validate` over each item.
///
/// Returns the unsanitized `SecurityError` (rather than `UserError`) so
/// callers can log the precise cause (e.g. `toml::de::Error` line/column)
/// before converting to a sanitized error for the user.
pub(crate) fn parse_and_validate<F>(
    content: &str,
    max_count: usize,
    validate: impl Fn(&F::Item) -> Result<(), SecurityError>,
) -> Result<Vec<F::Item>, SecurityError>
where
    F: ItemsFile,
{
    let file: F = toml::from_str(content).map_err(|e| SecurityError::InvalidToml(e.to_string()))?;

    let items = file.into_items();

    if items.len() > max_count {
        return Err(SecurityError::TooManyScenarios {
            max: max_count,
            actual: items.len(),
        });
    }

    for item in &items {
        validate(item)?;
    }

    Ok(items)
}

impl ItemsFile for super::scenarios::ScenariosFile {
    type Item = super::scenarios::Scenario;
    fn into_items(self) -> Vec<Self::Item> {
        self.scenarios
    }
}

impl ItemsFile for super::quests::QuestsFile {
    type Item = super::quests::QuestTemplate;
    fn into_items(self) -> Vec<Self::Item> {
        self.quests
    }
}

#[cfg(test)]
mod tests {
    use super::super::scenarios::ScenariosFile;
    use super::*;

    fn two_scenario_toml() -> String {
        let scenario = |id: &str| {
            format!(
                r#"
[[scenarios]]
id = "{id}"
name = "Test"
description = "Test"

[scenarios.setup]
file_content = "test"
cursor_position = [0, 0]

[scenarios.target]
file_content = "test"
cursor_position = [0, 0]

[scenarios.solution]
commands = ["test"]
description = "test"

[scenarios.scoring]
optimal_count = 1
max_points = 100
tolerance = 0
"#
            )
        };
        format!("{}\n{}", scenario("scenario_a"), scenario("scenario_b"))
    }

    #[test]
    fn parse_and_validate_returns_items_within_limit() {
        let toml = two_scenario_toml();
        let items = parse_and_validate::<ScenariosFile>(&toml, 2, |_| Ok(())).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_and_validate_rejects_invalid_toml() {
        let result = parse_and_validate::<ScenariosFile>("not valid toml {{{", 100, |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn parse_and_validate_rejects_over_max_count() {
        let toml = two_scenario_toml();
        let result = parse_and_validate::<ScenariosFile>(&toml, 1, |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn parse_and_validate_propagates_item_validation_error() {
        let toml = two_scenario_toml();
        let result = parse_and_validate::<ScenariosFile>(&toml, 2, |_| {
            Err(SecurityError::InvalidInput("bad item".to_string()))
        });
        assert!(result.is_err());
    }
}
