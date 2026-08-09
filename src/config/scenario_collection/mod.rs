//! Scenario collection with filtering and sorting capabilities
//!
//! This module provides a wrapper around a Vec of scenarios with support for:
//! - Filtering by category, difficulty, commands, completion status
//! - Sorting by name, difficulty, category, progress, mastery
//! - Efficient index mapping for menu display

use super::scenarios::{Difficulty, Scenario, ScenarioCategory};
use crate::gamification::UserProfile;
use std::collections::HashSet;

/// Collection of scenarios with filtering and sorting support
#[derive(Debug, Clone)]
pub struct ScenarioCollection {
    /// All scenarios (immutable after creation)
    scenarios: Vec<Scenario>,

    /// Indices of scenarios that pass the current filter (maps to scenarios vec)
    filtered_indices: Vec<usize>,

    /// Currently active filter
    active_filter: ScenarioFilter,

    /// Currently active sort mode
    active_sort: SortMode,
}

/// Filter criteria for scenarios
#[derive(Debug, Clone, Default)]
pub struct ScenarioFilter {
    /// Include only scenarios in these categories (None = all categories)
    pub categories: Option<HashSet<ScenarioCategory>>,

    /// Include only scenarios at these difficulty levels (None = all difficulties)
    pub difficulties: Option<HashSet<Difficulty>>,

    /// Include only scenarios teaching these commands (None = no filter)
    pub commands: Option<HashSet<String>>,

    /// Completion-status criterion (defaults to [`CompletionFilter::Any`])
    pub completion: CompletionFilter,

    /// Include only scenarios with prerequisites met (None = no check)
    pub has_prerequisites: Option<bool>,

    /// Include only scenarios with specific tags
    pub tags: Option<HashSet<String>>,
}

/// Completion-status criterion for [`ScenarioFilter`].
///
/// Replaces the previous pair of independent `completed_only` /
/// `not_completed_only` booleans, whose both-true combination silently
/// filtered out every scenario. The three states are mutually exclusive by
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompletionFilter {
    /// No completion-status restriction.
    #[default]
    Any,

    /// Keep only scenarios present in the profile's completion history.
    CompletedOnly,

    /// Keep only scenarios absent from the profile's completion history.
    NotCompletedOnly,
}

/// Sorting strategies for scenario display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    /// Alphabetical by name
    ByName,

    /// Beginner → Intermediate → Advanced (default for no metadata)
    ByDifficulty,

    /// Group by category (Movement, Editing, etc.)
    ByCategory,

    /// Uncompleted → Completed (requires profile)
    ByProgress,

    /// Category groups, then difficulty within each
    ByCategoryThenDifficulty,

    /// Difficulty first, then category within each (recommended for training)
    #[default]
    ByDifficultyThenCategory,

    /// Weak (low mastery) → Strong (requires profile)
    ByMastery,
}

impl ScenarioCollection {
    /// Create a new collection from a vector of scenarios
    ///
    /// Applies the default sort mode (ByDifficultyThenCategory) on creation.
    pub fn new(scenarios: Vec<Scenario>) -> Self {
        let count = scenarios.len();
        let filtered_indices: Vec<usize> = (0..count).collect();

        let mut collection = Self {
            scenarios,
            filtered_indices,
            active_filter: ScenarioFilter::default(),
            active_sort: SortMode::default(),
        };

        // Apply default sort on creation
        collection.sort(SortMode::default(), None);
        collection
    }

    /// Apply a filter to the collection
    ///
    /// This updates the internal filtered_indices to only include scenarios
    /// that match all specified criteria.
    pub fn apply_filter(&mut self, filter: &ScenarioFilter, profile: Option<&UserProfile>) {
        self.active_filter = filter.clone();

        self.filtered_indices = self
            .scenarios
            .iter()
            .enumerate()
            .filter(|(_, scenario)| self.matches_filter(scenario, filter, profile))
            .map(|(idx, _)| idx)
            .collect();
    }

