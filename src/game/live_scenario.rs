//! The live state of one scenario being played
//!
//! [`LiveScenario`] is what training ([`GameSession`](super::GameSession)) and
//! arcade ([`ActiveMiniScenario`](crate::minigame::ActiveMiniScenario)) have in
//! common: the scenario, its simulator, the target to reach and the actions
//! taken so far. Executing an action, deciding completion and answering the
//! rendering queries live here once, so the two modes cannot drift apart.

use std::sync::Arc;
use std::time::Duration;

use super::{GameError, RecordedAction, ScenarioState, UserAction};
use crate::config::{LastAction, Scenario};
use crate::helix::InsertInput;
use crate::helix::normal_command::NormalCommand;
use crate::helix::{AnyModeSimulator, EditorDisplay, EditorSnapshot, Mode, ModeKind, Pos};
use crate::helix::{Language, SelectionBounds};
use crate::ids::CommandId;
use crate::security;

/// A scenario in play: simulator, target and the actions taken.
///
/// # Examples
///
/// ```
/// use helix_trainer::config::ScenarioLoader;
/// use helix_trainer::game::LiveScenario;
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// let scenario = ScenarioLoader::new().load_from_embedded("en")?.remove(0);
/// let mut live = LiveScenario::new(Arc::new(scenario));
/// assert_eq!(live.action_count(), 0);
/// live.apply_text("l", Duration::ZERO)?;
/// assert_eq!(live.action_count(), 1);
/// # Ok::<(), helix_trainer::error::UserError>(())
/// ```
pub struct LiveScenario {
    scenario: Arc<Scenario>,
    /// The scenario's taught commands as card ids, derived once.
    taught: Box<[CommandId]>,
    simulator: AnyModeSimulator,
    target_snapshot: EditorSnapshot,
    actions: Vec<UserAction>,
    /// Whether the editor already matched the target before the last action ran.
    target_held_before_last: bool,
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "the method path cannot name the higher-ranked lifetime or private type of the receiver"
)]
impl LiveScenario {
    /// Start a scenario from its setup, with no actions taken.
    #[must_use]
    pub fn new(scenario: Arc<Scenario>) -> Self {
        let state = ScenarioState::from_scenario(&scenario);
        let taught = scenario
            .metadata
            .commands_taught
            .iter()
            .filter_map(|taught| CommandId::from_action(taught.as_str()))
            .collect();
        Self {
            scenario,
            taught,
            simulator: state.simulator,
            target_snapshot: state.target_snapshot,
            actions: Vec::new(),
            target_held_before_last: false,
        }
    }

    /// The scenario being played.
    #[must_use]
    pub fn scenario(&self) -> &Scenario {
        &self.scenario
    }

    /// Return to the setup state and forget all actions.
    ///
    /// The simulator is rebuilt the way [`LiveScenario::new`] builds it, so the
    /// scenario's language (indent unit) survives.
    pub fn reset(&mut self) {
        self.simulator = ScenarioState::from_scenario(&self.scenario).simulator;
        self.actions.clear();
        self.target_held_before_last = false;
    }

    /// Run one parsed Normal-mode command and record it as one action at `at`.
    ///
    /// A simulator in Insert mode takes the command as the text it spells, and
    /// the action is recorded as that Insert input.
    ///
    /// # Errors
    ///
    /// Returns an error if the action limit is exceeded or the simulator
    /// rejects the command; the action is then not recorded.
    pub fn apply_command(
        &mut self,
        command: &NormalCommand,
        at: Duration,
    ) -> Result<(), GameError> {
        let action = if self.is_insert_mode() {
            RecordedAction::Insert(InsertInput::parse(&command.to_string()))
        } else {
            RecordedAction::Command(command.clone())
        };
        self.apply_action(action, at)
    }

    /// Run one typed Insert-mode input and record it as one action at `at`.
    ///
    /// # Errors
    ///
    /// Returns an error if the action limit is exceeded or the simulator
    /// rejects the input; the action is then not recorded.
    pub fn apply_insert(&mut self, input: &InsertInput, at: Duration) -> Result<(), GameError> {
        self.apply_action(RecordedAction::Insert(input.clone()), at)
    }

    /// Run one command string and record it as one action at `at`.
    ///
    /// The string is parsed once, by the simulator's mode: a possibly counted
    /// command in Normal mode, typed text or a key in Insert mode.
    ///
    /// # Errors
    ///
    /// Returns an error if the string names no command, the action limit is
    /// exceeded or the simulator rejects it; the action is then not recorded.
    pub fn apply_text(&mut self, command: &str, at: Duration) -> Result<(), GameError> {
        let action = if self.is_insert_mode() {
            RecordedAction::Insert(InsertInput::parse(command))
        } else {
            RecordedAction::Command(NormalCommand::parse(command)?)
        };
        self.apply_action(action, at)
    }

    /// Run `action` and record it at `at`.
    ///
    /// # Errors
    ///
    /// Returns an error if the action limit is exceeded or the simulator
    /// rejects the action; the action is then not recorded.
    pub fn apply_action(&mut self, action: RecordedAction, at: Duration) -> Result<(), GameError> {
        security::arithmetic::validate_action_count(self.actions.len() + 1)?;

        let held_before = self.simulator.matches_snapshot(&self.target_snapshot);
        match &action {
            RecordedAction::Command(command) => self.simulator.execute_normal(command)?,
            RecordedAction::Insert(input) => self.simulator.execute_insert(input)?,
        }
        self.target_held_before_last = held_before;
        self.actions.push(UserAction::new(action, at));
        Ok(())
    }

