//! Test scenario builder for creating test fixtures
//!
//! Provides a fluent builder API for creating `Scenario` instances in tests.

use crate::config::{
    AlternativeSolution, Difficulty, Scenario, ScenarioCategory, ScenarioMetadata, ScoringConfig,
    Setup, Solution, TargetState,
};
use std::num::NonZeroUsize;

/// Builder for creating test scenarios with sensible defaults
///
/// # Example
/// ```rust,no_run
/// use helix_trainer::testing::ScenarioBuilder;
/// use helix_trainer::config::Difficulty;
///
/// let scenario = ScenarioBuilder::new()
///     .id("test_001")
///     .difficulty(Difficulty::Beginner)
///     .build();
/// ```
pub struct ScenarioBuilder {
    id: String,
    name: Option<String>,
    description: String,
    setup_content: String,
    setup_cursor: (usize, usize),
    setup_selection: Option<[usize; 4]>,
    target_content: String,
    target_cursor: (usize, usize),
    target_selection: Option<[usize; 4]>,
    commands: Vec<String>,
    command_description: String,
    alternatives: Vec<AlternativeSolution>,
    hints: Vec<String>,
    optimal_count: usize,
    max_points: u32,
    tolerance: usize,
    difficulty: Option<Difficulty>,
    category: Option<ScenarioCategory>,
    tags: Vec<String>,
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioBuilder {
    /// Create a new builder with sensible defaults
    pub fn new() -> Self {
        Self {
            id: "test_scenario".to_string(),
            name: None,
            description: "Test scenario description".to_string(),
            setup_content: "line 1\nline 2\nline 3\n".to_string(),
            setup_cursor: (0, 0),
            setup_selection: None,
            target_content: "line 2\nline 3\n".to_string(),
            target_cursor: (0, 0),
            target_selection: None,
            commands: vec!["x".to_string(), "d".to_string()],
            command_description: "Delete first line".to_string(),
            alternatives: vec![],
            hints: vec![],
            optimal_count: 2,
            max_points: 100,
            tolerance: 0,
            difficulty: None,
            category: None,
            tags: vec![],
        }
    }

    /// Set the scenario ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the scenario name (defaults to "Test Scenario {id}")
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the scenario description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the initial file content
    pub fn setup_content(mut self, content: impl Into<String>) -> Self {
        self.setup_content = content.into();
        self
    }

    /// Set the initial cursor position (row, col)
    pub fn setup_cursor(mut self, row: usize, col: usize) -> Self {
        self.setup_cursor = (row, col);
        self
    }

    /// Set the initial selection range
    ///
    /// # Arguments
    /// * `start` - Start position as (row, col)
    /// * `end` - End position as (row, col)
    pub fn setup_selection(mut self, start: (usize, usize), end: (usize, usize)) -> Self {
        self.setup_selection = Some([start.0, start.1, end.0, end.1]);
        self
    }

    /// Set the target file content
    pub fn target_content(mut self, content: impl Into<String>) -> Self {
        self.target_content = content.into();
        self
    }

    /// Set the target cursor position (row, col)
    pub fn target_cursor(mut self, row: usize, col: usize) -> Self {
        self.target_cursor = (row, col);
        self
    }

    /// Set the target selection range
    ///
    /// # Arguments
    /// * `start` - Start position as (row, col)
    /// * `end` - End position as (row, col)
    pub fn target_selection(mut self, start: (usize, usize), end: (usize, usize)) -> Self {
        self.target_selection = Some([start.0, start.1, end.0, end.1]);
        self
    }

    /// Set the solution commands
    pub fn commands(mut self, commands: Vec<impl Into<String>>) -> Self {
        self.commands = commands.into_iter().map(Into::into).collect();
        self
    }

    /// Set the solution description
    pub fn command_description(mut self, description: impl Into<String>) -> Self {
        self.command_description = description.into();
        self
    }

    /// Add a hint
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }

    /// Set all hints at once
    pub fn hints(mut self, hints: Vec<impl Into<String>>) -> Self {
        self.hints = hints.into_iter().map(Into::into).collect();
        self
    }

    /// Add an alternative solution
    ///
    /// # Arguments
    /// * `commands` - The alternative command sequence
    /// * `points_multiplier` - Score multiplier for this solution (e.g., 0.8 for 80%)
    /// * `description` - Description of the alternative approach
    pub fn alternative(
        mut self,
        commands: Vec<impl Into<String>>,
        points_multiplier: f32,
        description: impl Into<String>,
    ) -> Self {
        self.alternatives.push(AlternativeSolution {
            commands: commands.into_iter().map(Into::into).collect(),
            points_multiplier,
            description: description.into(),
        });
        self
    }

