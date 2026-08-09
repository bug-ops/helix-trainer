//! Daily quest generation and tracking

use chrono::{Datelike, Utc};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

use crate::config::quests::{QuestLoader, QuestTemplate};
use crate::learning::PerformanceTracker;
use crate::security::UserError;

use super::{UserProfile, XPCalculator};

/// Quest difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestDifficulty {
    Easy,
    Medium,
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
        command: String,
        target: u32,
        current: u32,
    },

    /// Complete N scenarios
    ScenarioCompletion { target: u32, current: u32 },

    /// Complete scenario within time limit
    SpeedRun {
        scenario_id: String,
        #[serde(with = "duration_serde")]
        time_limit: Duration,
    },

    /// Practice for N minutes
    TimeInvested {
        target_minutes: u32,
        current_minutes: u32,
    },

    /// Use N different commands
    Exploration {
        target_commands: u32,
        commands_used: HashSet<String>,
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
    fn progress(&self) -> f64 {
        match self {
            QuestType::CommandPractice {
                target, current, ..
            } => (*current as f64 / (*target).max(1) as f64).min(1.0),
            QuestType::ScenarioCompletion { target, current } => {
                (*current as f64 / (*target).max(1) as f64).min(1.0)
            }
            QuestType::SpeedRun { .. } => 0.0,
            QuestType::TimeInvested {
                target_minutes,
                current_minutes,
            } => (*current_minutes as f64 / (*target_minutes).max(1) as f64).min(1.0),
            QuestType::Exploration {
                target_commands,
                commands_used,
            } => (commands_used.len() as f64 / (*target_commands).max(1) as f64).min(1.0),
        }
    }

    fn current(&self) -> u64 {
        match self {
            QuestType::CommandPractice { current, .. } => *current as u64,
            QuestType::ScenarioCompletion { current, .. } => *current as u64,
            QuestType::SpeedRun { .. } => 0,
            QuestType::TimeInvested {
                current_minutes, ..
            } => *current_minutes as u64,
            QuestType::Exploration { commands_used, .. } => commands_used.len() as u64,
        }
    }

    fn target(&self) -> u64 {
        match self {
            QuestType::CommandPractice { target, .. } => *target as u64,
            QuestType::ScenarioCompletion { target, .. } => *target as u64,
            QuestType::SpeedRun { .. } => 1,
            QuestType::TimeInvested { target_minutes, .. } => *target_minutes as u64,
            QuestType::Exploration {
                target_commands, ..
            } => *target_commands as u64,
        }
    }
}

/// A daily quest
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub quest_type: QuestType,
    pub description: String,
    pub difficulty: QuestDifficulty,
    pub xp_reward: u32,
    pub completed: bool,
}

impl Quest {
    /// Create a new quest
    pub fn new(
        id: String,
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
        tracker: &PerformanceTracker,
        registry: &QuestTemplateRegistry,
    ) -> Vec<Quest> {
        let mut quests = Vec::new();

        for i in 0..self.easy {
            quests.push(QuestGenerator::generate_easy_quest(
                rng, i, tracker, registry,
            ));
        }

        for i in 0..self.medium {
            quests.push(QuestGenerator::generate_medium_quest(
                rng, i, tracker, registry,
            ));
        }

        for i in 0..self.hard {
            quests.push(QuestGenerator::generate_hard_quest(rng, i, registry));
        }

        for i in 0..self.exploration {
            quests.push(QuestGenerator::generate_exploration_quest(rng, i, registry));
        }

        quests
    }
}

/// Registry for quest templates loaded from TOML files
///
/// Provides caching and filtering of quest templates loaded from configuration files.
/// Quest templates are defined in TOML files under `quests/{locale}/daily.toml`.
#[derive(Debug, Clone)]
pub struct QuestTemplateRegistry {
    templates: Vec<QuestTemplate>,
}

