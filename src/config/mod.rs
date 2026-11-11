//! Configuration and scenario loading
//!
//! This module handles loading and parsing scenario files in TOML format,
//! as well as application configuration.

#![cfg_attr(
    not(test),
    warn(
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::unwrap_used,
        clippy::expect_used
    )
)]

pub mod app_config;
pub mod data_dir;
pub mod json_store;
pub mod keymap;
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
    AlternativeSolution, CompletionRule, CursorSpec, DEFAULT_LANGUAGE, Difficulty, LastAction,
    Scenario, ScenarioCategory, ScenarioLoader, ScenarioMetadata, ScenariosFile, ScoringConfig,
    Setup, Solution, TargetState,
};