    /// Sort the filtered scenarios according to the specified mode
    pub fn sort(&mut self, mode: SortMode, profile: Option<&UserProfile>) {
        self.active_sort = mode;

        match mode {
            SortMode::ByName => {
                self.filtered_indices
                    .sort_by(|&a, &b| self.scenarios[a].name.cmp(&self.scenarios[b].name));
            }

            SortMode::ByDifficulty => {
                self.filtered_indices.sort_by_key(|&idx| {
                    self.scenarios[idx]
                        .metadata
                        .as_ref()
                        .and_then(|m| m.difficulty)
                        .unwrap_or(Difficulty::Beginner) // Default to beginner if no metadata
                });
            }

            SortMode::ByCategory => {
                self.filtered_indices.sort_by_key(|&idx| {
                    // Sort by category as integers (based on enum order)
                    self.scenarios[idx]
                        .metadata
                        .as_ref()
                        .and_then(|m| m.category)
                        .map(|c| c as u8)
                        .unwrap_or(255) // Put scenarios without category at the end
                });
            }

            SortMode::ByProgress => {
                if let Some(prof) = profile {
                    self.filtered_indices.sort_by_key(|&idx| {
                        // Completed scenarios go to the end
                        let scenario_id = &self.scenarios[idx].id;
                        prof.scenario_history.get(scenario_id).is_some()
                    });
                }
                // If no profile, don't change order
            }

            SortMode::ByCategoryThenDifficulty => {
                self.filtered_indices.sort_by_key(|&idx| {
                    let metadata = self.scenarios[idx].metadata.as_ref();
                    let category = metadata
                        .and_then(|m| m.category)
                        .map(|c| c as u8)
                        .unwrap_or(255);
                    let difficulty = metadata
                        .and_then(|m| m.difficulty)
                        .unwrap_or(Difficulty::Beginner);
                    (category, difficulty)
                });
            }

            SortMode::ByDifficultyThenCategory => {
                self.filtered_indices.sort_by_key(|&idx| {
                    let metadata = self.scenarios[idx].metadata.as_ref();
                    let difficulty = metadata
                        .and_then(|m| m.difficulty)
                        .unwrap_or(Difficulty::Beginner);
                    let category = metadata
                        .and_then(|m| m.category)
                        .map(|c| c as u8)
                        .unwrap_or(255);
                    (difficulty, category)
                });
            }

            SortMode::ByMastery => {
                if let Some(prof) = profile {
                    self.filtered_indices.sort_by_key(|&idx| {
                        // Sort by completion count (fewer completions = lower mastery = higher priority)
                        let scenario_id = &self.scenarios[idx].id;
                        prof.scenario_history
                            .get(scenario_id)
                            .map(|completion| completion.attempts)
                            .unwrap_or(0) // Never attempted = highest priority
                    });
                }
                // If no profile, don't change order
            }
        }
    }

    /// Get filtered scenarios (references)
    pub fn get_filtered(&self) -> Vec<&Scenario> {
        self.filtered_indices
            .iter()
            .map(|&idx| &self.scenarios[idx])
            .collect()
    }

    /// Get a specific filtered scenario by its index in the filtered list
    pub fn get_filtered_by_index(&self, filtered_idx: usize) -> Option<&Scenario> {
        self.filtered_indices
            .get(filtered_idx)
            .map(|&original_idx| &self.scenarios[original_idx])
    }

    /// Get total count of filtered scenarios
    pub fn count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get total count of all scenarios (unfiltered)
    pub fn total_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Reset filter to show all scenarios
    pub fn reset_filter(&mut self) {
        self.active_filter = ScenarioFilter::default();
        self.filtered_indices = (0..self.scenarios.len()).collect();
    }

    /// Get all unique categories present in the collection
    pub fn get_categories(&self) -> Vec<ScenarioCategory> {
        let mut categories: Vec<ScenarioCategory> = self
            .scenarios
            .iter()
            .filter_map(|s| s.metadata.as_ref())
            .filter_map(|m| m.category)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        categories.sort_by_key(|c| *c as u8);
        categories
    }

    /// Get all unique difficulties present in the collection
    pub fn get_difficulties(&self) -> Vec<Difficulty> {
        let mut difficulties: Vec<Difficulty> = self
            .scenarios
            .iter()
            .filter_map(|s| s.metadata.as_ref())
            .filter_map(|m| m.difficulty)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        difficulties.sort();
        difficulties
    }

    /// Check if a scenario matches the current filter
    fn matches_filter(
        &self,
        scenario: &Scenario,
        filter: &ScenarioFilter,
        profile: Option<&UserProfile>,
    ) -> bool {
        // Check category filter
        if let Some(ref categories) = filter.categories {
            let scenario_category = scenario.metadata.as_ref().and_then(|m| m.category);

            match scenario_category {
                Some(cat) if categories.contains(&cat) => {} // Pass
                _ => return false, // Scenario has no category or wrong category
            }
        }

        // Check difficulty filter
        if let Some(ref difficulties) = filter.difficulties {
            let scenario_difficulty = scenario.metadata.as_ref().and_then(|m| m.difficulty);

            match scenario_difficulty {
                Some(diff) if difficulties.contains(&diff) => {} // Pass
                _ => return false,                               // Wrong difficulty
            }
        }

        // Check commands filter (scenario must teach at least one of the commands)
        if let Some(ref commands) = filter.commands {
            let scenario_commands = scenario
                .metadata
                .as_ref()
                .map(|m| &m.commands_taught)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if !scenario_commands.iter().any(|cmd| commands.contains(cmd)) {
                return false;
            }
        }

        // Check tags filter (scenario must have at least one matching tag)
        if let Some(ref tags) = filter.tags {
            let scenario_tags = scenario
                .metadata
                .as_ref()
                .map(|m| &m.tags)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if !scenario_tags.iter().any(|tag| tags.contains(tag)) {
                return false;
            }
        }

        // Check completion status
        if let Some(prof) = profile {
            let is_completed = prof.scenario_history.get(&scenario.id).is_some();

            let keep = match filter.completion {
                CompletionFilter::Any => true,
                CompletionFilter::CompletedOnly => is_completed,
                CompletionFilter::NotCompletedOnly => !is_completed,
            };
            if !keep {
                return false;
            }
        }

        // Check prerequisites filter
        if let Some(has_prereqs) = filter.has_prerequisites {
            let scenario_has_prereqs = scenario
                .metadata
                .as_ref()
                .map(|m| !m.prerequisites.is_empty())
                .unwrap_or(false);

            if has_prereqs != scenario_has_prereqs {
                return false;
            }
        }

        true
    }

