//! Quest system message handlers
//!
//! Handles quest progress updates and tracking

use crate::gamification::{QuestTracker, QuestType};
use crate::security::UserError;
use crate::ui::state::{AppState, QuestProgressChange};
use std::collections::HashMap;
use std::time::Duration;

/// Get current progress value from quest type
fn get_quest_current_progress(quest_type: &QuestType) -> u32 {
    use QuestType::*;
    match quest_type {
        CommandPractice { current, .. } => *current,
        ScenarioCompletion { current, .. } => *current,
        TimeInvested {
            current_minutes, ..
        } => *current_minutes,
        Exploration { commands_used, .. } => commands_used.len() as u32,
        SpeedRun { .. } => 0, // Single-attempt quest
    }
}

/// Handle UpdateQuestProgress message
///
/// Updates quest progress based on commands, scenario completions, or time
pub fn handle_update_quest_progress(
    state: &mut AppState,
    command: Option<String>,
    scenario_completed: bool,
    duration: Duration,
) -> Result<(), UserError> {
    // Clear previous progress changes
    state.ui.quest_progress_changes.clear();

    // Snapshot progress BEFORE updates
    let progress_before: HashMap<String, u32> = {
        let profile = state.progress.profile.borrow();
        profile
            .daily_quests
            .iter()
            .map(|q| (q.id.clone(), get_quest_current_progress(&q.quest_type)))
            .collect()
    };

    // Track which quests were already completed before this update
    let was_completed: Vec<bool> = {
        let profile = state.progress.profile.borrow();
        profile.daily_quests.iter().map(|q| q.completed).collect()
    };

    // Update command practice quests and exploration quests
    if let Some(cmd) = &command {
        // Track for exploration quests
        state.progress.commands_used_today.insert(cmd.clone());

        // Update command progress in quests
        let mut profile = state.progress.profile.borrow_mut();
        QuestTracker::update_command_progress(&mut profile.daily_quests, cmd);
    }

    // Update scenario completion quests and speed run quests
    if scenario_completed {
        let scenario_id = state
            .game
            .session
            .as_ref()
            .map(|s| s.scenario().id.clone())
            .unwrap_or_default();

        let mut profile = state.progress.profile.borrow_mut();
        QuestTracker::update_scenario_progress(&mut profile.daily_quests, &scenario_id, duration);
    }

    // Update time invested quests
    let minutes = duration.as_secs() / 60;
    if minutes > 0 {
        let mut profile = state.progress.profile.borrow_mut();
        QuestTracker::update_time_progress(&mut profile.daily_quests, minutes as u32);
    }

    // Detect progress changes AFTER updates
    {
        let profile = state.progress.profile.borrow();
        for quest in &profile.daily_quests {
            let old = progress_before.get(&quest.id).copied().unwrap_or(0);
            let new = get_quest_current_progress(&quest.quest_type);

            if new > old {
                state.ui.quest_progress_changes.push(QuestProgressChange {
                    quest_description: super::format_quest_description(&quest.quest_type),
                    old_progress: old,
                    new_progress: new,
                });
            }
        }
    }

    // Check for newly completed quests and award bonus XP
    let newly_completed_xp: Vec<u32> = {
        let profile = state.progress.profile.borrow();
        profile
            .daily_quests
            .iter()
            .enumerate()
            .filter_map(|(idx, quest)| {
                if !was_completed[idx] && quest.completed {
                    Some(quest.xp_reward)
                } else {
                    None
                }
            })
            .collect()
    };

    // Award XP for newly completed quests
    if !newly_completed_xp.is_empty() {
        let total_bonus_xp: u64 = newly_completed_xp.iter().map(|xp| *xp as u64).sum();
        let mut profile = state.progress.profile.borrow_mut();
        profile.add_xp(total_bonus_xp);
    }

    Ok(())
}

/// Format quest type as readable description
pub fn format_quest_description(quest_type: &QuestType) -> String {
    use QuestType::*;
    match quest_type {
        CommandPractice {
            command, target, ..
        } => format!("Use '{}' {} times", command, target),
        ScenarioCompletion { target, .. } => format!("Complete {} scenarios", target),
        SpeedRun { scenario_id, .. } => format!("Speed run: {}", scenario_id),
        TimeInvested { target_minutes, .. } => format!("Practice {} min", target_minutes),
        Exploration {
            target_commands, ..
        } => format!("Try {} commands", target_commands),
    }
}
