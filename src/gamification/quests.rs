//! Daily quest generation and tracking

use chrono::{Datelike, NaiveDate};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::config::quests::{
    QuestDifficulty as TemplateDifficulty, QuestLoader, QuestSpec, QuestTemplate,
};
use crate::ids::{NormalizedCommand, QuestId, ScenarioId};
use crate::security::UserError;

use super::{UserProfile, XPCalculator};

/// Quest difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestDifficulty {
    /// Short quests suited to new players.
    Easy,
    /// Mid-length quests.
    Medium,
    /// Long or demanding quests.
    Hard,
}

impl crate::learning::ProgressionTier for QuestDifficulty {
    fn name(&self) -> &'static str {
        match self {
            QuestDifficulty::Easy => "Easy",
            QuestDifficulty::Medium => "Medium",
            QuestDifficulty::Hard => "Hard",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            QuestDifficulty::Easy => "🟢",
            QuestDifficulty::Medium => "🟡",
            QuestDifficulty::Hard => "🔴",
        }
    }

    fn tier_level(&self) -> u32 {
        match self {
            QuestDifficulty::Easy => 0,
            QuestDifficulty::Medium => 1,
            QuestDifficulty::Hard => 2,
        }
    }

    fn is_max_tier(&self) -> bool {
        matches!(self, QuestDifficulty::Hard)
    }
}

/// Types of quests available
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestType {
    /// Practice a specific command N times
    CommandPractice {
        /// Command whose executions are counted.
        command: NormalizedCommand,
        /// Executions required.
        target: u32,
        /// Executions recorded so far.
        current: u32,
    },

    /// Complete N scenarios
    ScenarioCompletion {
        /// Completions required.
        target: u32,
        /// Completions recorded so far.
        current: u32,
    },

    /// Complete scenario within time limit
    SpeedRun {
        /// Scenario that must be completed.
        scenario_id: ScenarioId,
        /// Time limit for the run, persisted as whole seconds.
        #[serde(with = "duration_serde")]
        time_limit: Duration,
    },

    /// Practice for N minutes
    TimeInvested {
        /// Practice minutes required.
        target_minutes: u32,
        /// Practice minutes recorded so far.
        current_minutes: u32,
    },

    /// Use N different commands
    Exploration {
        /// Distinct commands required.
        target_commands: u32,
        /// Distinct commands seen so far.
        commands_used: HashSet<NormalizedCommand>,
    },
}

// Helper module for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

impl QuestType {
    /// Check if quest type is completed (convenience method)
    pub fn is_completed(&self) -> bool {
        use crate::learning::ProgressTracker;
        ProgressTracker::is_complete(self)
    }

    /// Get progress (0.0 - 1.0) (convenience method)
    pub fn progress(&self) -> f64 {
        use crate::learning::ProgressTracker;
        ProgressTracker::progress(self)
    }
}

impl crate::learning::ProgressTracker for QuestType {
    #[expect(
        clippy::cast_precision_loss,
        reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
    )]
    fn progress(&self) -> f64 {
        match self {
            QuestType::CommandPractice {
                target, current, ..
            } => (f64::from(*current) / f64::from((*target).max(1))).min(1.0),
            QuestType::ScenarioCompletion { target, current } => {
                (f64::from(*current) / f64::from((*target).max(1))).min(1.0)
            }
            QuestType::SpeedRun { .. } => 0.0,
            QuestType::TimeInvested {
                target_minutes,
                current_minutes,
            } => (f64::from(*current_minutes) / f64::from((*target_minutes).max(1))).min(1.0),
            QuestType::Exploration {
                target_commands,
                commands_used,
            } => (commands_used.len() as f64 / f64::from((*target_commands).max(1))).min(1.0),
        }
    }

    fn current(&self) -> u64 {
        match self {
            QuestType::CommandPractice { current, .. } => u64::from(*current),
            QuestType::ScenarioCompletion { current, .. } => u64::from(*current),
            QuestType::SpeedRun { .. } => 0,
            QuestType::TimeInvested {
                current_minutes, ..
            } => u64::from(*current_minutes),
            QuestType::Exploration { commands_used, .. } => commands_used.len() as u64,
        }
    }

    fn target(&self) -> u64 {
        match self {
            QuestType::CommandPractice { target, .. } => u64::from(*target),
            QuestType::ScenarioCompletion { target, .. } => u64::from(*target),
            QuestType::SpeedRun { .. } => 1,
            QuestType::TimeInvested { target_minutes, .. } => u64::from(*target_minutes),
            QuestType::Exploration {
                target_commands, ..
            } => u64::from(*target_commands),
        }
    }
}