impl QuestTemplateRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            templates: Vec::new(),
        }
    }

    /// Load quest templates from embedded data for a given locale
    ///
    /// This method loads quests from compile-time embedded content,
    /// eliminating filesystem access for consistent behavior.
    ///
    /// # Errors
    /// Returns UserError if loading fails
    ///
    /// # Examples
    /// ```ignore
    /// let registry = QuestTemplateRegistry::load_from_default_path("en")?;
    /// ```
    pub fn load_from_default_path(locale: &str) -> Result<Self, UserError> {
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
    pub fn get_by_id(&self, id: &str) -> Option<&QuestTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    /// Get all eligible quests based on user conditions
    ///
    /// Filters quests by:
    /// - Level requirements (min_level, max_level)
    /// - Required commands (user must have used these)
    /// - Required scenarios (user must have completed these)
    pub fn get_eligible_quests(
        &self,
        user_level: u32,
        commands_used: &HashSet<String>,
        scenarios_completed: &HashSet<String>,
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
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::gamification::{QuestGenerator, QuestTemplateRegistry, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    ///
    /// let profile = UserProfile::new(); // Level 1
    /// let tracker = PerformanceTracker::new();
    /// let registry = QuestTemplateRegistry::load_from_default_path("en")?;
    /// let quests = QuestGenerator::generate_quests(&profile, &tracker, &registry);
    ///
    /// assert_eq!(quests.len(), 3); // Beginners get 3 quests
    /// ```
    pub fn generate_quests(
        profile: &UserProfile,
        tracker: &PerformanceTracker,
        registry: &QuestTemplateRegistry,
    ) -> Vec<Quest> {
        let mut rng = Self::create_rng();
        let distribution = QuestDistribution::for_level(profile.level);
        distribution.generate_quests(&mut rng, tracker, registry)
    }

    fn create_rng() -> StdRng {
        // Seed with current date for consistency within a day
        let today = Utc::now().date_naive();
        let seed = today.num_days_from_ce() as u64;
        StdRng::seed_from_u64(seed)
    }

    fn generate_easy_quest(
        rng: &mut StdRng,
        _index: usize,
        _tracker: &PerformanceTracker,
        registry: &QuestTemplateRegistry,
    ) -> Quest {
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Easy);
        assert!(!templates.is_empty(), "No easy quest templates available");

        let choice = rng.random_range(0..templates.len());
        let template = templates[choice];
        tracing::debug!(id = %template.id, "Selected easy quest template");
        template.to_quest()
    }

    fn generate_medium_quest(
        rng: &mut StdRng,
        _index: usize,
        _tracker: &PerformanceTracker,
        registry: &QuestTemplateRegistry,
    ) -> Quest {
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Medium);
        assert!(!templates.is_empty(), "No medium quest templates available");

        let choice = rng.random_range(0..templates.len());
        let template = templates[choice];
        tracing::debug!(id = %template.id, "Selected medium quest template");
        template.to_quest()
    }

    fn generate_hard_quest(
        rng: &mut StdRng,
        _index: usize,
        registry: &QuestTemplateRegistry,
    ) -> Quest {
        let templates = registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Hard);
        assert!(!templates.is_empty(), "No hard quest templates available");

        let choice = rng.random_range(0..templates.len());
        let template = templates[choice];
        tracing::debug!(id = %template.id, "Selected hard quest template");
        template.to_quest()
    }

    fn generate_exploration_quest(
        rng: &mut StdRng,
        _index: usize,
        registry: &QuestTemplateRegistry,
    ) -> Quest {
        // Exploration quests can be either Medium or Hard difficulty
        let mut templates =
            registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Hard);
        templates
            .extend(registry.get_by_difficulty(crate::config::quests::QuestDifficulty::Medium));

        // Filter to only exploration type quests
        let exploration_templates: Vec<_> = templates
            .into_iter()
            .filter(|t| matches!(t.spec, crate::config::quests::QuestSpec::Exploration { .. }))
            .collect();

        assert!(
            !exploration_templates.is_empty(),
            "No exploration quest templates available"
        );

        let choice = rng.random_range(0..exploration_templates.len());
        let template = exploration_templates[choice];
        tracing::debug!(id = %template.id, "Selected exploration quest template");
        template.to_quest()
    }
}

/// Tracks quest progress and updates
pub struct QuestTracker;

