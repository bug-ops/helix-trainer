//! Quest system message handlers
//!
//! Handles quest progress updates and tracking

use crate::gamification::{QuestTracker, QuestType, StreakManager};
use crate::security::UserError;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::{AppState, QuestProgressChange};
use std::collections::HashMap;
use std::time::Duration;

/// Track command usage for quests
///
/// Shared between training and arcade modes.
/// Updates command practice and exploration quests.
///
/// Normalizes register ops (`"ay` -> `"y`) and command-line invocations
/// (`:g 3` -> `:goto`) before tracking, so quest matching (exact equality)
/// and FSRS see the same command id for the same skill.
pub fn track_command_for_quests(state: &mut AppState, command: &str) {
    let normalized = crate::helix::commands::normalize_command_id(command);

    // Track for exploration quests
    state
        .progress
        .commands_used_today
        .insert(normalized.to_string());

    // Update command progress in quests
    let profile = &mut state.progress.profile;
    QuestTracker::update_command_progress(&mut profile.daily_quests, &normalized);
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

/// Result of [`award_quest_completion_xp`]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct QuestXpAward {
    /// (description, xp) pairs for each quest newly completed by this call
    pub bonuses: Vec<(String, u64)>,
    /// Whether the XP granted here alone crossed an account level-up threshold.
    /// Callers that also call `profile.add_xp` for other XP in the same operation
    /// (e.g. scenario completion) must OR this into their own level-up check, since
    /// each `add_xp` call only reports whether *that* call crossed a level boundary.
    pub leveled_up: bool,
}

/// Check and award XP for newly completed quests
///
/// Marks each newly completed quest on the profile's `completed_quests_today` set (which
/// gates next-day streak increments), grants a streak freeze once every quest generated
/// for today is completed, and returns the (description, xp) pairs for each quest newly
/// completed by this call, for the caller to display or total up. This is the only place
/// quest-completion XP is applied to `profile.xp` - callers must not re-add it.
pub fn award_quest_completion_xp(state: &mut AppState, was_completed: &[bool]) -> QuestXpAward {
    let newly_completed: Vec<(String, String, u32)> = {
        let profile = &state.progress.profile;
        profile
            .daily_quests
            .iter()
            .enumerate()
            .filter_map(|(idx, quest)| {
                if idx < was_completed.len() && !was_completed[idx] && quest.completed {
                    Some((quest.id.clone(), quest.description.clone(), quest.xp_reward))
                } else {
                    None
                }
            })
            .collect()
    };

    if newly_completed.is_empty() {
        return QuestXpAward::default();
    }

    let total_bonus_xp: u64 = newly_completed.iter().map(|(_, _, xp)| *xp as u64).sum();
    let profile = &mut state.progress.profile;
    let leveled_up = profile.add_xp(total_bonus_xp);
    for (quest_id, _, _) in &newly_completed {
        profile.complete_quest(quest_id.clone());
    }

    // Show notifications for each completed quest
    let bonuses: Vec<(String, u64)> = newly_completed
        .iter()
        .map(|(_, description, xp_reward)| (description.clone(), *xp_reward as u64))
        .collect();
    for (_, description, xp_reward) in newly_completed {
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::QuestComplete {
                description,
                xp_reward,
            }));
    }

    // Grant a streak freeze once every quest generated for today is completed
    if StreakManager::check_freeze_eligibility(&state.progress.profile) {
        StreakManager::grant_freeze(&mut state.progress.profile);
        state
            .ui
            .notifications
            .push(Notification::new(NotificationType::StreakFreezeGranted));
    }

    QuestXpAward {
        bonuses,
        leveled_up,
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
    scenario_id: Option<String>,
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
    if scenario_completed
        && let Some(id) = scenario_id
        && crate::learning::is_valid_scenario_id(&id)
    {
        track_scenario_completion_for_quests(state, &id, duration);
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

    // Award XP for newly completed quests, keeping the breakdown and level-up outcome
    // for the caller (results display, level-up notification/save)
    let award = award_quest_completion_xp(state, &was_completed);
    state.ui.quest_xp_bonuses = award.bonuses;
    state.ui.quest_leveled_up = award.leveled_up;

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