/// A daily quest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quest {
    /// Unique quest identifier, stable across the day.
    pub id: QuestId,
    /// Quest type with its progress.
    pub quest_type: QuestType,
    /// Text shown to the player.
    pub description: String,
    /// Difficulty tier.
    pub difficulty: QuestDifficulty,
    /// XP granted on completion.
    pub xp_reward: u32,
    /// Whether the reward has been earned.
    pub completed: bool,
}

/// Instantiate a runtime quest from its template.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::quests::QuestLoader;
/// use helix_trainer::gamification::Quest;
///
/// let templates = QuestLoader::new().load_from_embedded("en")?;
/// let quest = Quest::from(&templates[0]);
/// assert_eq!(quest.id, templates[0].id);
/// assert!(!quest.completed);
/// # Ok::<(), helix_trainer::security::UserError>(())
/// ```
impl From<&QuestTemplate> for Quest {
    fn from(template: &QuestTemplate) -> Self {
        let quest_type = match &template.spec {
            QuestSpec::CommandPractice { command, target } => QuestType::CommandPractice {
                command: command.clone(),
                target: *target,
                current: 0,
            },
            QuestSpec::ScenarioCompletion { target } => QuestType::ScenarioCompletion {
                target: *target,
                current: 0,
            },
            QuestSpec::SpeedRun {
                scenario_id,
                time_limit_seconds,
            } => QuestType::SpeedRun {
                scenario_id: scenario_id.clone(),
                time_limit: Duration::from_secs(*time_limit_seconds),
            },
            QuestSpec::TimeInvested { target_minutes } => QuestType::TimeInvested {
                target_minutes: *target_minutes,
                current_minutes: 0,
            },
            QuestSpec::Exploration { target_commands } => QuestType::Exploration {
                target_commands: *target_commands,
                commands_used: HashSet::new(),
            },
        };
        let difficulty = match template.difficulty {
            TemplateDifficulty::Easy => QuestDifficulty::Easy,
            TemplateDifficulty::Medium => QuestDifficulty::Medium,
            TemplateDifficulty::Hard => QuestDifficulty::Hard,
        };
        Self::new(
            template.id.clone(),
            quest_type,
            template.description.clone(),
            difficulty,
        )
    }
}

impl Quest {
    /// Create a new quest
    pub fn new(
        id: QuestId,
        quest_type: QuestType,
        description: String,
        difficulty: QuestDifficulty,
    ) -> Self {
        let xp_reward = XPCalculator::quest_xp_reward(&quest_type, difficulty);
        Self {
            id,
            quest_type,
            description,
            difficulty,
            xp_reward,
            completed: false,
        }
    }

    /// Mark quest as completed
    pub fn mark_completed(&mut self) {
        self.completed = true;
    }

    /// Check if quest is completed (convenience method)
    pub fn is_completed(&self) -> bool {
        use crate::learning::ProgressTracker;
        ProgressTracker::is_complete(self)
    }

    /// Get progress (0.0 - 1.0) (convenience method)
    pub fn progress(&self) -> f64 {
        use crate::learning::ProgressTracker;
        ProgressTracker::progress(self)
    }
}

