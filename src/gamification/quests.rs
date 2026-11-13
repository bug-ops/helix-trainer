//! Daily quest generation and tracking

use chrono::Datelike;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use crate::config::ConfigError;
use crate::config::quests::{
    QuestDifficulty as TemplateDifficulty, QuestLoader, QuestSpec, QuestTemplate,
};
use crate::ids::{NormalizedCommand, QuestId, ScenarioId};
use crate::time::PlayerDay;

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

    pub(crate) fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
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

    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
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

    #[expect(
        clippy::match_same_arms,
        reason = "arms stay apart to document each case"
    )]
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

/// Where a quest stands: the one place its completion is stored.
///
/// # Examples
///
/// ```
/// use helix_trainer::gamification::QuestStatus;
///
/// assert!(!QuestStatus::InProgress.is_completed());
/// assert!(QuestStatus::Completed { rewarded: false }.is_completed());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    /// The target has not been reached.
    InProgress,
    /// The target was reached.
    Completed {
        /// Whether the completion XP was granted.
        rewarded: bool,
    },
}

impl QuestStatus {
    /// Whether the target was reached, rewarded or not.
    #[must_use]
    pub fn is_completed(self) -> bool {
        matches!(self, Self::Completed { .. })
    }
}

/// A daily quest
///
/// Reads the legacy `completed` flag of older profile files: `true` loads as
/// `Completed { rewarded: false }`; the profile then settles `rewarded` from its
/// legacy completed-quest set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "QuestWire")]
pub struct Quest {
    /// Unique quest identifier, stable across the day.
    pub id: QuestId,
    /// Quest type with its progress.
    pub quest_type: QuestType,
    /// Difficulty tier.
    pub difficulty: QuestDifficulty,
    /// XP granted on completion.
    pub xp_reward: u32,
    status: QuestStatus,
}

/// The persisted shape of a [`Quest`], including the pre-status `completed` flag.
#[derive(Deserialize)]
struct QuestWire {
    id: QuestId,
    quest_type: QuestType,
    difficulty: QuestDifficulty,
    xp_reward: u32,
    #[serde(default)]
    status: Option<QuestStatus>,
    #[serde(default)]
    completed: bool,
}

impl From<QuestWire> for Quest {
    fn from(wire: QuestWire) -> Self {
        let status = wire.status.unwrap_or(if wire.completed {
            QuestStatus::Completed { rewarded: false }
        } else {
            QuestStatus::InProgress
        });
        Self {
            id: wire.id,
            quest_type: wire.quest_type,
            difficulty: wire.difficulty,
            xp_reward: wire.xp_reward,
            status,
        }
    }
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
/// assert!(!quest.status().is_completed());
/// # Ok::<(), helix_trainer::error::UserError>(())
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
        Self::new(template.id.clone(), quest_type, difficulty)
    }
}

impl Quest {
    /// Create a new quest
    pub fn new(id: QuestId, quest_type: QuestType, difficulty: QuestDifficulty) -> Self {
        let xp_reward = XPCalculator::quest_xp_reward(&quest_type, difficulty);
        Self {
            id,
            quest_type,
            difficulty,
            xp_reward,
            status: QuestStatus::InProgress,
        }
    }

    /// Where the quest stands.
    #[must_use]
    pub fn status(&self) -> QuestStatus {
        self.status
    }

    /// Mark the quest completed, keeping an earlier reward mark.
    pub fn mark_completed(&mut self) {
        if !self.status.is_completed() {
            self.status = QuestStatus::Completed { rewarded: false };
        }
    }

    /// Record that the completion XP was granted.
    pub fn mark_rewarded(&mut self) {
        self.status = QuestStatus::Completed { rewarded: true };
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
        if self.status.is_completed() {
            1.0
        } else {
            ProgressTracker::progress(&self.quest_type)
        }
    }

