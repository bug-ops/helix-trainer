//! Locale lookup for achievement, quest and error text
//!
//! The domain types hold ids and parameters only; the player-visible names and
//! descriptions resolve here, from `achievements.<id>.*`, `quests.<template_id>.*` and
//! `errors.*` in `locales/`.

use rust_i18n::t;

use crate::error::{SimulatorErrorKind, UserErrorKind};
use crate::gamification::{AchievementId, Quest, QuestType};

/// The text of a user-facing error, resolved from its kind.
pub fn user_error(kind: &UserErrorKind) -> String {
    match kind {
        UserErrorKind::InvalidState => t!("errors.invalid_state"),
        UserErrorKind::WrongScreen => t!("errors.wrong_screen"),
        UserErrorKind::ScenarioLoad => t!("errors.scenario_load"),
        UserErrorKind::ScenarioTooComplex => t!("errors.scenario_too_complex"),
        UserErrorKind::OperationFailed => t!("errors.operation_failed"),
        UserErrorKind::Command(simulator) => {
            t!("errors.command_failed", detail = simulator_error(simulator))
        }
        UserErrorKind::Storage => t!("errors.storage"),
    }
    .into_owned()
}

/// The text of a simulator rejection, resolved from its kind.
pub fn simulator_error(kind: &SimulatorErrorKind) -> String {
    match kind {
        SimulatorErrorKind::UnknownCommand { command } => {
            t!("errors.simulator.unknown_command", command = command)
        }
        SimulatorErrorKind::CountNotAccepted => t!("errors.simulator.count_not_accepted"),
        SimulatorErrorKind::InvalidRegex { detail } => {
            t!("errors.simulator.invalid_regex", detail = detail)
        }
        SimulatorErrorKind::PatternTooLong { max } => {
            t!("errors.simulator.pattern_too_long", max = max)
        }
        SimulatorErrorKind::PatternRequired => t!("errors.simulator.pattern_required"),
        SimulatorErrorKind::AlignSpansLines => t!("errors.simulator.align_spans_lines"),
        SimulatorErrorKind::TooManyCursors { max } => {
            t!("errors.simulator.too_many_cursors", max = max)
        }
        SimulatorErrorKind::NoSelections => t!("errors.simulator.no_selections"),
        SimulatorErrorKind::UnknownRegisterOp { op } => {
            t!("errors.simulator.unknown_register_op", op = op)
        }
        SimulatorErrorKind::ReadOnlyRegister { register } => {
            t!("errors.simulator.read_only_register", register = register)
        }
        SimulatorErrorKind::CommandLineInInsertMode => {
            t!("errors.simulator.command_line_in_insert_mode")
        }
        SimulatorErrorKind::NotACommand { input } => {
            t!("errors.simulator.not_a_command", input = input)
        }
        SimulatorErrorKind::EmptyCommandLine => t!("errors.simulator.empty_command_line"),
        SimulatorErrorKind::MissingArgument { name } => {
            t!("errors.simulator.missing_argument", name = name)
        }
        SimulatorErrorKind::TooManyArguments { name } => {
            t!("errors.simulator.too_many_arguments", name = name)
        }
        SimulatorErrorKind::NotANumber { name, argument } => {
            t!(
                "errors.simulator.not_a_number",
                name = name,
                argument = argument
            )
        }
        SimulatorErrorKind::UnknownCommandLine { name } => {
            t!("errors.simulator.unknown_command_line", name = name)
        }
    }
    .into_owned()
}

/// Display name of an achievement.
pub fn achievement_name(id: AchievementId) -> String {
    let key = format!("achievements.{id:?}.name");
    t!(key.as_str()).into_owned()
}

/// How an achievement is earned.
pub fn achievement_description(id: AchievementId) -> String {
    let key = format!("achievements.{id:?}.description");
    t!(key.as_str()).into_owned()
}

/// Display name of a quest template.
pub fn quest_name(quest: &Quest) -> String {
    let key = format!("quests.{}.name", quest.id.as_str());
    t!(key.as_str()).into_owned()
}

