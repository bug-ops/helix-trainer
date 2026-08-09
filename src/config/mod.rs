//! Configuration and scenario loading
//!
//! This module handles loading and parsing scenario files in TOML format,
//! as well as application configuration.

pub mod app_config;
mod loader;
pub mod quests;
pub mod scenario_collection;
pub mod scenarios;

pub use app_config::{AppConfig, ConfigStorage};
pub use quests::{
    QuestConditions, QuestDifficulty, QuestLoader, QuestSpec, QuestTemplate, QuestsFile,
    QuestsMetadata, XpConfig,
};
pub use scenario_collection::{
    CompletionFilter, CurriculumStats, ScenarioCollection, ScenarioFilter, SortMode,
};
pub use scenarios::{
    AlternativeSolution, CursorSpec, Difficulty, Scenario, ScenarioCategory, ScenarioLoader,
    ScenarioMetadata, ScenariosFile, ScoringConfig, Setup, Solution, TargetState,
};
