//! Quest system message handlers
//!
//! Handles quest progress updates and tracking

use crate::gamification::{QuestTracker, QuestType, StreakManager};
use crate::helix::normal_command::NormalCommand;
use crate::ids::{NormalizedCommand, QuestId, ScenarioId};
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::QuestBonus;
use crate::ui::state::{AppState, QuestEvent, QuestProgressChange};
use crate::ui::text;
use std::collections::HashMap;
use std::time::Duration;

/// The quest form of an executed Normal-mode command, or `None` (logged) when it has none.
pub(crate) fn quest_command(command: &NormalCommand) -> Option<NormalizedCommand> {
    command
        .normalized()
        .inspect_err(|error| {
            tracing::warn!(%error, %command, "Executed command has no quest form");
        })
        .ok()
}

/// Track command usage for quests
///
/// Shared between training and arcade modes.
/// Updates command practice and exploration quests.
///
/// The command is already normalized (register ops `"ay` -> `"y`, command lines
/// `:g 3` -> `:goto`), so quest matching (exact equality) and FSRS see the same
/// command id for the same skill.
pub(crate) fn track_command_for_quests(state: &mut AppState, command: &NormalizedCommand) {
    // Track for exploration quests
    state.progress.commands_used_today.insert(command.clone());

    // Update command progress in quests
    let profile = &mut state.progress.profile;
    QuestTracker::update_command_progress(&mut profile.daily_quests, command);
}

/// Track scenario completion for quests
///
/// Shared between training and arcade modes.
/// Updates scenario completion and speed run quests.
pub(crate) fn track_scenario_completion_for_quests(
    state: &mut AppState,
    scenario_id: &ScenarioId,
    duration: Duration,
) {
    let profile = &mut state.progress.profile;
    QuestTracker::update_scenario_progress(&mut profile.daily_quests, scenario_id, duration);
}

/// Track time invested for quests, in whole minutes
fn track_time_for_quests(state: &mut AppState, duration: Duration) {
    let minutes = duration.as_secs() / 60;
    if minutes > 0 {
        QuestTracker::update_time_progress(
            &mut state.progress.profile.daily_quests,
            u32::try_from(minutes).unwrap_or(u32::MAX),
        );
    }
}

/// Result of [`award_quest_completion_xp`]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct QuestXpAward {
    /// (description, xp) pairs for each quest newly completed by this call
    pub bonuses: Vec<QuestBonus>,
    /// Whether the XP granted here alone crossed an account level-up threshold.
    /// Callers that also call `profile.add_xp` for other XP in the same operation
    /// (e.g. scenario completion) must OR this into their own level-up check, since
    /// each `add_xp` call only reports whether *that* call crossed a level boundary.
    pub leveled_up: bool,
}

impl QuestXpAward {
    /// Fold the outcome of a later award into this one.
    pub(crate) fn absorb(&mut self, other: Self) {
        self.bonuses.extend(other.bonuses);
        self.leveled_up |= other.leveled_up;
    }
}