impl QuestTracker {
    /// Update quest progress based on command execution
    pub fn update_command_progress(quests: &mut [Quest], command: &str) {
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
                    *current = (*current + 1).min(*target);
                    if *current >= *target {
                        quest.completed = true;
                    }
                }
                QuestType::Exploration {
                    target_commands,
                    commands_used,
                } => {
                    commands_used.insert(command.to_string());
                    if commands_used.len() >= *target_commands as usize {
                        quest.completed = true;
                    }
                }
                _ => {}
            }
        }
    }

    /// Update quest progress for scenario completion
    pub fn update_scenario_progress(quests: &mut [Quest], scenario_id: &str, duration: Duration) {
        for quest in quests.iter_mut() {
            if quest.completed {
                continue;
            }

            match &mut quest.quest_type {
                QuestType::ScenarioCompletion { target, current } => {
                    *current = (*current + 1).min(*target);
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
                *current_minutes = (*current_minutes + minutes).min(*target_minutes);
                if *current_minutes >= *target_minutes {
                    quest.completed = true;
                }
            }
        }
    }

    /// Check if any quests were newly completed and return their XP rewards
    pub fn check_completions(quests: &[Quest]) -> Vec<(String, u32)> {
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
    use crate::helix::commands::{CMD_DELETE_SELECTION, CMD_YANK};

    /// Helper to create a registry for tests
    fn test_registry() -> QuestTemplateRegistry {
        QuestTemplateRegistry::load_from_default_path("en")
            .expect("Failed to load quest templates for tests")
    }

    #[test]
    fn test_quest_type_completion() {
        let mut quest_type = QuestType::CommandPractice {
            command: CMD_DELETE_SELECTION.to_string(),
            target: 5,
            current: 3,
        };
        assert!(!quest_type.is_completed());

        quest_type = QuestType::CommandPractice {
            command: CMD_DELETE_SELECTION.to_string(),
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
            command: CMD_DELETE_SELECTION.to_string(),
            target: 5,
            current: 0,
        };
        let quest = Quest::new(
            "test_quest".to_string(),
            quest_type,
            "Test quest".to_string(),
            QuestDifficulty::Easy,
        );

        assert_eq!(quest.id, "test_quest");
        assert_eq!(quest.xp_reward, 25); // Easy command practice
        assert!(!quest.completed);
    }

    #[test]
    fn test_quest_generator_level_based() {
        let tracker = PerformanceTracker::new();
        let registry = test_registry();

        // Level 1 - should get 3 quests (2 easy + 1 medium)
        let mut profile = UserProfile::new();
        profile.level = 1;
        let quests = QuestGenerator::generate_quests(&profile, &tracker, &registry);
        assert_eq!(quests.len(), 3);

        // Level 10 - should get 4 quests (1 easy + 2 medium + 1 hard)
        profile.level = 10;
        let quests = QuestGenerator::generate_quests(&profile, &tracker, &registry);
        assert_eq!(quests.len(), 4);

        // Level 20 - should get 4 quests (1 medium + 2 hard + 1 exploration)
        profile.level = 20;
        let quests = QuestGenerator::generate_quests(&profile, &tracker, &registry);
        assert_eq!(quests.len(), 4);
    }

    #[test]
    fn test_quest_tracker_command_progress() {
        let mut quests = vec![Quest::new(
            "test".to_string(),
            QuestType::CommandPractice {
                command: CMD_DELETE_SELECTION.to_string(),
                target: 3,
                current: 0,
            },
            "Test".to_string(),
            QuestDifficulty::Easy,
        )];

        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_SELECTION);
        assert!(!quests[0].is_completed());

        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_SELECTION);
        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_SELECTION);
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_scenario_progress() {
        let mut quests = vec![Quest::new(
            "test".to_string(),
            QuestType::ScenarioCompletion {
                target: 2,
                current: 0,
            },
            "Test".to_string(),
            QuestDifficulty::Medium,
        )];

        QuestTracker::update_scenario_progress(&mut quests, "any_scenario", Duration::from_secs(1));
        assert!(!quests[0].is_completed());

        QuestTracker::update_scenario_progress(&mut quests, "any_scenario", Duration::from_secs(1));
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_speed_run() {
        let mut quests = vec![Quest::new(
            "test".to_string(),
            QuestType::SpeedRun {
                scenario_id: "delete_line_001".to_string(),
                time_limit: Duration::from_secs(5),
            },
            "Test".to_string(),
            QuestDifficulty::Hard,
        )];

        // Too slow
        QuestTracker::update_scenario_progress(
            &mut quests,
            "delete_line_001",
            Duration::from_secs(10),
        );
        assert!(!quests[0].is_completed());

        // Fast enough
        QuestTracker::update_scenario_progress(
            &mut quests,
            "delete_line_001",
            Duration::from_secs(3),
        );
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_exploration() {
        let mut quests = vec![Quest::new(
            "test".to_string(),
            QuestType::Exploration {
                target_commands: 3,
                commands_used: HashSet::new(),
            },
            "Test".to_string(),
            QuestDifficulty::Hard,
        )];

        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_SELECTION);
        QuestTracker::update_command_progress(&mut quests, CMD_YANK);
        assert!(!quests[0].is_completed());

        QuestTracker::update_command_progress(&mut quests, crate::helix::commands::CMD_PASTE_AFTER);
        assert!(quests[0].is_completed());
    }

    #[test]
    fn test_quest_tracker_time_invested() {
        let mut quests = vec![Quest::new(
            "test".to_string(),
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
                id: "completed".to_string(),
                quest_type: QuestType::CommandPractice {
                    command: CMD_DELETE_SELECTION.to_string(),
                    target: 1,
                    current: 1,
                },
                description: "Test".to_string(),
                difficulty: QuestDifficulty::Easy,
                xp_reward: 25,
                completed: true,
            },
            Quest {
                id: "incomplete".to_string(),
                quest_type: QuestType::CommandPractice {
                    command: CMD_YANK.to_string(),
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
        assert_eq!(completions[0].0, "completed");
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
