//! Test utilities module
//!
//! This module is only compiled in test builds and provides shared utilities
//! for creating test fixtures across the codebase.
//!
//! # Example
//!
//! ```rust,no_run
//! use helix_trainer::testing::{ScenarioBuilder, default_test_scenario, empty_test_app_state};
//! use helix_trainer::config::Difficulty;
//!
//! // Create a simple test scenario
//! let scenario = default_test_scenario();
//!
//! // Create a scenario with custom configuration
//! let scenario = ScenarioBuilder::new()
//!     .id("my_test")
//!     .difficulty(Difficulty::Beginner)
//!     .build();
//!
//! // Create an empty app state for testing
//! let state = empty_test_app_state();
//! ```

pub mod app_state;
pub mod log_capture;
pub mod scenario;

// Re-export commonly used items
pub use app_state::{
    TestAppStateBuilder, empty_test_app_state, paused_minigame_return, review_session,
    run_queued_effects, start_scenario_from_menu, started_minigame_data,
    test_app_state_with_scenarios, update_executing_effects,
};
pub use scenario::{
    ScenarioBuilder, default_test_scenario, test_scenario_with_difficulty, test_scenario_with_id,
};

use crate::config::{CursorRange, CursorSpec, Pos};
use crate::gamification::{PreviousCopy, QuarantineError, QuarantineOutcome};
use crate::ids::{CommandId, NormalizedCommand, QuestId, ScenarioId};
use crate::ui::state::Quarantine;

/// A single-cursor [`CursorSpec`] at `(row, col)`, panicking on an out-of-range fixture.
///
/// # Panics
///
/// Panics if a coordinate exceeds the scenario position limit.
#[must_use]
pub fn point(row: usize, col: usize) -> CursorSpec {
    CursorSpec::Point(pos(row, col))
}

/// A single-selection [`CursorSpec`] from `anchor` to `head`, panicking on an out-of-range fixture.
///
/// # Panics
///
/// Panics if a coordinate exceeds the scenario position limit.
#[must_use]
pub fn selection(anchor: (usize, usize), head: (usize, usize)) -> CursorSpec {
    CursorSpec::Range(
        CursorRange::try_from([anchor.0, anchor.1, head.0, head.1]).expect("valid range fixture"),
    )
}

fn pos(row: usize, col: usize) -> Pos {
    Pos::try_from([row, col]).expect("valid position fixture")
}

/// Parse a canonical Normal-mode command string, panicking on a bad fixture.
///
/// # Panics
///
/// Panics if `raw` is not a command.
#[must_use]
pub fn normal(raw: &str) -> crate::helix::normal_command::NormalCommand {
    crate::helix::normal_command::NormalCommand::parse(raw).expect("valid command fixture")
}

/// A quarantined profile with `backup` and no restorable previous copy.
#[must_use]
pub fn quarantine(backup: QuarantineOutcome) -> Quarantine {
    Quarantine {
        backup,
        previous: PreviousCopy::Unavailable,
    }
}

/// A quarantine outcome for a profile whose backup could not be made.
#[must_use]
pub fn failed_backup() -> QuarantineOutcome {
    QuarantineOutcome::Failed(std::sync::Arc::new(QuarantineError::Missing))
}

/// Build a [`ScenarioId`] from a literal, panicking on an invalid test fixture.
///
/// # Panics
///
/// Panics if `raw` is not a valid fixture value.
#[must_use]
pub fn sid(raw: &str) -> ScenarioId {
    ScenarioId::parse(raw).expect("valid scenario id fixture")
}

/// Build a [`QuestId`] from a literal, panicking on an invalid test fixture.
///
/// # Panics
///
/// Panics if `raw` is not a valid fixture value.
#[must_use]
pub fn qid(raw: &str) -> QuestId {
    QuestId::parse(raw).expect("valid quest id fixture")
}

/// Build a [`CommandId`] from a canonical card id literal.
///
/// # Panics
///
/// Panics if `raw` is not a canonical card id.
#[must_use]
pub fn cid(raw: &str) -> CommandId {
    CommandId::from_card_id(raw).expect("canonical card id fixture")
}

/// Build a [`NormalizedCommand`] from a literal, panicking on an invalid test fixture.
///
/// # Panics
///
/// Panics if `raw` is not a valid fixture value.
#[must_use]
pub fn ncmd(raw: &str) -> NormalizedCommand {
    NormalizedCommand::from_raw(raw).expect("valid command fixture")
}
