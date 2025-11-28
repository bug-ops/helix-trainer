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

    /// Include only completed scenarios
    pub completed_only: bool,

    /// Include only not-yet-completed scenarios
    pub not_completed_only: bool,

    /// Include only scenarios with prerequisites met (None = no check)
    pub has_prerequisites: Option<bool>,

    /// Include only scenarios with specific tags
    pub tags: Option<HashSet<String>>,
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

    /// Category groups, then difficulty within each (recommended default)
    #[default]
    ByCategoryThenDifficulty,

    /// Weak (low mastery) → Strong (requires profile)
    ByMastery,
}

impl ScenarioCollection {
    /// Create a new collection from a vector of scenarios
    pub fn new(scenarios: Vec<Scenario>) -> Self {
        let count = scenarios.len();
        let filtered_indices: Vec<usize> = (0..count).collect();

        Self {
            scenarios,
            filtered_indices,
            active_filter: ScenarioFilter::default(),
            active_sort: SortMode::default(),
        }
    }

    /// Apply a filter to the collection
    ///
    /// This updates the internal filtered_indices to only include scenarios
    /// that match all specified criteria.
    pub fn apply_filter(&mut self, filter: &ScenarioFilter, profile: Option<&UserProfile>) {
        self.active_filter = filter.clone();

        self.filtered_indices = self.scenarios
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
                self.filtered_indices.sort_by(|&a, &b| {
                    self.scenarios[a].name.cmp(&self.scenarios[b].name)
                });
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
        let mut categories: Vec<ScenarioCategory> = self.scenarios
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
        let mut difficulties: Vec<Difficulty> = self.scenarios
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
            let scenario_category = scenario
                .metadata
                .as_ref()
                .and_then(|m| m.category);

            match scenario_category {
                Some(cat) if categories.contains(&cat) => {}, // Pass
                _ => return false, // Scenario has no category or wrong category
            }
        }

        // Check difficulty filter
        if let Some(ref difficulties) = filter.difficulties {
            let scenario_difficulty = scenario
                .metadata
                .as_ref()
                .and_then(|m| m.difficulty);

            match scenario_difficulty {
                Some(diff) if difficulties.contains(&diff) => {}, // Pass
                _ => return false, // Wrong difficulty
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

            if filter.completed_only && !is_completed {
                return false;
            }

            if filter.not_completed_only && is_completed {
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
}

#[cfg(test)]
mod tests;