/// One-line goal of a quest, with its targets filled in from the quest parameters.
pub fn quest_description(quest: &Quest) -> String {
    let key = format!("quests.{}.description", quest.id.as_str());
    match &quest.quest_type {
        QuestType::CommandPractice {
            command, target, ..
        } => t!(key.as_str(), command = command.as_str(), target = target),
        QuestType::ScenarioCompletion { target, .. } => t!(key.as_str(), target = target),
        QuestType::SpeedRun {
            scenario_id,
            time_limit,
        } => t!(
            key.as_str(),
            scenario = scenario_id.as_str(),
            seconds = time_limit.as_secs()
        ),
        QuestType::TimeInvested { target_minutes, .. } => {
            t!(key.as_str(), minutes = target_minutes)
        }
        QuestType::Exploration {
            target_commands, ..
        } => t!(key.as_str(), target = target_commands),
    }
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::quests::QuestLoader;
    use crate::gamification::AchievementEngine;
    use crate::helix::error::{CommandLineError, SimulatorError};
    use crate::helix::register::ReadOnlyRegister;

    fn name() -> String {
        "goto".to_string()
    }

    /// One value of every simulator error variant, so a new variant needs a sample here.
    fn every_simulator_error() -> Vec<SimulatorError> {
        vec![
            SimulatorError::UnknownCommand {
                command: "zz".to_string(),
            },
            SimulatorError::CountNotAccepted,
            SimulatorError::invalid_pattern(&"unclosed group"),
            SimulatorError::PatternTooLong { max: 7 },
            SimulatorError::PatternRequired,
            SimulatorError::AlignSpansLines,
            SimulatorError::TooManyCursors { max: 3 },
            SimulatorError::NoSelections,
            SimulatorError::UnknownRegisterOp { op: 'z' },
            SimulatorError::ReadOnlyRegister {
                register: ReadOnlyRegister::FileName,
            },
            SimulatorError::CommandLineInInsertMode,
            CommandLineError::NotACommand {
                input: "x".to_string(),
            }
            .into(),
            CommandLineError::Empty.into(),
            CommandLineError::MissingArgument { name: name() }.into(),
            CommandLineError::TooManyArguments { name: name() }.into(),
            CommandLineError::NotANumber {
                name: name(),
                argument: "abc".to_string(),
            }
            .into(),
            CommandLineError::UnknownCommand { name: name() }.into(),
        ]
    }

    #[test]
    fn every_simulator_error_keeps_its_english_text() {
        for error in every_simulator_error() {
            let kind = SimulatorErrorKind::from(&error);
            assert_eq!(simulator_error(&kind), error.to_string(), "{error:?}");

            let user = crate::error::UserError::from(error);
            assert_eq!(user_error(&user.kind()), user.to_string());
        }
    }

    #[test]
    fn every_other_user_error_keeps_its_english_text() {
        use crate::error::UserError;

        let errors = [
            UserError::invalid_state("detail"),
            UserError::WrongScreen,
            UserError::scenario_load_error(),
            UserError::scenario_too_complex(),
            UserError::operation_failed(),
        ];
        for error in errors {
            assert_eq!(user_error(&error.kind()), error.to_string(), "{error:?}");
        }
    }

    #[test]
    fn a_read_only_register_write_is_reported_with_its_register() {
        let kind = SimulatorErrorKind::ReadOnlyRegister {
            register: ReadOnlyRegister::SelectionIndices,
        };
        assert_eq!(
            user_error(&UserErrorKind::Command(kind)),
            "Command execution failed: Register # does not support writing"
        );
    }

    #[test]
    fn every_achievement_id_resolves_to_text() {
        for achievement in AchievementEngine::all_achievements() {
            for text in [
                achievement_name(achievement.id),
                achievement_description(achievement.id),
            ] {
                assert!(
                    !text.is_empty() && !text.starts_with("achievements."),
                    "{text}"
                );
            }
        }
    }

    #[test]
    fn every_quest_template_resolves_without_leftover_placeholders() {
        let templates = QuestLoader::new().load_from_embedded("en").unwrap();
        assert!(!templates.is_empty());
        for template in &templates {
            let quest = Quest::from(template);
            for text in [quest_name(&quest), quest_description(&quest)] {
                assert!(
                    !text.is_empty() && !text.starts_with("quests.") && !text.contains("%{"),
                    "{}: {text}",
                    template.id
                );
            }
        }
    }

    #[test]
    fn quest_description_fills_the_quest_parameters() {
        let templates = QuestLoader::new().load_from_embedded("en").unwrap();
        let template = templates
            .iter()
            .find(|t| t.id.as_str() == "cmd_h_easy")
            .unwrap();
        assert_eq!(
            quest_description(&Quest::from(template)),
            "Move cursor left 10 times using 'h'"
        );
    }

    #[test]
    fn speed_run_description_fills_scenario_and_seconds() {
        let templates = QuestLoader::new().load_from_embedded("en").unwrap();
        for (id, expected) in [
            (
                "speed_delete_hard",
                "Complete 'delete_line_001' in under 5 seconds",
            ),
            (
                "speed_move_hard",
                "Complete 'move_down_basic_001' in under 3 seconds",
            ),
            (
                "speed_word_hard",
                "Complete 'word_forward_basic_001' in under 3 seconds",
            ),
        ] {
            let template = templates.iter().find(|t| t.id.as_str() == id).unwrap();
            assert_eq!(quest_description(&Quest::from(template)), expected, "{id}");
        }
    }
}