    fn current(&self) -> u64 {
        use crate::learning::ProgressTracker;
        if self.status.is_completed() {
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
        self.status.is_completed() || ProgressTracker::is_complete(&self.quest_type)
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
    /// Returns [`ConfigError`] if no templates are embedded for `locale` or they
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
    pub fn from_embedded(locale: &str) -> Result<Self, ConfigError> {
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
    /// use helix_trainer::time::Moment;
    ///
    /// let profile = UserProfile::new(); // Level 1
    /// let registry = QuestTemplateRegistry::from_embedded("en").unwrap();
    /// let generation = QuestGenerator::generate_quests(&profile, &registry, Moment::from(Utc::now()).day());
    /// assert_eq!(generation.quests.len(), 3); // Beginners get 3 quests
    /// assert_eq!(generation.unfilled_slots, 0);
    ///
    /// let empty = QuestGenerator::generate_quests(
    ///     &profile,
    ///     &QuestTemplateRegistry::new(),
    ///     Moment::from(Utc::now()).day(),
    /// );
    /// assert!(empty.quests.is_empty());
    /// assert_eq!(empty.unfilled_slots, 3);
    /// ```
    pub fn generate_quests(
        profile: &UserProfile,
        registry: &QuestTemplateRegistry,
        today: PlayerDay,
    ) -> QuestGeneration {
        let mut rng = Self::create_rng(today);
        QuestDistribution::for_level(profile.level()).generate_quests(&mut rng, registry)
    }

    fn create_rng(today: PlayerDay) -> StdRng {
        // Seed with current date for consistency within a day
        let seed = u64::from(today.date().num_days_from_ce().cast_unsigned());
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
///
/// Every update returns the ids of the quests it newly completed, so the caller
/// awards exactly those; a quest already completed is never returned again.
pub struct QuestTracker;

impl QuestTracker {
    /// Update quest progress based on command execution
    #[must_use]
    pub fn update_command_progress(
        quests: &mut [Quest],
        command: &NormalizedCommand,
    ) -> Vec<QuestId> {
        Self::advance(quests, |quest_type| match quest_type {
            QuestType::CommandPractice {
                command: cmd,
                target,
                current,
            } if cmd == command => {
                *current = current.saturating_add(1).min(*target);
                *current >= *target
            }
            QuestType::Exploration {
                target_commands,
                commands_used,
            } => {
                commands_used.insert(command.clone());
                commands_used.len() >= *target_commands as usize
            }
            _ => false,
        })
    }

    /// Update quest progress for scenario completion
    #[must_use]
    pub fn update_scenario_progress(
        quests: &mut [Quest],
        scenario_id: &ScenarioId,
        duration: Duration,
    ) -> Vec<QuestId> {
        Self::advance(quests, |quest_type| match quest_type {
            QuestType::ScenarioCompletion { target, current } => {
                *current = current.saturating_add(1).min(*target);
                *current >= *target
            }
            QuestType::SpeedRun {
                scenario_id: quest_scenario,
                time_limit,
            } => quest_scenario == scenario_id && duration <= *time_limit,
            _ => false,
        })
    }

    /// Update quest progress for time invested
    #[must_use]
    pub fn update_time_progress(quests: &mut [Quest], minutes: u32) -> Vec<QuestId> {
        Self::advance(quests, |quest_type| match quest_type {
            QuestType::TimeInvested {
                target_minutes,
                current_minutes,
            } => {
                *current_minutes = current_minutes.saturating_add(minutes).min(*target_minutes);
                *current_minutes >= *target_minutes
            }
            _ => false,
        })
    }

    /// Apply `advance` to every quest still in progress and complete those it
    /// reports as having reached their target.
    fn advance(
        quests: &mut [Quest],
        mut advance: impl FnMut(&mut QuestType) -> bool,
    ) -> Vec<QuestId> {
        let mut newly_completed = Vec::new();
        for quest in quests
            .iter_mut()
            .filter(|quest| !quest.status.is_completed())
        {
            if advance(&mut quest.quest_type) {
                quest.mark_completed();
                newly_completed.push(quest.id.clone());
            }
        }
        newly_completed
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
        let quest = Quest::new(qid("test_quest"), quest_type, QuestDifficulty::Easy);

        assert_eq!(quest.id.as_str(), "test_quest");
        assert_eq!(quest.xp_reward, 25); // Easy command practice
        assert_eq!(quest.status(), QuestStatus::InProgress);
    }

    #[test]
    fn tracker_returns_each_newly_completed_quest_once() {
        let quest = |id: &str| {
            Quest::new(
                qid(id),
                QuestType::CommandPractice {
                    command: ncmd(CMD_YANK),
                    target: 1,
                    current: 0,
                },
                QuestDifficulty::Easy,
            )
        };
        let mut quests = vec![quest("a"), quest("b")];

        let first = QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_YANK));
        let again = QuestTracker::update_command_progress(&mut quests, &ncmd(CMD_YANK));

        assert_eq!(first, vec![qid("a"), qid("b")]);
        assert_eq!(again, Vec::<QuestId>::new());
        assert!(quests.iter().all(|quest| quest.status().is_completed()));
    }

    #[test]
    fn legacy_completed_flag_reads_as_an_unrewarded_completion() {
        let mut json = serde_json::to_value(Quest::new(
            qid("legacy"),
            QuestType::ScenarioCompletion {
                target: 1,
                current: 1,
            },
            QuestDifficulty::Easy,
        ))
        .unwrap();
        json.as_object_mut().unwrap().remove("status");
        json["completed"] = serde_json::json!(true);

        let quest: Quest = serde_json::from_value(json).unwrap();

        assert_eq!(quest.status(), QuestStatus::Completed { rewarded: false });
    }

    #[test]
    fn test_quest_generator_level_based() {
        let registry = test_registry();

        let today = crate::testing::today();

        // Level 1 - should get 3 quests (2 easy + 1 medium)
        let mut profile = UserProfile::new();
        let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
        assert_eq!(quests.len(), 3);
        profile.set_total_xp(crate::gamification::XPCalculator::xp_for_level(10));
        let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
        assert_eq!(quests.len(), 4);
        profile.set_total_xp(crate::gamification::XPCalculator::xp_for_level(20));
        let quests = QuestGenerator::generate_quests(&profile, &registry, today).quests;
        assert_eq!(quests.len(), 4);
    }

    #[test]
    fn generated_quests_are_distinct_templates() {
        let registry = test_registry();
        let day = |y, m, d| crate::time::PlayerDay::from_ymd(y, m, d).unwrap();
        let days = [day(2026, 11, 11), day(2026, 1, 1), day(2025, 6, 30)];
        for level in [6, 10, 15, 16, 22, 40] {
            for today in days {
                let mut profile = UserProfile::new();
                profile.set_total_xp(crate::gamification::XPCalculator::xp_for_level(level));
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
        let mut rng =
            QuestGenerator::create_rng(crate::time::PlayerDay::from_ymd(2026, 11, 11).unwrap());
        for _ in &templates {
            assert!(
                QuestGenerator::pick_quest(&mut rng, &templates, "easy", &mut picked).is_some()
            );
        }
        assert!(QuestGenerator::pick_quest(&mut rng, &templates, "easy", &mut picked).is_none());
    }

    #[test]
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn test_generate_quests_with_empty_registry_leaves_every_slot_unfilled() {
        let today = crate::testing::today();
        for (level, slots) in [(1, 3), (10, 4), (20, 4)] {
            let mut profile = UserProfile::new();
            profile.set_total_xp(crate::gamification::XPCalculator::xp_for_level(level));

            let generation =
                QuestGenerator::generate_quests(&profile, &QuestTemplateRegistry::new(), today);

            assert!(generation.quests.is_empty());
            assert_eq!(generation.unfilled_slots, slots, "level {level}");
        }
    }

    #[test]
    fn test_generate_quests_with_a_difficulty_missing_fills_only_the_rest() {
        let today = crate::testing::today();
        let mut profile = UserProfile::new();
        profile.set_total_xp(crate::gamification::XPCalculator::xp_for_level(10));
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
            QuestDifficulty::Easy,
        )];

        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(CMD_DELETE_SELECTION),
        ));
        assert!(!quests[0].is_completed());

        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(CMD_DELETE_SELECTION),
        ));
        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(CMD_DELETE_SELECTION),
        ));
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
            QuestDifficulty::Medium,
        )];

        drop(QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("any_scenario"),
            Duration::from_secs(1),
        ));
        assert!(!quests[0].is_completed());

        drop(QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("any_scenario"),
            Duration::from_secs(1),
        ));
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
            QuestDifficulty::Hard,
        )];

        // Too slow
        drop(QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("delete_line_001"),
            Duration::from_secs(10),
        ));
        assert!(!quests[0].is_completed());

        // Fast enough
        drop(QuestTracker::update_scenario_progress(
            &mut quests,
            &sid("delete_line_001"),
            Duration::from_secs(3),
        ));
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
            QuestDifficulty::Hard,
        )];

        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(CMD_DELETE_SELECTION),
        ));
        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(CMD_YANK),
        ));
        assert!(!quests[0].is_completed());

        drop(QuestTracker::update_command_progress(
            &mut quests,
            &ncmd(crate::grammar::commands::CMD_PASTE_AFTER),
        ));
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
            QuestDifficulty::Medium,
        )];

        drop(QuestTracker::update_time_progress(&mut quests, 5));
        assert!(!quests[0].is_completed());

        drop(QuestTracker::update_time_progress(&mut quests, 5));
        assert!(quests[0].is_completed());
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

    mod mutant_gaps {
        use super::*;
        use crate::learning::ProgressTracker;

        fn exploration(target: u32, used: &[&str]) -> QuestType {
            QuestType::Exploration {
                target_commands: target,
                commands_used: used.iter().map(|c| ncmd(c)).collect(),
            }
        }

        #[test]
        fn progress_current_and_target_per_variant() {
            let practice = QuestType::CommandPractice {
                command: ncmd(CMD_YANK),
                target: 4,
                current: 1,
            };
            let scenarios = QuestType::ScenarioCompletion {
                target: 8,
                current: 2,
            };
            let time = QuestType::TimeInvested {
                target_minutes: 10,
                current_minutes: 5,
            };
            let explore = exploration(4, &["h", "j"]);
            let speed = QuestType::SpeedRun {
                scenario_id: sid("delete_line_001"),
                time_limit: Duration::from_secs(5),
            };

            let expected = [
                (&practice, 0.25, 1, 4),
                (&scenarios, 0.25, 2, 8),
                (&time, 0.5, 5, 10),
                (&explore, 0.5, 2, 4),
                (&speed, 0.0, 0, 1),
            ];
            for (quest, progress, current, target) in expected {
                assert!(
                    (quest.progress() - progress).abs() < f64::EPSILON,
                    "{quest:?}"
                );
                assert_eq!(ProgressTracker::current(quest), current, "{quest:?}");
                assert_eq!(ProgressTracker::target(quest), target, "{quest:?}");
            }
        }

        #[test]
        fn progress_is_capped_at_one() {
            let over = QuestType::TimeInvested {
                target_minutes: 5,
                current_minutes: 50,
            };
            assert!((over.progress() - 1.0).abs() < f64::EPSILON);
            assert!(over.is_completed());
        }

        #[test]
        fn completed_quest_reports_full_progress_and_target() {
            let mut quest = Quest::new(
                qid("scenario_5_easy"),
                QuestType::ScenarioCompletion {
                    target: 5,
                    current: 1,
                },
                QuestDifficulty::Easy,
            );
            assert!(!quest.status().is_completed());
            assert!(!quest.is_completed());
            assert_eq!(ProgressTracker::current(&quest), 1);
            assert_eq!(ProgressTracker::target(&quest), 5);

            quest.mark_completed();

            assert!(quest.status().is_completed());
            assert!(quest.is_completed());
            assert!((quest.progress() - 1.0).abs() < f64::EPSILON);
            assert_eq!(ProgressTracker::current(&quest), 5);
            assert_eq!(ProgressTracker::target(&quest), 5);
        }

        #[test]
        fn registry_lookup_and_size() {
            let registry = test_registry();
            assert!(!registry.is_empty());
            let first = registry
                .get_by_difficulty(crate::config::quests::QuestDifficulty::Easy)
                .first()
                .map(|t| t.id.clone())
                .expect("an easy template exists");

            assert_eq!(registry.get_by_id(&first).map(|t| &t.id), Some(&first));
            assert!(registry.get_by_id(&qid("no_such_quest")).is_none());
            let total: usize = [
                crate::config::quests::QuestDifficulty::Easy,
                crate::config::quests::QuestDifficulty::Medium,
                crate::config::quests::QuestDifficulty::Hard,
            ]
            .into_iter()
            .map(|d| registry.get_by_difficulty(d).len())
            .sum();
            assert_eq!(registry.len(), total);
        }

        #[test]
        fn speed_run_time_limit_round_trips_as_whole_seconds() {
            let quest = QuestType::SpeedRun {
                scenario_id: sid("delete_line_001"),
                time_limit: Duration::from_secs(7),
            };

            let json = serde_json::to_value(&quest).unwrap();
            assert_eq!(json["SpeedRun"]["time_limit"], 7);
            let back: QuestType = serde_json::from_value(json).unwrap();

            assert_eq!(back, quest);
        }
    }
}