    /// Whether the scenario's completion rule is satisfied by the current state.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.scenario.metadata.completion.is_satisfied(
            self.simulator.matches_snapshot(&self.target_snapshot),
            &self.taught,
            self.actions.last().map(|action| LastAction {
                command: action.command_id(),
                target_held_before: self.target_held_before_last,
            }),
        )
    }

    /// The actions taken so far, in order.
    #[must_use]
    pub fn actions(&self) -> &[UserAction] {
        &self.actions
    }

    /// The number of actions taken so far.
    #[must_use]
    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    pub(crate) fn simulator(&self) -> &AnyModeSimulator {
        &self.simulator
    }

    pub(crate) fn target_snapshot(&self) -> &EditorSnapshot {
        &self.target_snapshot
    }

    /// The editor mode as an enum.
    #[must_use]
    pub fn mode(&self) -> Mode {
        self.simulator.mode()
    }

    /// The editor mode for UI display.
    #[must_use]
    pub fn mode_kind(&self) -> ModeKind {
        self.simulator.mode_kind()
    }

    /// Whether the simulator is in Insert mode.
    #[must_use]
    pub fn is_insert_mode(&self) -> bool {
        self.simulator.is_insert_mode()
    }

    /// Whether a `Q`/`q` macro is being recorded.
    #[must_use]
    pub fn is_recording_macro(&self) -> bool {
        self.simulator.is_recording_macro()
    }

    /// The scenario's content language, falling back to [`crate::config::DEFAULT_LANGUAGE`].
    #[must_use]
    pub fn language(&self) -> Language {
        self.scenario
            .setup
            .language
            .unwrap_or(crate::config::DEFAULT_LANGUAGE)
    }

    /// The current document text.
    #[must_use]
    pub fn current_content(&self) -> String {
        self.simulator.display().content()
    }

    /// The target document text.
    #[must_use]
    pub fn target_content(&self) -> String {
        self.target_snapshot.content.clone()
    }

    /// The primary cursor of the editor.
    #[must_use]
    pub fn current_cursor(&self) -> Pos {
        self.simulator.display().cursor_position()
    }

    /// The primary cursor of the target.
    #[must_use]
    pub fn target_cursor(&self) -> Pos {
        self.with_target_display(|d| d.cursor_position())
    }

    /// The primary selection of the editor, if any.
    #[must_use]
    pub fn current_selection(&self) -> Option<SelectionBounds> {
        self.simulator.display().selection()
    }

    /// The primary selection of the target, if any.
    #[must_use]
    pub fn target_selection(&self) -> Option<SelectionBounds> {
        self.with_target_display(|d| d.selection())
    }

    /// Every cursor of the editor, primary first.
    #[must_use]
    pub fn all_cursors(&self) -> Vec<Pos> {
        self.simulator.display().all_cursor_positions()
    }

    /// Every non-empty selection of the editor.
    #[must_use]
    pub fn all_selections(&self) -> Vec<SelectionBounds> {
        self.simulator.display().all_selection_bounds()
    }

    /// Every cursor of the target, primary first.
    #[must_use]
    pub fn all_target_cursors(&self) -> Vec<Pos> {
        self.with_target_display(|d| d.all_cursor_positions())
    }

    /// Every non-empty selection of the target.
    #[must_use]
    pub fn all_target_selections(&self) -> Vec<SelectionBounds> {
        self.with_target_display(|d| d.all_selection_bounds())
    }

    /// Run `f` on a display facade built from the target snapshot.
    fn with_target_display<T>(&self, f: impl FnOnce(&EditorDisplay<'_>) -> T) -> T {
        let rope = helix_core::Rope::from(self.target_snapshot.content.as_str());
        let selection = self.target_snapshot.to_helix_selection();
        f(&EditorDisplay::new(&rope, &selection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{ScenarioBuilder, normal};

    fn live() -> LiveScenario {
        LiveScenario::new(Arc::new(
            ScenarioBuilder::new()
                .setup_content("ab\ncd\n")
                .target_content("ab\ncd\n")
                .target_cursor(0, 1)
                .build(),
        ))
    }

    #[test]
    fn typed_and_string_actions_record_the_same_canonical_command() {
        let mut typed = live();
        let mut text = live();

        typed
            .apply_command(&normal("l"), Duration::from_secs(2))
            .unwrap();
        text.apply_text("l", Duration::from_secs(2)).unwrap();

        assert_eq!(typed.actions(), text.actions());
        assert_eq!(typed.actions()[0].timestamp, Duration::from_secs(2));
        assert_eq!(typed.current_cursor(), text.current_cursor());
    }

    #[test]
    fn a_rejected_command_is_not_recorded() {
        let mut live = live();

        assert!(live.apply_text("no-such-command", Duration::ZERO).is_err());

        assert_eq!(live.action_count(), 0);
    }

    #[test]
    fn completion_follows_the_target_and_reset_starts_over() {
        let mut live = live();
        assert!(!live.is_completed());

        live.apply_text("l", Duration::ZERO).unwrap();
        assert!(live.is_completed());

        live.reset();
        assert_eq!(live.action_count(), 0);
        assert!(!live.is_completed());
    }

    #[test]
    fn target_queries_read_the_target_snapshot() {
        let live = live();

        assert_eq!(live.target_cursor(), Pos::new(0, 1));
        assert_eq!(live.all_target_cursors(), vec![Pos::new(0, 1)]);
        assert_eq!(live.target_content(), "ab\ncd\n");
    }
}