impl crate::learning::ProgressTracker for Quest {
    fn progress(&self) -> f64 {
        use crate::learning::ProgressTracker;
        if self.completed {
            1.0
        } else {
            ProgressTracker::progress(&self.quest_type)
        }
    }

    fn current(&self) -> u64 {
        use crate::learning::ProgressTracker;
        if self.completed {
            ProgressTracker::target(self)
        } else {
            ProgressTracker::current(&self.quest_type)
        }
    }

    fn target(&self) -> u64 {
        use crate::learning::ProgressTracker;
        ProgressTracker::target(&self.quest_type)
    }

    fn is_complete(&self) -> bool {
        use crate::learning::ProgressTracker;
        self.completed || ProgressTracker::is_complete(&self.quest_type)
    }
}

/// Quest distribution configuration by difficulty
struct QuestDistribution {
    easy: usize,
    medium: usize,
    hard: usize,
    exploration: usize,
}

impl QuestDistribution {
    /// Get quest distribution for a given level
    fn for_level(level: u32) -> Self {
        match level {
            1..=5 => Self {
                easy: 2,
                medium: 1,
                hard: 0,
                exploration: 0,
            },
            6..=15 => Self {
                easy: 1,
                medium: 2,
                hard: 1,
                exploration: 0,
            },
            _ => Self {
                easy: 0,
                medium: 1,
                hard: 2,
                exploration: 1,
            },
        }
    }

    /// Generate quests according to this distribution
    fn generate_quests(
        &self,
        rng: &mut StdRng,
        registry: &QuestTemplateRegistry,
    ) -> QuestGeneration {
        let mut quests = Vec::new();
        let mut unfilled_slots: usize = 0;
        let mut picked = HashSet::new();
        let mut fill = |quest: Option<Quest>| match quest {
            Some(quest) => quests.push(quest),
            None => unfilled_slots = unfilled_slots.saturating_add(1),
        };

        for _ in 0..self.easy {
            fill(QuestGenerator::generate_easy_quest(
                rng,
                registry,
                &mut picked,
            ));
        }
        for _ in 0..self.medium {
            fill(QuestGenerator::generate_medium_quest(
                rng,
                registry,
                &mut picked,
            ));
        }
        for _ in 0..self.hard {
            fill(QuestGenerator::generate_hard_quest(
                rng,
                registry,
                &mut picked,
            ));
        }
        for _ in 0..self.exploration {
            fill(QuestGenerator::generate_exploration_quest(
                rng,
                registry,
                &mut picked,
            ));
        }

        QuestGeneration {
            quests,
            unfilled_slots,
        }
    }
}

/// Result of [`QuestGenerator::generate_quests`]: the quests that could be
/// generated and how many of the level's quest slots had no matching template.
#[derive(Debug, Clone)]
#[must_use]
pub struct QuestGeneration {
    /// Quests generated from the registry's templates.
    pub quests: Vec<Quest>,

    /// Slots left empty because the registry had no template for them
    /// (an unavailable or empty registry leaves every slot unfilled).
    pub unfilled_slots: usize,
}

/// Registry for quest templates loaded from TOML files
///
/// Provides caching and filtering of quest templates loaded from configuration files.
/// Quest templates are defined in TOML files under `quests/{locale}/daily.toml`.
#[derive(Debug, Clone)]
pub struct QuestTemplateRegistry {
    templates: Vec<QuestTemplate>,
}

/// Load state of the shared [`QuestTemplateRegistry`] held by the application.
///
/// Makes "not loaded yet" and "failed to load" distinct states instead of
/// collapsing both into an empty registry, so quest generation can tell a missing
/// registry from one that legitimately has no templates.
#[derive(Debug, Clone, Default)]
pub enum QuestRegistrySlot {
    /// The background load has not reported back yet.
    #[default]
    Pending,
    /// Templates loaded successfully and are shared by quest generation.
    Ready(Arc<QuestTemplateRegistry>),
    /// Loading failed; quest generation must not assume any templates exist.
    Unavailable,
}

