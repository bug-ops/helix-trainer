//! Quest system message handlers
//!
//! Handles quest progress updates and tracking

use crate::gamification::{QuestTracker, QuestType};
use crate::security::UserError;
use crate::ui::state::{AppState, QuestProgressChange};
use std::collections::HashMap;
use std::time::Duration;

/// Track command usage for quests
///
/// Shared between training and arcade modes.
/// Updates command practice and exploration quests.
pub fn track_command_for_quests(state: &mut AppState, command: &str) {
    // Track for exploration quests
    state
        .progress
        .commands_used_today
        .insert(command.to_string());

    // Update command progress in quests
    let profile = &mut state.progress.profile;
    QuestTracker::update_command_progress(&mut profile.daily_quests, command);
}

/// Track scenario completion for quests
///
/// Shared between training and arcade modes.
/// Updates scenario completion and speed run quests.
pub fn track_scenario_completion_for_quests(
    state: &mut AppState,
    scenario_id: &str,
    duration: Duration,
) {
    let profile = &mut state.progress.profile;
    QuestTracker::update_scenario_progress(&mut profile.daily_quests, scenario_id, duration);
}

/// Check and award XP for newly completed quests
///
/// Returns total XP awarded for quest completions.
pub fn award_quest_completion_xp(state: &mut AppState, was_completed: &[bool]) -> u64 {
    let newly_completed: Vec<(String, u32)> = {
        let profile = &state.progress.profile;
        profile
            .daily_quests
            .iter()
            .enumerate()
            .filter_map(|(idx, quest)| {
                if idx < was_completed.len() && !was_completed[idx] && quest.completed {
                    Some((quest.description.clone(), quest.xp_reward))
                } else {
                    None
                }
            })
            .collect()
    };

    if !newly_completed.is_empty() {
        let total_bonus_xp: u64 = newly_completed.iter().map(|(_, xp)| *xp as u64).sum();
        let profile = &mut state.progress.profile;
        profile.add_xp(total_bonus_xp);

        // Show notifications for each completed quest
        for (description, xp_reward) in newly_completed {
            let notification = crate::ui::notification::Notification::new(
                crate::ui::notification::NotificationType::QuestComplete {
                    description,
                    xp_reward,
                },
            );
            state.ui.notifications.push(notification);
        }

        total_bonus_xp
    } else {
        0
    }
}

/// Snapshot quest completion status before updates
pub fn snapshot_quest_completion(state: &AppState) -> Vec<bool> {
    let profile = &state.progress.profile;
    profile.daily_quests.iter().map(|q| q.completed).collect()
}

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
/// Updates quest progress based on commands, scenario completions, or time.
/// This is used by training mode through the Message system.
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
        let profile = &state.progress.profile;
        profile
            .daily_quests
            .iter()
            .map(|q| (q.id.clone(), get_quest_current_progress(&q.quest_type)))
            .collect()
    };

    // Track which quests were already completed before this update
    let was_completed = snapshot_quest_completion(state);

    // Update command practice quests and exploration quests
    if let Some(cmd) = &command {
        track_command_for_quests(state, cmd);
    }

    // Update scenario completion quests and speed run quests
    if scenario_completed {
        let scenario_id = state
            .game
            .session
            .as_ref()
            .map(|s| s.scenario().id.clone())
            .unwrap_or_default();

        track_scenario_completion_for_quests(state, &scenario_id, duration);
    }

    // Update time invested quests
    let minutes = duration.as_secs() / 60;
    if minutes > 0 {
        let profile = &mut state.progress.profile;
        QuestTracker::update_time_progress(&mut profile.daily_quests, minutes as u32);
    }

    // Detect progress changes AFTER updates
    {
        let profile = &state.progress.profile;
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

    // Award XP for newly completed quests
    award_quest_completion_xp(state, &was_completed);

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