    /// Set the optimal command count
    ///
    /// # Panics
    /// [`Self::build`] panics if `count` is 0, since `ScoringConfig::optimal_count`
    /// requires a non-zero value.
    pub fn optimal_count(mut self, count: usize) -> Self {
        self.optimal_count = count;
        self
    }

    /// Set the maximum points
    pub fn max_points(mut self, points: u32) -> Self {
        self.max_points = points;
        self
    }

    /// Set the tolerance for scoring
    pub fn tolerance(mut self, tolerance: usize) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set the difficulty level
    pub fn difficulty(mut self, difficulty: Difficulty) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    /// Set the category
    pub fn category(mut self, category: ScenarioCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Build the scenario
    pub fn build(self) -> Scenario {
        let name = self
            .name
            .unwrap_or_else(|| format!("Test Scenario {}", self.id));

        let metadata =
            if self.difficulty.is_some() || self.category.is_some() || !self.tags.is_empty() {
                Some(ScenarioMetadata {
                    difficulty: self.difficulty,
                    category: self.category,
                    tags: self.tags,
                    ..Default::default()
                })
            } else {
                None
            };

        Scenario {
            id: self.id,
            name,
            description: self.description,
            setup: Setup {
                file_content: self.setup_content,
                cursor_position: Some(self.setup_cursor),
                selection: self.setup_selection,
                cursors: None,
                selections: None,
            },
            target: TargetState {
                file_content: self.target_content,
                cursor_position: Some(self.target_cursor),
                selection: self.target_selection,
                cursors: None,
                selections: None,
            },
            solution: Solution {
                commands: self.commands,
                description: self.command_description,
            },
            alternatives: self.alternatives,
            hints: self.hints,
            scoring: ScoringConfig {
                optimal_count: NonZeroUsize::new(self.optimal_count)
                    .expect("optimal_count must be non-zero"),
                max_points: self.max_points,
                tolerance: self.tolerance,
            },
            metadata,
        }
    }
}

/// Create a default test scenario with minimal configuration
///
/// Equivalent to `ScenarioBuilder::new().build()`
pub fn default_test_scenario() -> Scenario {
    ScenarioBuilder::new().build()
}

/// Create a test scenario with a specific ID
///
/// Equivalent to `ScenarioBuilder::new().id(id).build()`
pub fn test_scenario_with_id(id: impl Into<String>) -> Scenario {
    ScenarioBuilder::new().id(id).build()
}

/// Create a test scenario with ID and difficulty
///
/// Equivalent to `ScenarioBuilder::new().id(id).difficulty(difficulty).build()`
pub fn test_scenario_with_difficulty(id: impl Into<String>, difficulty: Difficulty) -> Scenario {
    ScenarioBuilder::new().id(id).difficulty(difficulty).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scenario_helper() {
        let scenario = default_test_scenario();
        assert_eq!(scenario.id, "test_scenario");
        assert_eq!(scenario.name, "Test Scenario test_scenario");
        assert!(scenario.metadata.is_none());
    }

    #[test]
    fn test_scenario_with_id_helper() {
        let scenario = test_scenario_with_id("custom_001");
        assert_eq!(scenario.id, "custom_001");
        assert_eq!(scenario.name, "Test Scenario custom_001");
    }

    #[test]
    fn test_scenario_with_difficulty_helper() {
        let scenario = test_scenario_with_difficulty("s1", Difficulty::Beginner);
        assert_eq!(scenario.id, "s1");
        assert!(scenario.metadata.is_some());
        assert_eq!(
            scenario.metadata.as_ref().unwrap().difficulty,
            Some(Difficulty::Beginner)
        );
    }

    #[test]
    fn test_full_builder() {
        let scenario = ScenarioBuilder::new()
            .id("full_test")
            .name("Full Test Scenario")
            .description("A complete test")
            .setup_content("hello")
            .setup_cursor(0, 0)
            .target_content("world")
            .target_cursor(0, 0)
            .commands(vec!["ciw", "world", "Esc"])
            .hint("Change the word")
            .difficulty(Difficulty::Advanced)
            .optimal_count(3)
            .build();

        assert_eq!(scenario.id, "full_test");
        assert_eq!(scenario.name, "Full Test Scenario");
        assert_eq!(scenario.setup.file_content, "hello");
        assert_eq!(scenario.target.file_content, "world");
        assert_eq!(scenario.hints.len(), 1);
        assert_eq!(scenario.scoring.optimal_count.get(), 3);
    }
}