    /// Get current active filter
    pub fn active_filter(&self) -> &ScenarioFilter {
        &self.active_filter
    }

    /// Get current active sort mode
    pub fn active_sort(&self) -> SortMode {
        self.active_sort
    }

    /// Whether a scenario has at least one recorded completion in `profile`.
    ///
    /// Shared by [`Self::completed_count`] (fast, per-frame safe) and
    /// [`Self::curriculum_stats`] (single-pass, allocates `per_category`) so the
    /// two can never disagree on what counts as "completed".
    fn is_scenario_completed(scenario: &Scenario, profile: &UserProfile) -> bool {
        profile.scenario_history.get(&scenario.id).is_some()
    }

    /// Number of unfiltered scenarios with at least one recorded completion.
    ///
    /// Cheap enough to call every render frame (a `HashMap` lookup per scenario,
    /// no allocation) — used by [`Self::is_curriculum_complete`].
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::config::{
    ///     CursorSpec, Scenario, ScenarioCollection, ScoringConfig, Setup, Solution, TargetState,
    /// };
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// fn minimal_scenario(id: &str) -> Scenario {
    ///     let cursor = CursorSpec {
    ///         cursor_position: Some((0, 0)),
    ///         selection: None,
    ///         cursors: None,
    ///         selections: None,
    ///     };
    ///     Scenario {
    ///         id: id.to_string(),
    ///         name: "Test".to_string(),
    ///         description: "Test".to_string(),
    ///         setup: Setup { file_content: "test".to_string(), cursor: cursor.clone() },
    ///         target: TargetState { file_content: "test".to_string(), cursor },
    ///         solution: Solution { commands: vec!["x".to_string()], description: "Test".to_string() },
    ///         alternatives: vec![],
    ///         hints: vec![],
    ///         scoring: ScoringConfig {
    ///             optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
    ///             max_points: 100,
    ///             tolerance: 0,
    ///         },
    ///         metadata: None,
    ///     }
    /// }
    ///
    /// let collection = ScenarioCollection::new(vec![minimal_scenario("s1")]);
    /// let mut profile = UserProfile::new();
    /// assert_eq!(collection.completed_count(&profile), 0);
    ///
    /// profile.scenario_history.record_completion("s1", 100, 50, Utc::now());
    /// assert_eq!(collection.completed_count(&profile), 1);
    /// ```
    pub fn completed_count(&self, profile: &UserProfile) -> usize {
        self.scenarios
            .iter()
            .filter(|s| Self::is_scenario_completed(s, profile))
            .count()
    }

    /// Whether every scenario in the collection has been completed at least once.
    ///
    /// False for an empty collection — a failed scenario load must never
    /// celebrate curriculum completion.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::config::{
    ///     CursorSpec, Scenario, ScenarioCollection, ScoringConfig, Setup, Solution, TargetState,
    /// };
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// fn minimal_scenario(id: &str) -> Scenario {
    ///     let cursor = CursorSpec {
    ///         cursor_position: Some((0, 0)),
    ///         selection: None,
    ///         cursors: None,
    ///         selections: None,
    ///     };
    ///     Scenario {
    ///         id: id.to_string(),
    ///         name: "Test".to_string(),
    ///         description: "Test".to_string(),
    ///         setup: Setup { file_content: "test".to_string(), cursor: cursor.clone() },
    ///         target: TargetState { file_content: "test".to_string(), cursor },
    ///         solution: Solution { commands: vec!["x".to_string()], description: "Test".to_string() },
    ///         alternatives: vec![],
    ///         hints: vec![],
    ///         scoring: ScoringConfig {
    ///             optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
    ///             max_points: 100,
    ///             tolerance: 0,
    ///         },
    ///         metadata: None,
    ///     }
    /// }
    ///
    /// let collection = ScenarioCollection::new(vec![minimal_scenario("s1")]);
    /// let mut profile = UserProfile::new();
    /// assert!(!collection.is_curriculum_complete(&profile));
    ///
    /// profile.scenario_history.record_completion("s1", 100, 50, Utc::now());
    /// assert!(collection.is_curriculum_complete(&profile));
    ///
    /// // An empty collection is never complete, even with no scenarios to finish.
    /// assert!(!ScenarioCollection::new(vec![]).is_curriculum_complete(&profile));
    /// ```
    pub fn is_curriculum_complete(&self, profile: &UserProfile) -> bool {
        self.total_count() > 0 && self.completed_count(profile) == self.total_count()
    }

    /// Join the unfiltered scenario set against the profile's completion
    /// history in a single pass, computing per-category and overall mastery
    /// counts for the end-game summary screen.
    ///
    /// Scenarios with no `metadata.category` (both are `Option`) are counted
    /// in `total`/`perfected` but excluded from `per_category` — there is no
    /// category to attribute them to. Every scenario shipped today has a
    /// category, so this is currently a no-op exclusion.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::config::{
    ///     CursorSpec, Scenario, ScenarioCategory, ScenarioCollection, ScenarioMetadata,
    ///     ScoringConfig, Setup, Solution, TargetState,
    /// };
    /// use helix_trainer::gamification::UserProfile;
    ///
    /// let cursor = CursorSpec {
    ///     cursor_position: Some((0, 0)),
    ///     selection: None,
    ///     cursors: None,
    ///     selections: None,
    /// };
    /// let scenario = Scenario {
    ///     id: "s1".to_string(),
    ///     name: "Test".to_string(),
    ///     description: "Test".to_string(),
    ///     setup: Setup { file_content: "test".to_string(), cursor: cursor.clone() },
    ///     target: TargetState { file_content: "test".to_string(), cursor },
    ///     solution: Solution { commands: vec!["x".to_string()], description: "Test".to_string() },
    ///     alternatives: vec![],
    ///     hints: vec![],
    ///     scoring: ScoringConfig {
    ///         optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
    ///         max_points: 100,
    ///         tolerance: 0,
    ///     },
    ///     metadata: Some(ScenarioMetadata {
    ///         category: Some(ScenarioCategory::Movement),
    ///         ..Default::default()
    ///     }),
    /// };
    ///
    /// let collection = ScenarioCollection::new(vec![scenario]);
    /// let mut profile = UserProfile::new();
    /// profile.scenario_history.record_completion("s1", 100, 50, Utc::now());
    ///
    /// let stats = collection.curriculum_stats(&profile);
    /// assert_eq!(stats.total, 1);
    /// assert_eq!(stats.perfected, 1);
    /// assert_eq!(stats.per_category, vec![(ScenarioCategory::Movement, 1, 1)]);
    /// ```
    pub fn curriculum_stats(&self, profile: &UserProfile) -> CurriculumStats {
        let mut stats = CurriculumStats {
            total: self.scenarios.len(),
            ..Default::default()
        };
        let mut per_category: Vec<(ScenarioCategory, usize, usize)> = Vec::new();

        for scenario in &self.scenarios {
            let completed = Self::is_scenario_completed(scenario, profile);
            let perfected = profile
                .scenario_history
                .get(&scenario.id)
                .is_some_and(|c| c.perfect_count > 0);

            if completed {
                stats.completed += 1;
            }
            if perfected {
                stats.perfected += 1;
            }

            if let Some(category) = scenario.metadata.as_ref().and_then(|m| m.category) {
                match per_category.iter_mut().find(|(c, _, _)| *c == category) {
                    Some((_, cat_perfected, cat_total)) => {
                        *cat_total += 1;
                        if perfected {
                            *cat_perfected += 1;
                        }
                    }
                    None => per_category.push((category, usize::from(perfected), 1)),
                }
            }
        }

        per_category.sort_by_key(|(c, _, _)| *c as u8);
        stats.per_category = per_category;
        stats
    }
}

/// Per-category and overall mastery counts over the unfiltered scenario set.
///
/// Returned by [`ScenarioCollection::curriculum_stats`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurriculumStats {
    /// Total scenarios in the collection, ignoring the active filter.
    pub total: usize,
    /// Scenarios with at least one recorded completion.
    pub completed: usize,
    /// Scenarios with at least one 100% completion (`perfect_count > 0`).
    pub perfected: usize,
    /// `(category, perfected_in_category, total_in_category)`, sorted the same
    /// way as [`ScenarioCollection::get_categories`] (`*category as u8`).
    pub per_category: Vec<(ScenarioCategory, usize, usize)>,
}

#[cfg(test)]
mod tests;