impl QuestRegistrySlot {
    /// The registry to generate quests from, or `None` while loading is pending.
    ///
    /// An unavailable registry resolves to an empty one, so quest generation
    /// fills no slot instead of assuming templates exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::QuestRegistrySlot;
    ///
    /// assert!(QuestRegistrySlot::Pending.resolved().is_none());
    /// assert!(QuestRegistrySlot::Unavailable.resolved().unwrap().is_empty());
    /// ```
    pub fn resolved(&self) -> Option<Arc<QuestTemplateRegistry>> {
        match self {
            Self::Pending => None,
            Self::Ready(registry) => Some(Arc::clone(registry)),
            Self::Unavailable => Some(Arc::new(QuestTemplateRegistry::new())),
        }
    }
}

impl QuestTemplateRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    /// Parse and validate the quest templates embedded for `locale`
    ///
    /// Reads compile-time embedded content, so there is no filesystem access.
    ///
    /// # Errors
    ///
    /// Returns `UserError` if no templates are embedded for `locale` or they
    /// fail validation.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::QuestTemplateRegistry;
    ///
    /// let registry = QuestTemplateRegistry::from_embedded("en").unwrap();
    /// assert!(!registry.is_empty());
    /// assert!(QuestTemplateRegistry::from_embedded("zz").is_err());
    /// ```
    pub fn from_embedded(locale: &str) -> Result<Self, UserError> {
        let loader = QuestLoader::new();
        let templates = loader.load_from_embedded(locale)?;

        tracing::info!(
            count = templates.len(),
            locale = locale,
            "Loaded quest templates from embedded data"
        );

        Ok(Self { templates })
    }

    /// Get all templates matching a specific difficulty
    pub fn get_by_difficulty(
        &self,
        difficulty: crate::config::quests::QuestDifficulty,
    ) -> Vec<&QuestTemplate> {
        self.templates
            .iter()
            .filter(|t| t.difficulty == difficulty)
            .collect()
    }

    /// Get a template by its ID
    pub fn get_by_id(&self, id: &QuestId) -> Option<&QuestTemplate> {
        self.templates.iter().find(|t| &t.id == id)
    }

    /// Get all eligible quests based on user conditions
    ///
    /// Filters quests by:
    /// - Level requirements (`min_level`, `max_level`)
    /// - Required commands (user must have used these)
    /// - Required scenarios (user must have completed these)
    pub fn get_eligible_quests(
        &self,
        user_level: u32,
        commands_used: &HashSet<NormalizedCommand>,
        scenarios_completed: &HashSet<ScenarioId>,
    ) -> Vec<&QuestTemplate> {
        self.templates
            .iter()
            .filter(|template| {
                // Check min_level
                if let Some(min) = template.conditions.min_level
                    && user_level < min
                {
                    return false;
                }

                // Check max_level
                if let Some(max) = template.conditions.max_level
                    && user_level > max
                {
                    return false;
                }

                // Check required commands
                if !template.conditions.requires_commands.is_empty() {
                    let has_all_commands = template
                        .conditions
                        .requires_commands
                        .iter()
                        .all(|cmd| commands_used.contains(cmd));
                    if !has_all_commands {
                        return false;
                    }
                }

                // Check required scenarios
                if !template.conditions.requires_scenarios.is_empty() {
                    let has_all_scenarios = template
                        .conditions
                        .requires_scenarios
                        .iter()
                        .all(|scenario| scenarios_completed.contains(scenario));
                    if !has_all_scenarios {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Get count of loaded templates
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

impl Default for QuestTemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates daily quests based on user level
pub struct QuestGenerator;

impl QuestGenerator {
    /// Generate daily quests for a user
    ///
    /// # Quest Mix by Level
    ///
    /// - Level 1-5: 2 easy + 1 medium
    /// - Level 6-15: 1 easy + 2 medium + 1 hard
    /// - Level 16+: 1 medium + 2 hard + 1 exploration
    ///
    /// A slot with no matching template in `registry` is left unfilled and
    /// counted in [`QuestGeneration::unfilled_slots`]; an empty registry
    /// therefore yields no quests rather than a panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use chrono::Utc;
    /// use helix_trainer::gamification::{QuestGenerator, QuestTemplateRegistry, UserProfile};
    ///
    /// let profile = UserProfile::new(); // Level 1
    /// let registry = QuestTemplateRegistry::from_embedded("en").unwrap();
    /// let generation = QuestGenerator::generate_quests(&profile, &registry, Utc::now().date_naive());
    /// assert_eq!(generation.quests.len(), 3); // Beginners get 3 quests
    /// assert_eq!(generation.unfilled_slots, 0);
    ///
    /// let empty = QuestGenerator::generate_quests(
    ///     &profile,
    ///     &QuestTemplateRegistry::new(),
    ///     Utc::now().date_naive(),
    /// );
    /// assert!(empty.quests.is_empty());
    /// assert_eq!(empty.unfilled_slots, 3);
    /// ```
    pub fn generate_quests(
        profile: &UserProfile,
        registry: &QuestTemplateRegistry,
        today: NaiveDate,
    ) -> QuestGeneration {
        let mut rng = Self::create_rng(today);
        QuestDistribution::for_level(profile.level).generate_quests(&mut rng, registry)
    }

    fn create_rng(today: NaiveDate) -> StdRng {
        // Seed with current date for consistency within a day
        let seed = u64::from(today.num_days_from_ce().cast_unsigned());
        StdRng::seed_from_u64(seed)
    }

    fn pick_quest(
        rng: &mut StdRng,
        templates: &[&QuestTemplate],
        kind: &str,
        picked: &mut HashSet<QuestId>,
    ) -> Option<Quest> {
        let candidates: Vec<&&QuestTemplate> = templates
            .iter()
            .filter(|template| !picked.contains(&template.id))
            .collect();
        if candidates.is_empty() {
            tracing::warn!(kind, "No unpicked quest templates available");
            return None;
        }
        let template = candidates.get(rng.random_range(0..candidates.len()))?;
        tracing::debug!(id = %template.id, kind, "Selected quest template");
        picked.insert(template.id.clone());
        Some(Quest::from(**template))
    }

    fn generate_easy_quest(
        rng: &mut StdRng,
        registry: &QuestTemplateRegistry,
        picked: &mut HashSet<QuestId>,
    ) -> Option<Quest> {
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Easy);
        Self::pick_quest(rng, &templates, "easy", picked)
    }

    fn generate_medium_quest(
        rng: &mut StdRng,
        registry: &QuestTemplateRegistry,
        picked: &mut HashSet<QuestId>,
    ) -> Option<Quest> {
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Medium);
        Self::pick_quest(rng, &templates, "medium", picked)
    }

    fn generate_hard_quest(
        rng: &mut StdRng,
        registry: &QuestTemplateRegistry,
        picked: &mut HashSet<QuestId>,
    ) -> Option<Quest> {
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Hard);
        Self::pick_quest(rng, &templates, "hard", picked)
    }

    fn generate_exploration_quest(
        rng: &mut StdRng,
        registry: &QuestTemplateRegistry,
        picked: &mut HashSet<QuestId>,
    ) -> Option<Quest> {
        // Exploration quests can be either Medium or Hard difficulty
        let mut templates =
            registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Hard);
        templates
            .extend(registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Medium));
        templates
            .retain(|t| matches!(t.spec, crate::config::quests::QuestSpec::Exploration { .. }));
        Self::pick_quest(rng, &templates, "exploration", picked)
    }
}

