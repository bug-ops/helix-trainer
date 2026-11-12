//! Locale lookup for achievement and quest text
//!
//! The domain types hold ids and parameters only; the player-visible names and
//! descriptions resolve here, from `achievements.<id>.*` and `quests.<template_id>.*`
//! in `locales/`.

use rust_i18n::t;

use crate::gamification::{AchievementId, Quest, QuestType};

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
        QuestType::SpeedRun { .. } => t!(key.as_str()),
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
}
