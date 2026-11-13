//! Scenario collection with filtering and sorting capabilities
//!
//! This module provides a wrapper around a Vec of scenarios with support for:
//! - Filtering by category, difficulty, commands, completion status
//! - Sorting by name, difficulty, category, progress, mastery
//! - Efficient index mapping for menu display

use crate::config::{Difficulty, Scenario, ScenarioCategory, ScenarioMetadata, ScenarioTag};
use crate::gamification::UserProfile;
use crate::ids::NormalizedCommand;
use std::collections::HashSet;
use std::hash::Hash;

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

/// A non-empty set of accepted values.
///
/// Emptiness is unrepresentable, so "accept one of these" can never mean "accept
/// nothing".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptySet<T: Eq + Hash>(HashSet<T>);

impl<T: Eq + Hash> NonEmptySet<T> {
    /// The set of `first` and `rest`.
    #[must_use]
    pub fn new(first: T, rest: impl IntoIterator<Item = T>) -> Self {
        let mut set: HashSet<T> = rest.into_iter().collect();
        set.insert(first);
        Self(set)
    }

    /// Whether `item` is in the set.
    #[must_use]
    pub fn contains(&self, item: &T) -> bool {
        self.0.contains(item)
    }

    /// The set with `item` added.
    #[must_use]
    pub fn with(mut self, item: T) -> Self {
        self.0.insert(item);
        self
    }

    /// The set without `item`, or `None` when that leaves it empty.
    #[must_use]
    pub fn without(mut self, item: &T) -> Option<Self> {
        self.0.remove(item);
        (!self.0.is_empty()).then_some(self)
    }
}

impl<T: Eq + Hash> TryFrom<HashSet<T>> for NonEmptySet<T> {
    type Error = EmptyCriterion;

    fn try_from(set: HashSet<T>) -> Result<Self, EmptyCriterion> {
        if set.is_empty() {
            Err(EmptyCriterion)
        } else {
            Ok(Self(set))
        }
    }
}

/// An empty set was given where at least one accepted value is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a filter criterion needs at least one accepted value")]
pub struct EmptyCriterion;

/// What a [`ScenarioFilter`] requires of one scenario property.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::Difficulty;
/// use helix_trainer::game::Criterion;
///
/// let beginners = Criterion::one_of(Difficulty::Beginner, []);
/// assert!(beginners.admits(Some(&Difficulty::Beginner)));
/// assert!(!beginners.admits(None));
/// assert!(Criterion::<Difficulty>::Any.admits(None));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Criterion<T: Eq + Hash> {
    /// No restriction.
    #[default]
    Any,
    /// The property must be one of these values.
    OneOf(NonEmptySet<T>),
}

impl<T: Eq + Hash> Criterion<T> {
    /// A criterion accepting `first` and `rest`.
    #[must_use]
    pub fn one_of(first: T, rest: impl IntoIterator<Item = T>) -> Self {
        Self::OneOf(NonEmptySet::new(first, rest))
    }

    /// Whether the criterion places no restriction.
    #[must_use]
    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }

    /// Whether the criterion restricts the property to some values.
    #[must_use]
    pub fn is_restricted(&self) -> bool {
        !self.is_any()
    }

    /// Whether a property value passes; a missing value fails any restriction.
    #[must_use]
    pub fn admits(&self, value: Option<&T>) -> bool {
        match self {
            Self::Any => true,
            Self::OneOf(accepted) => value.is_some_and(|value| accepted.contains(value)),
        }
    }

    /// Whether at least one of `values` passes.
    #[must_use]
    pub fn admits_any<'a>(&self, values: impl IntoIterator<Item = &'a T>) -> bool
    where
        T: 'a,
    {
        match self {
            Self::Any => true,
            Self::OneOf(accepted) => values.into_iter().any(|value| accepted.contains(value)),
        }
    }
}

/// Prerequisite criterion for [`ScenarioFilter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrerequisiteFilter {
    /// No restriction.
    #[default]
    Any,
    /// Keep only scenarios that list prerequisites.
    WithPrerequisites,
    /// Keep only scenarios that list none.
    WithoutPrerequisites,
}