/// Tracks quest progress and updates
pub struct QuestTracker;

impl QuestTracker {
    /// Update quest progress based on command execution
    pub fn update_command_progress(quests: &mut [Quest], command: &NormalizedCommand) {
        for quest in quests.iter_mut() {
            if quest.completed {
                continue;
            }

            match &mut quest.quest_type {
                QuestType::CommandPractice {
                    command: cmd,
                    target,
                    current,
                } if cmd == command => {
                    *current = current.saturating_add(1).min(*target);
                    if *current >= *target {
                        quest.completed = true;
                    }
                }
                QuestType::Exploration {
                    target_commands,
                    commands_used,
                } => {
                    commands_used.insert(command.clone());
                    if commands_used.len() >= *target_commands as usize {
                        quest.completed = true;
                    }
                }
                _ => {}
            }
        }
    }

    /// Update quest progress for scenario completion
    pub fn update_scenario_progress(
        quests: &mut [Quest],
        scenario_id: &ScenarioId,
        duration: Duration,
    ) {
        for quest in quests.iter_mut() {
            if quest.completed {
                continue;
            }

            match &mut quest.quest_type {
                QuestType::ScenarioCompletion { target, current } => {
                    *current = current.saturating_add(1).min(*target);
                    if *current >= *target {
                        quest.completed = true;
                    }
                }
                QuestType::SpeedRun {
                    scenario_id: quest_scenario,
                    time_limit,
                } if quest_scenario == scenario_id && duration <= *time_limit => {
                    quest.completed = true;
                }
                _ => {}
            }
        }
    }

