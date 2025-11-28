//! Configuration and scenario loading
//!
//! This module handles loading and parsing scenario files in TOML format,
//! as well as application configuration.

pub mod scenario_collection;
pub mod scenarios;

pub use scenario_collection::{ScenarioCollection, ScenarioFilter, SortMode};
pub use scenarios::{
    AlternativeSolution, Difficulty, Scenario, ScenarioCategory, ScenarioLoader, ScenarioMetadata,
    ScenariosFile, ScoringConfig, Setup, Solution, TargetState,
};