/// Filter criteria for scenarios
#[derive(Debug, Clone, Default)]
pub struct ScenarioFilter {
    /// Include only scenarios in these categories
    pub categories: Criterion<ScenarioCategory>,

    /// Include only scenarios at these difficulty levels
    pub difficulties: Criterion<Difficulty>,

    /// Include only scenarios teaching at least one of these commands
    pub commands: Criterion<NormalizedCommand>,

    /// Completion-status criterion (defaults to [`CompletionFilter::Any`])
    pub completion: CompletionFilter,

    /// Prerequisite criterion (defaults to [`PrerequisiteFilter::Any`])
    pub prerequisites: PrerequisiteFilter,

    /// Include only scenarios with at least one of these tags
    pub tags: Criterion<ScenarioTag>,
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
    /// Applies the default sort mode (`ByDifficultyThenCategory`) on creation.
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
    /// This updates the internal `filtered_indices` to only include scenarios
    /// that match all specified criteria.
    pub fn apply_filter(&mut self, filter: &ScenarioFilter, profile: Option<&UserProfile>) {
        self.active_filter = filter.clone();

        self.filtered_indices = self
            .scenarios
            .iter()
            .enumerate()
            .filter(|(_, scenario)| Self::matches_filter(scenario, filter, profile))
            .map(|(idx, _)| idx)
            .collect();
    }

    /// Sort the filtered scenarios according to the specified mode
    pub fn sort(&mut self, mode: SortMode, profile: Option<&UserProfile>) {
        self.active_sort = mode;

        match mode {
            SortMode::ByName => {
                self.filtered_indices.sort_by(|&a, &b| {
                    self.scenarios
                        .get(a)
                        .map(|s| &s.name)
                        .cmp(&self.scenarios.get(b).map(|s| &s.name))
                });
            }

            SortMode::ByDifficulty => {
                self.filtered_indices.sort_by_key(|&idx| {
                    Self::metadata_of(&self.scenarios, idx)
                        .and_then(|m| m.difficulty)
                        .unwrap_or(Difficulty::Beginner) // Default to beginner if no metadata
                });
            }

            SortMode::ByCategory => {
                self.filtered_indices.sort_by_key(|&idx| {
                    // Sort by category as integers (based on enum order)
                    Self::metadata_of(&self.scenarios, idx)
                        .and_then(|m| m.category)
                        .map_or(255, |c| c as u8) // Put scenarios without category at the end
                });
            }

            SortMode::ByProgress => {
                if let Some(prof) = profile {
                    self.filtered_indices.sort_by_cached_key(|&idx| {
                        // Completed scenarios go to the end
                        self.scenarios
                            .get(idx)
                            .is_some_and(|s| prof.scenario_history().get(&s.id).is_some())
                    });
                }
                // If no profile, don't change order
            }

            SortMode::ByCategoryThenDifficulty => {
                self.filtered_indices.sort_by_key(|&idx| {
                    let metadata = Self::metadata_of(&self.scenarios, idx);
                    let category = metadata.and_then(|m| m.category).map_or(255, |c| c as u8);
                    let difficulty = metadata
                        .and_then(|m| m.difficulty)
                        .unwrap_or(Difficulty::Beginner);
                    (category, difficulty)
                });
            }

            SortMode::ByDifficultyThenCategory => {
                self.filtered_indices.sort_by_key(|&idx| {
                    let metadata = Self::metadata_of(&self.scenarios, idx);
                    let difficulty = metadata
                        .and_then(|m| m.difficulty)
                        .unwrap_or(Difficulty::Beginner);
                    let category = metadata.and_then(|m| m.category).map_or(255, |c| c as u8);
                    (difficulty, category)
                });
            }

            SortMode::ByMastery => {
                if let Some(prof) = profile {
                    self.filtered_indices.sort_by_cached_key(|&idx| {
                        // Sort by completion count (fewer completions = lower mastery = higher priority)
                        self.scenarios
                            .get(idx)
                            .and_then(|s| prof.scenario_history().get(&s.id))
                            .map_or(0, |completion| completion.attempts) // Never attempted = highest priority
                    });
                }
                // If no profile, don't change order
            }
        }
    }