    /// Update quest progress for time invested
    pub fn update_time_progress(quests: &mut [Quest], minutes: u32) {
        for quest in quests.iter_mut() {
            if quest.completed {
                continue;
            }

            if let QuestType::TimeInvested {
                target_minutes,
                current_minutes,
            } = &mut quest.quest_type
            {
                *current_minutes = current_minutes.saturating_add(minutes).min(*target_minutes);
                if *current_minutes >= *target_minutes {
                    quest.completed = true;
                }
            }
        }
    }

    /// Check if any quests were newly completed and return their XP rewards
    pub fn check_completions(quests: &[Quest]) -> Vec<(QuestId, u32)> {
        quests
            .iter()
            .filter(|q| q.is_completed())
            .map(|q| (q.id.clone(), q.xp_reward))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::commands::{CMD_DELETE_SELECTION, CMD_YANK};
    use crate::testing::{ncmd, qid, sid};

    /// Helper to create a registry for tests
    fn test_registry() -> QuestTemplateRegistry {
        QuestTemplateRegistry::from_embedded("en")
            .expect("Failed to load quest templates for tests")
    }

    #[test]
    fn test_quest_type_completion() {
        let mut quest_type = QuestType::CommandPractice {
            command: ncmd(CMD_DELETE_SELECTION),
            target: 5,
            current: 3,
        };
        assert!(!quest_type.is_completed());

        quest_type = QuestType::CommandPractice {
            command: ncmd(CMD_DELETE_SELECTION),
            target: 5,
            current: 5,
        };
        assert!(quest_type.is_completed());
    }

    #[test]
    fn test_quest_type_progress() {
        let quest_type = QuestType::ScenarioCompletion {
            target: 10,
            current: 5,
        };
        assert_eq!(quest_type.progress(), 0.5);
    }

    #[test]
    fn test_quest_creation() {
        let quest_type = QuestType::CommandPractice {
            command: ncmd(CMD_DELETE_SELECTION),
            target: 5,
            current: 0,
        };
        let quest = Quest::new(
            qid("test_quest"),
            quest_type,
            "Test quest".to_string(),
            QuestDifficulty::Easy,
        );

        assert_eq!(quest.id.as_str(), "test_quest");
        assert_eq!(quest.xp_reward, 25); // Easy command practice
        assert!(!quest.completed);
    }

    #[test]
    fn test_quest_generator_level_based() {
        let registry = test_registry();

        let today = chrono::Utc::now().date_naive();

        // Level 1 - should get 3 quests (2 easy + 1 medium)
        let mut profile = UserProfile::new();
        profile.level = 1;
        let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
        assert_eq!(quests.len(), 3);

        // Level 10 - should get 4 quests (1 easy + 2 medium + 1 hard)
        profile.level = 10;
        let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
        assert_eq!(quests.len(), 4);

        // Level 20 - should get 4 quests (1 medium + 2 hard + 1 exploration)
        profile.level = 20;
        let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
        assert_eq!(quests.len(), 4);
    }

    #[test]
    fn generated_quests_are_distinct_templates() {
        let registry = test_registry();
        let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        let days = [day(2026, 11, 11), day(2026, 1, 1), day(2025, 6, 30)];
        for level in [6, 10, 15, 16, 22, 40] {
            for today in days {
                let mut profile = UserProfile::new();
                profile.level = level;
                let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
                let ids: HashSet<_> = quests.iter().map(|q| q.id.clone()).collect();
                assert_eq!(ids.len(), quests.len(), "level {level} on {today}");
            }
        }
    }

    #[test]
    fn slot_without_unpicked_templates_stays_unfilled() {
        let registry = test_registry();
        let mut picked = HashSet::new();
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Easy);
        let mut rng = QuestGenerator::create_rng(NaiveDate::from_ymd_opt(2026, 11, 11).unwrap());
        for _ in &templates {
            assert!(
                QuestGenerator::pick_quest(&mut rng, &templates, "easy", &mut picked).is_some()
            );
        }
        assert!(QuestGenerator::pick_quest(&mut rng, &templates, "easy", &mut picked).is_none());
    }

