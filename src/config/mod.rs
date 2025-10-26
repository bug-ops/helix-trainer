//! Configuration and scenario loading
//!
//! This module handles loading and parsing scenario files in TOML format,
//! as well as application configuration.

pub mod scenarios;

pub use scenarios::{
    AlternativeSolution, Scenario, ScenarioLoader, ScenariosFile, ScoringConfig, Setup, Solution,
    TargetState,
};