    fn metadata_of(scenarios: &[Scenario], idx: usize) -> Option<&ScenarioMetadata> {
        scenarios.get(idx).map(|s| &s.metadata)
    }

    /// Filtered scenarios (references)
    pub fn filtered(&self) -> Vec<&Scenario> {
        self.filtered_indices
            .iter()
            .filter_map(|&idx| self.scenarios.get(idx))
            .collect()
    }

    /// Get a specific filtered scenario by its index in the filtered list
    pub fn get_filtered_by_index(&self, filtered_idx: usize) -> Option<&Scenario> {
        self.filtered_indices
            .get(filtered_idx)
            .and_then(|&original_idx| self.scenarios.get(original_idx))
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

    /// All unique categories present in the collection
    pub fn categories(&self) -> Vec<ScenarioCategory> {
        let mut categories: Vec<ScenarioCategory> = self
            .scenarios
            .iter()
            .filter_map(|s| s.metadata.category)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        categories.sort_by_key(|c| *c as u8);
        categories
    }

    /// All unique difficulties present in the collection
    pub fn difficulties(&self) -> Vec<Difficulty> {
        let mut difficulties: Vec<Difficulty> = self
            .scenarios
            .iter()
            .filter_map(|s| s.metadata.difficulty)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        difficulties.sort();
        difficulties
    }

    /// Check if a scenario matches the current filter
    fn matches_filter(
        scenario: &Scenario,
        filter: &ScenarioFilter,
        profile: Option<&UserProfile>,
    ) -> bool {
        if !filter
            .categories
            .admits(scenario.metadata.category.as_ref())
            || !filter
                .difficulties
                .admits(scenario.metadata.difficulty.as_ref())
            || !filter
                .commands
                .admits_any(&scenario.metadata.commands_taught)
            || !filter.tags.admits_any(&scenario.metadata.tags)
        {
            return false;
        }

        // Check completion status
        if let Some(prof) = profile {
            let is_completed = prof.scenario_history().get(&scenario.id).is_some();

            let keep = match filter.completion {
                CompletionFilter::Any => true,
                CompletionFilter::CompletedOnly => is_completed,
                CompletionFilter::NotCompletedOnly => !is_completed,
            };
            if !keep {
                return false;
            }
        }

        let has_prerequisites = !scenario.metadata.prerequisites.is_empty();
        match filter.prerequisites {
            PrerequisiteFilter::Any => true,
            PrerequisiteFilter::WithPrerequisites => has_prerequisites,
            PrerequisiteFilter::WithoutPrerequisites => !has_prerequisites,
        }
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
        profile.scenario_history().get(&scenario.id).is_some()
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
    ///     CursorSpec, Pos, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution,
    ///     TargetState,
    /// };
    /// use helix_trainer::game::ScenarioCollection;
    /// use helix_trainer::gamification::UserProfile;
    /// use helix_trainer::ids::ScenarioId;
    ///
    /// fn minimal_scenario(id: &str) -> Scenario {
    ///     let cursor = CursorSpec::Point(Pos::try_from([0, 0]).unwrap());
    ///     Scenario {
    ///         id: ScenarioId::parse(id).unwrap(),
    ///         name: "Test".to_string(),
    ///         description: "Test".to_string(),
    ///         setup: Setup { file_content: "test".to_string(), language: None, cursor: cursor.clone() },
    ///         target: TargetState { file_content: "test".to_string(), cursor },
    ///         solution: Solution { commands: vec!["x".to_string()], description: "Test".to_string() },
    ///         hints: vec![],
    ///         scoring: ScoringConfig {
    ///             optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
    ///             max_points: 100,
    ///             tolerance: 0,
    ///         },
    ///         metadata: ScenarioMetadata::default(),
    ///     }
    /// }
    ///
    /// let collection = ScenarioCollection::new(vec![minimal_scenario("s1")]);
    /// let mut profile = UserProfile::new();
    /// assert_eq!(collection.completed_count(&profile), 0);
    ///
    /// profile.scenario_history_mut().record_completion(&ScenarioId::parse("s1").unwrap(), 100, 50, helix_trainer::time::Moment::local(Utc::now()));
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
    ///     CursorSpec, Pos, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution,
    ///     TargetState,
    /// };
    /// use helix_trainer::game::ScenarioCollection;
    /// use helix_trainer::gamification::UserProfile;
    /// use helix_trainer::ids::ScenarioId;
    ///
    /// fn minimal_scenario(id: &str) -> Scenario {
    ///     let cursor = CursorSpec::Point(Pos::try_from([0, 0]).unwrap());
    ///     Scenario {
    ///         id: ScenarioId::parse(id).unwrap(),
    ///         name: "Test".to_string(),
    ///         description: "Test".to_string(),
    ///         setup: Setup { file_content: "test".to_string(), language: None, cursor: cursor.clone() },
    ///         target: TargetState { file_content: "test".to_string(), cursor },
    ///         solution: Solution { commands: vec!["x".to_string()], description: "Test".to_string() },
    ///         hints: vec![],
    ///         scoring: ScoringConfig {
    ///             optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
    ///             max_points: 100,
    ///             tolerance: 0,
    ///         },
    ///         metadata: ScenarioMetadata::default(),
    ///     }
    /// }
    ///
    /// let collection = ScenarioCollection::new(vec![minimal_scenario("s1")]);
    /// let mut profile = UserProfile::new();
    /// assert!(!collection.is_curriculum_complete(&profile));
    ///
    /// profile.scenario_history_mut().record_completion(&ScenarioId::parse("s1").unwrap(), 100, 50, helix_trainer::time::Moment::local(Utc::now()));
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
    ///     CursorSpec, Pos, Scenario, ScenarioCategory, ScenarioMetadata,
    ///     ScoringConfig, Setup, Solution, TargetState,
    /// };
    /// use helix_trainer::game::ScenarioCollection;
    /// use helix_trainer::gamification::UserProfile;
    /// use helix_trainer::ids::ScenarioId;
    ///
    /// let cursor = CursorSpec::Point(Pos::try_from([0, 0]).unwrap());
    /// let scenario = Scenario {
    ///     id: ScenarioId::parse("s1").unwrap(),
    ///     name: "Test".to_string(),
    ///     description: "Test".to_string(),
    ///     setup: Setup { file_content: "test".to_string(), language: None, cursor: cursor.clone() },
    ///     target: TargetState { file_content: "test".to_string(), cursor },
    ///     solution: Solution { commands: vec!["x".to_string()], description: "Test".to_string() },
    ///     hints: vec![],
    ///     scoring: ScoringConfig {
    ///         optimal_count: std::num::NonZeroUsize::new(1).unwrap(),
    ///         max_points: 100,
    ///         tolerance: 0,
    ///     },
    ///     metadata: ScenarioMetadata {
    ///         category: Some(ScenarioCategory::Movement),
    ///         ..Default::default()
    ///     },
    /// };
    ///
    /// let collection = ScenarioCollection::new(vec![scenario]);
    /// let mut profile = UserProfile::new();
    /// profile.scenario_history_mut().record_completion(&ScenarioId::parse("s1").unwrap(), 100, 50, helix_trainer::time::Moment::local(Utc::now()));
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
                .scenario_history()
                .get(&scenario.id)
                .is_some_and(|c| c.perfect_count > 0);

            if completed {
                stats.completed = stats.completed.saturating_add(1);
            }
            if perfected {
                stats.perfected = stats.perfected.saturating_add(1);
            }

            if let Some(category) = scenario.metadata.category {
                match per_category.iter_mut().find(|(c, _, _)| *c == category) {
                    Some((_, cat_perfected, cat_total)) => {
                        *cat_total = cat_total.saturating_add(1);
                        if perfected {
                            *cat_perfected = cat_perfected.saturating_add(1);
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
    /// way as [`ScenarioCollection::categories`] (`*category as u8`).
    pub per_category: Vec<(ScenarioCategory, usize, usize)>,
}

#[cfg(test)]
mod tests;
