//! Daily quest generation and tracking

use chrono::{Datelike, Utc};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

use crate::config::quests::{QuestLoader, QuestTemplate};
use crate::helix::commands::{
    CMD_CHANGE, CMD_DELETE_CHAR, CMD_DELETE_LINE, CMD_INSERT, CMD_MOVE_WORD_FORWARD, CMD_YANK,
};
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
    /// Check if quest is completed
    pub fn is_completed(&self) -> bool {
        match self {
            QuestType::CommandPractice {
                target, current, ..
            } => current >= target,
            QuestType::ScenarioCompletion { target, current } => current >= target,
            QuestType::SpeedRun { .. } => false, // Completed in one attempt
            QuestType::TimeInvested {
                target_minutes,
                current_minutes,
            } => current_minutes >= target_minutes,
            QuestType::Exploration {
                target_commands,
                commands_used,
            } => commands_used.len() >= *target_commands as usize,
        }
    }

    /// Get progress percentage (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        match self {
            QuestType::CommandPractice {
                target, current, ..
            } => (*current as f64) / (*target as f64).max(1.0),
            QuestType::ScenarioCompletion { target, current } => {
                (*current as f64) / (*target as f64).max(1.0)
            }
            QuestType::SpeedRun { .. } => 0.0,
            QuestType::TimeInvested {
                target_minutes,
                current_minutes,
            } => (*current_minutes as f64) / (*target_minutes as f64).max(1.0),
            QuestType::Exploration {
                target_commands,
                commands_used,
            } => (commands_used.len() as f64) / (*target_commands as f64).max(1.0),
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

    /// Check if quest is completed
    pub fn is_completed(&self) -> bool {
        self.completed || self.quest_type.is_completed()
    }

    /// Get progress (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        if self.completed {
            1.0
        } else {
            self.quest_type.progress()
        }
    }

    /// Mark quest as completed
    pub fn mark_completed(&mut self) {
        self.completed = true;
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
        registry: Option<&QuestTemplateRegistry>,
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
/// Acts as an optional enhancement to the hardcoded quest generation logic.
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

    /// Load quest templates from the default path for a given locale
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
        let templates = loader.load_for_locale(locale)?;

        tracing::info!(
            count = templates.len(),
            locale = locale,
            "Loaded quest templates from TOML"
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
    /// # Registry
    ///
    /// If a QuestTemplateRegistry is provided, quests will be selected from
    /// the loaded templates when available. If None or registry is empty,
    /// falls back to hardcoded quest generation.
    ///
    /// # Examples
    ///
    /// ```
    /// use helix_trainer::gamification::{QuestGenerator, UserProfile};
    /// use helix_trainer::learning::PerformanceTracker;
    ///
    /// let profile = UserProfile::new(); // Level 1
    /// let tracker = PerformanceTracker::new();
    /// let quests = QuestGenerator::generate_quests(&profile, &tracker, None);
    ///
    /// assert_eq!(quests.len(), 3); // Beginners get 3 quests
    /// ```
    pub fn generate_quests(
        profile: &UserProfile,
        tracker: &PerformanceTracker,
        registry: Option<&QuestTemplateRegistry>,
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
        index: usize,
        _tracker: &PerformanceTracker,
        registry: Option<&QuestTemplateRegistry>,
    ) -> Quest {
        // Try to use templates from registry if available
        if let Some(reg) = registry {
            let templates = reg.get_by_difficulty(crate::config::quests::QuestDifficulty::Easy);
            if !templates.is_empty() {
                let choice = rng.random_range(0..templates.len());
                let template = templates[choice];
                tracing::debug!(
                    id = %template.id,
                    "Using TOML template for easy quest"
                );
                return template.to_quest();
            }
        }

        // Fallback to hardcoded quests
        tracing::debug!("Using hardcoded easy quest (no templates available)");

        let quest_types = [
            // Command practice (basic commands)
            (CMD_DELETE_LINE, 3, "Delete 3 lines"),
            (CMD_YANK, 3, "Yank 3 lines"),
            (CMD_MOVE_WORD_FORWARD, 5, "Move forward 5 words"),
            (CMD_DELETE_CHAR, 5, "Delete 5 characters"),
        ];

        let choice = rng.random_range(0..quest_types.len());
        let (cmd, target, desc) = quest_types[choice];

        let id = format!("quest_easy_{}", index);
        let quest_type = QuestType::CommandPractice {
            command: cmd.to_string(),
            target,
            current: 0,
        };

        Quest::new(id, quest_type, desc.to_string(), QuestDifficulty::Easy)
    }

    fn generate_medium_quest(
        rng: &mut StdRng,
        index: usize,
        _tracker: &PerformanceTracker,
        registry: Option<&QuestTemplateRegistry>,
    ) -> Quest {
        // Try to use templates from registry if available
        if let Some(reg) = registry {
            let templates = reg.get_by_difficulty(crate::config::quests::QuestDifficulty::Medium);
            if !templates.is_empty() {
                let choice = rng.random_range(0..templates.len());
                let template = templates[choice];
                tracing::debug!(
                    id = %template.id,
                    "Using TOML template for medium quest"
                );
                return template.to_quest();
            }
        }

        // Fallback to hardcoded quests
        tracing::debug!("Using hardcoded medium quest (no templates available)");

        let quest_types = [
            // Scenario completion
            (0, "Complete 2 scenarios"),
            // Command practice (intermediate)
            (1, "Practice insert mode 5 times"),
            (2, "Use change command 3 times"),
            // Time invested
            (3, "Practice for 5 minutes"),
        ];

        let choice = rng.random_range(0..quest_types.len());
        let (variant, desc) = quest_types[choice];

        let id = format!("quest_medium_{}", index);
        let quest_type = match variant {
            0 => QuestType::ScenarioCompletion {
                target: 2,
                current: 0,
            },
            1 => QuestType::CommandPractice {
                command: CMD_INSERT.to_string(),
                target: 5,
                current: 0,
            },
            2 => QuestType::CommandPractice {
                command: CMD_CHANGE.to_string(),
                target: 3,
                current: 0,
            },
            3 => QuestType::TimeInvested {
                target_minutes: 5,
                current_minutes: 0,
            },
            _ => unreachable!(),
        };

        Quest::new(id, quest_type, desc.to_string(), QuestDifficulty::Medium)
    }

    fn generate_hard_quest(
        rng: &mut StdRng,
        index: usize,
        registry: Option<&QuestTemplateRegistry>,
    ) -> Quest {
        // Try to use templates from registry if available
        if let Some(reg) = registry {
            let templates = reg.get_by_difficulty(crate::config::quests::QuestDifficulty::Hard);
            if !templates.is_empty() {
                let choice = rng.random_range(0..templates.len());
                let template = templates[choice];
                tracing::debug!(
                    id = %template.id,
                    "Using TOML template for hard quest"
                );
                return template.to_quest();
            }
        }

        // Fallback to hardcoded quests
        tracing::debug!("Using hardcoded hard quest (no templates available)");

        let quest_types = [
            // Scenario completion
            (0, "Complete 5 scenarios"),
            // Speed run (example scenario)
            (
                1,
                "Speed run: Complete 'delete_line_001' in under 5 seconds",
            ),
            // Time invested
            (2, "Practice for 15 minutes"),
        ];

        let choice = rng.random_range(0..quest_types.len());
        let (variant, desc) = quest_types[choice];

        let id = format!("quest_hard_{}", index);
        let quest_type = match variant {
            0 => QuestType::ScenarioCompletion {
                target: 5,
                current: 0,
            },
            1 => QuestType::SpeedRun {
                scenario_id: "delete_line_001".to_string(),
                time_limit: Duration::from_secs(5),
            },
            2 => QuestType::TimeInvested {
                target_minutes: 15,
                current_minutes: 0,
            },
            _ => unreachable!(),
        };

        Quest::new(id, quest_type, desc.to_string(), QuestDifficulty::Hard)
    }

    fn generate_exploration_quest(
        rng: &mut StdRng,
        index: usize,
        registry: Option<&QuestTemplateRegistry>,
    ) -> Quest {
        // Try to use exploration templates from registry if available
        if let Some(reg) = registry {
            // Exploration quests can be either Medium or Hard difficulty
            let mut templates = reg.get_by_difficulty(crate::config::quests::QuestDifficulty::Hard);
            templates.extend(reg.get_by_difficulty(crate::config::quests::QuestDifficulty::Medium));

            // Filter to only exploration type quests
            let exploration_templates: Vec<_> = templates
                .into_iter()
                .filter(|t| {
                    matches!(
                        t.quest_type,
                        crate::config::quests::QuestTypeTag::Exploration
                    )
                })
                .collect();

            if !exploration_templates.is_empty() {
                let choice = rng.random_range(0..exploration_templates.len());
                let template = exploration_templates[choice];
                tracing::debug!(
                    id = %template.id,
                    "Using TOML template for exploration quest"
                );
                return template.to_quest();
            }
        }

        // Fallback to hardcoded quests
        tracing::debug!("Using hardcoded exploration quest (no templates available)");

        let id = format!("quest_exploration_{}", index);
        let quest_type = QuestType::Exploration {
            target_commands: 10,
            commands_used: HashSet::new(),
        };

        Quest::new(
            id,
            quest_type,
            "Use 10 different commands today".to_string(),
            QuestDifficulty::Hard,
        )
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
                } if quest_scenario == scenario_id => {
                    if duration <= *time_limit {
                        quest.completed = true;
                    }
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

    #[test]
    fn test_quest_type_completion() {
        let mut quest_type = QuestType::CommandPractice {
            command: CMD_DELETE_LINE.to_string(),
            target: 5,
            current: 3,
        };
        assert!(!quest_type.is_completed());

        quest_type = QuestType::CommandPractice {
            command: CMD_DELETE_LINE.to_string(),
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
            command: CMD_DELETE_LINE.to_string(),
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

        // Level 1 - should get 3 quests (2 easy + 1 medium)
        let mut profile = UserProfile::new();
        profile.level = 1;
        let quests = QuestGenerator::generate_quests(&profile, &tracker, None);
        assert_eq!(quests.len(), 3);

        // Level 10 - should get 4 quests (1 easy + 2 medium + 1 hard)
        profile.level = 10;
        let quests = QuestGenerator::generate_quests(&profile, &tracker, None);
        assert_eq!(quests.len(), 4);

        // Level 20 - should get 4 quests (1 medium + 2 hard + 1 exploration)
        profile.level = 20;
        let quests = QuestGenerator::generate_quests(&profile, &tracker, None);
        assert_eq!(quests.len(), 4);
    }

    #[test]
    fn test_quest_tracker_command_progress() {
        let mut quests = vec![Quest::new(
            "test".to_string(),
            QuestType::CommandPractice {
                command: CMD_DELETE_LINE.to_string(),
                target: 3,
                current: 0,
            },
            "Test".to_string(),
            QuestDifficulty::Easy,
        )];

        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_LINE);
        assert!(!quests[0].is_completed());

        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_LINE);
        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_LINE);
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

        QuestTracker::update_command_progress(&mut quests, CMD_DELETE_LINE);
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
                    command: CMD_DELETE_LINE.to_string(),
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
}