    #[test]
    fn test_generate_quests_with_empty_registry_leaves_every_slot_unfilled() {
        let today = chrono::Utc::now().date_naive();
        for (level, slots) in [(1, 3), (10, 4), (20, 4)] {
            let mut profile = UserProfile::new();
            profile.level = level;

            let generation =
                QuestGenerator::generate_quests(&profile, &QuestTemplateRegistry::new(), today);

            assert!(generation.quests.is_empty());
            assert_eq!(generation.unfilled_slots, slots, "level {level}");
        }
    }

    #[test]
    fn test_generate_quests_with_a_difficulty_missing_fills_only_the_rest() {
        let today = chrono::Utc::now().date_naive();
        let mut profile = UserProfile::new();
        profile.level = 10;
        let full = test_registry();
        let mut partial = QuestTemplateRegistry::new();
        partial.templates = full
            .templates
            .iter()
            .filter(|t| t.difficulty != crate::config::quests::QuestDifficulty::Hard)
            .cloned()
            .collect();

        let generation = QuestGenerator::generate_quests(&profile, &partial, today);

        assert_eq!(generation.quests.len(), 3);
        assert_eq!(generation.unfilled_slots, 1);
    }

    #[test]
    fn test_quest_tracker_command_progress() {
        let mut quests = vec![Quest::new(
            qid("test"),
            QuestType::CommandPractice {
                command: ncmd(CMD_DELETE_SELECTION),
                target: 3,
                current: 0,
            },
            "Test".to_string(),
            QuestDifficulty::Easy,
        )];

        QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_DELETE_SELECTION));
        assert!(!quests[0].is_completed());

        QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_DELETE_SELECTION));
        QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_DELETE_SELECTION));
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_scenario_progress() {
        let mut quests = vec![Quest::new(
            qid("test"),
            QuestType::ScenarioCompletion {
                target: 2,
                current: 0,
            },
            "Test".to_string(),
            QuestDifficulty::Medium,
        )];

        QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("any_scenario"),
            Duration::from_secs(1),
        );
        assert!(!quests[0].is_completed());

        QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("any_scenario"),
            Duration::from_secs(1),
        );
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_speed_run() {
        let mut quests = vec![Quest::new(
            qid("test"),
            QuestType::SpeedRun {
                scenario_id: sid("delete_line_001"),
                time_limit: Duration::from_secs(5),
            },
            "Test".to_string(),
            QuestDifficulty::Hard,
        )];

        // Too slow
        QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("delete_line_001"),
            Duration::from_secs(10),
        );
        assert!(!quests[0].is_completed());

        // Fast enough
        QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("delete_line_001"),
            Duration::from_secs(3),
        );
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_exploration() {
        let mut quests = vec![Quest::new(
            qid("test"),
            QuestType::Exploration {
                target_commands: 3,
                commands_used: HashSet::new(),
            },
            "Test".to_string(),
            QuestDifficulty::Hard,
        )];

        QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_DELETE_SELECTION));
        QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_YANK));
        assert!(!quests[0].is_completed());

        QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(crate::grammar::commands::CMD_PASTE_AFTER),
        );
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_time_invested() {
        let mut quests = vec![Quest::new(
            qid("test"),
            QuestType::TimeInvested {
                target_minutes: 10,
                current_minutes: 0,
            },
            "Test".to_string(),
            QuestDifficulty::Medium,
        )];

        QuestTracker::update_time_progress(&mut quests, 5);
        assert!(!quests[0].is_completed());

        QuestTracker::update_time_progress(&mut quests, 5);
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_check_completions() {
        let quests = vec![
            Quest {
                id: qid("completed"),
                quest_type: QuestType::CommandPractice {
                    command: ncmd(CMD_DELETE_SELECTION),
                    target: 1,
                    current: 1,
                },
                description: "Test".to_string(),
                difficulty: QuestDifficulty::Easy,
                xp_reward: 25,
                completed: true,
            },
            Quest {
                id: qid("incomplete"),
                quest_type: QuestType::CommandPractice {
                    command: ncmd(CMD_YANK),
                    target: 5,
                    current: 2,
                },
                description: "Test".to_string(),
                difficulty: QuestDifficulty::Easy,
                xp_reward: 25,
                completed: false,
            },
        ];

        let completions = QuestTracker::check_completions(&quests);
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].0.as_str(), "completed");
        assert_eq!(completions[0].1, 25);
    }

    #[test]
    fn test_quest_difficulty_progression_tier() {
        use crate::learning::ProgressionTier;

        // Test Easy
        assert_eq!(QuestDifficulty::Easy.name(), "Easy");
        assert_eq!(QuestDifficulty::Easy.emoji(), "🟢");
        assert_eq!(QuestDifficulty::Easy.tier_level(), 0);
        assert!(!QuestDifficulty::Easy.is_max_tier());

        // Test Medium
        assert_eq!(QuestDifficulty::Medium.name(), "Medium");
        assert_eq!(QuestDifficulty::Medium.emoji(), "🟡");
        assert_eq!(QuestDifficulty::Medium.tier_level(), 1);
        assert!(!QuestDifficulty::Medium.is_max_tier());

        // Test Hard
        assert_eq!(QuestDifficulty::Hard.name(), "Hard");
        assert_eq!(QuestDifficulty::Hard.emoji(), "🔴");
        assert_eq!(QuestDifficulty::Hard.tier_level(), 2);
        assert!(QuestDifficulty::Hard.is_max_tier());
    }

    #[test]
    fn test_quest_template_registry_empty() {
        let registry = QuestTemplateRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_quest_template_registry_default() {
        let registry = QuestTemplateRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_get_eligible_quests_level_filter() {
        let registry = test_registry();
        let commands_used = HashSet::new();
        let scenarios_completed = HashSet::new();

        // Low level user should get some quests
        let eligible = registry.get_eligible_quests(1, &commands_used, &scenarios_completed);
        let low_level_count = eligible.len();

        // Higher level user should get different quests
        let eligible = registry.get_eligible_quests(15, &commands_used, &scenarios_completed);
        let high_level_count = eligible.len();

        // Both should get some quests (counts may differ)
        assert!(low_level_count > 0 || high_level_count > 0);
    }
}