/// Check and award XP for newly completed quests
///
/// Marks each newly completed quest on the profile's `completed_quests_today` set (which
/// gates next-day streak increments), grants a streak freeze once every quest generated
/// for today is completed, and returns the (description, xp) pairs for each quest newly
/// completed by this call, for the caller to display or total up. This is the only place
/// quest-completion XP is applied to `profile.xp` - callers must not re-add it.
pub(crate) fn award_quest_completion_xp(
    state: &mut AppState,
    was_completed: &[bool],
) -> QuestXpAward {
    let newly_completed: Vec<(QuestId, String, u32)> = {
        let profile = &state.progress.profile;
        profile
            .daily_quests
            .iter()
            .enumerate()
            .filter_map(|(idx, quest)| {
                if idx < was_completed.len() && !was_completed[idx] && quest.completed {
                    Some((
                        quest.id.clone(),
                        text::quest_description(quest),
                        quest.xp_reward,
                    ))
                } else {
                    None
                }
            })
            .collect()
    };

    if newly_completed.is_empty() {
        return QuestXpAward::default();
    }

    let total_bonus_xp: u64 = newly_completed
        .iter()
        .map(|(_, _, xp)| u64::from(*xp))
        .sum();
    let profile = &mut state.progress.profile;
    let leveled_up = profile.add_xp(total_bonus_xp);
    for (quest_id, _, _) in &newly_completed {
        profile.complete_quest(quest_id.clone());
    }

    // Show notifications for each completed quest
    let bonuses: Vec<QuestBonus> = newly_completed
        .iter()
        .map(|(_, description, xp_reward)| QuestBonus {
            quest: description.clone(),
            xp: u64::from(*xp_reward),
        })
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
pub(crate) fn snapshot_quest_completion(state: &AppState) -> Vec<bool> {
    let profile = &state.progress.profile;
    profile.daily_quests.iter().map(|q| q.completed).collect()
}

/// Get current progress value from quest type
#[expect(
    clippy::match_same_arms,
    reason = "arms stay apart to document each case"
)]
fn get_quest_current_progress(quest_type: &QuestType) -> u32 {
    match quest_type {
        QuestType::CommandPractice { current, .. } => *current,
        QuestType::ScenarioCompletion { current, .. } => *current,
        QuestType::TimeInvested {
            current_minutes, ..
        } => *current_minutes,
        QuestType::Exploration { commands_used, .. } => {
            u32::try_from(commands_used.len()).unwrap_or(u32::MAX)
        }
        QuestType::SpeedRun { .. } => 0, // Single-attempt quest
    }
}

/// What one quest-progress update changed
#[derive(Debug)]
pub(crate) struct QuestUpdate {
    /// Quests whose progress moved, for the results display
    pub changes: Vec<QuestProgressChange>,
    /// XP granted for quests completed by this update
    pub award: QuestXpAward,
}

/// Handle `UpdateQuestProgress` message
///
/// Updates quest progress after a quest event; the per-update outcome is only needed at
/// scenario completion, which calls [`apply_quest_event`] directly.
pub(crate) fn handle_update_quest_progress(state: &mut AppState, event: QuestEvent) {
    apply_quest_event(state, event);
}

/// Update quest progress for one event and return what changed so callers can hand it
/// on by value.
pub(crate) fn apply_quest_event(state: &mut AppState, event: QuestEvent) -> QuestUpdate {
    let mut changes = Vec::new();

    // Snapshot progress BEFORE updates
    let progress_before: HashMap<QuestId, u32> = {
        let profile = &state.progress.profile;
        profile
            .daily_quests
            .iter()
            .map(|q| (q.id.clone(), get_quest_current_progress(&q.quest_type)))
            .collect()
    };

    // Track which quests were already completed before this update
    let was_completed = snapshot_quest_completion(state);

    match event {
        QuestEvent::CommandExecuted(command) => track_command_for_quests(state, &command),
        QuestEvent::ScenarioCompleted { id, duration } => {
            track_scenario_completion_for_quests(state, &id, duration);
            track_time_for_quests(state, duration);
        }
    }

    // Detect progress changes AFTER updates
    {
        let profile = &state.progress.profile;
        for quest in &profile.daily_quests {
            let old = progress_before.get(&quest.id).copied().unwrap_or(0);
            let new = get_quest_current_progress(&quest.quest_type);

            if new > old {
                changes.push(QuestProgressChange {
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

    QuestUpdate { changes, award }
}

/// Format quest type as readable description
pub(crate) fn format_quest_description(quest_type: &QuestType) -> String {
    match quest_type {
        QuestType::CommandPractice {
            command, target, ..
        } => format!("Use '{command}' {target} times"),
        QuestType::ScenarioCompletion { target, .. } => format!("Complete {target} scenarios"),
        QuestType::SpeedRun { scenario_id, .. } => format!("Speed run: {scenario_id}"),
        QuestType::TimeInvested { target_minutes, .. } => format!("Practice {target_minutes} min"),
        QuestType::Exploration {
            target_commands, ..
        } => format!("Try {target_commands} commands"),
    }
}
