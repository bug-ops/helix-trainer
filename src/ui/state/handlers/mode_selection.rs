//! Message handlers for mode selection screen
//!
//! Type-safe handlers that receive `ModeSelectionData` directly instead of
//! performing runtime checks on `AppState`.

use crate::minigame::MiniGameSession;
use crate::ui::notification::{Notification, NotificationType};
use crate::ui::state::Effect;
use crate::ui::state::{
    HandlerContext, HandlerOutcome, MenuData, MiniGameData, MiniGameModeSelection,
    ModeSelectionData, TopMode, TypedScreen,
};
use rust_i18n::t;
use std::sync::Arc;

/// Handle mode selection up navigation
///
/// Type-safe handler that only accepts `ModeSelectionData`, ensuring compile-time
/// guarantee that it's called on the correct screen.
pub(in crate::ui::state) fn handle_mode_selection_up(
    data: &mut ModeSelectionData,
) -> HandlerOutcome {
    // Check if mini-game mode selection is active
    if let Some(ref mut selection) = data.minigame_mode_selection {
        selection.select_previous();
    } else {
        data.selected_mode = data.selected_mode.previous();
    }
    HandlerOutcome::Stay
}

/// Handle mode selection down navigation
///
/// Type-safe handler that only accepts `ModeSelectionData`.
pub(in crate::ui::state) fn handle_mode_selection_down(
    data: &mut ModeSelectionData,
) -> HandlerOutcome {
    // Check if mini-game mode selection is active
    if let Some(ref mut selection) = data.minigame_mode_selection {
        selection.select_next();
    } else {
        data.selected_mode = data.selected_mode.next();
    }
    HandlerOutcome::Stay
}

/// Handle mode selection confirmation
///
/// Type-safe handler that transitions to the selected mode screen.
/// Uses `HandlerContext` to delegate to other handlers.
pub(in crate::ui::state) fn handle_mode_selection_select(
    data: &mut ModeSelectionData,
    ctx: &mut HandlerContext<'_>,
) -> HandlerOutcome {
    // Check if mini-game mode selection is active
    if let Some(ref selection) = data.minigame_mode_selection {
        // Launch the selected mini-game mode
        let mode = selection.selected_mode(ctx.progress.today());
        return handle_launch_minigame_mode(ctx, mode);
    }

    match data.selected_mode {
        TopMode::Training => handle_select_training_mode(ctx),
        TopMode::Arcade => {
            // Show mini-game mode selection instead of immediately starting arcade
            data.minigame_mode_selection = Some(MiniGameModeSelection::new());
            HandlerOutcome::Stay
        }
    }
}

/// Handle escape/back navigation
///
/// If mini-game mode selection is open, close it. Otherwise, do nothing.
pub(in crate::ui::state) fn handle_mode_selection_back(
    data: &mut ModeSelectionData,
) -> HandlerOutcome {
    if data.minigame_mode_selection.is_some() {
        data.minigame_mode_selection = None;
    }
    HandlerOutcome::Stay
}

/// Handle selecting Training Mode
///
/// Transitions to the main menu screen.
pub(in crate::ui::state) fn handle_select_training_mode(
    _ctx: &mut HandlerContext<'_>,
) -> HandlerOutcome {
    HandlerOutcome::Transition(Box::new(TypedScreen::Menu(MenuData::default())))
}

/// Handle selecting Arcade Mode
///
/// Transitions to the mini-game screen with FSRS-weighted scenario selection, or stays with a
/// notification when no scenario matches the active filters.
pub(in crate::ui::state) fn handle_select_arcade_mode(
    ctx: &mut HandlerContext<'_>,
) -> HandlerOutcome {
    let Some(session) =
        super::minigame::create_minigame_session(ctx.game, Some(&ctx.progress.performance_tracker))
    else {
        notify_no_scenarios(ctx.ui);
        return HandlerOutcome::Stay;
    };
    ctx.ui.show_key_history = false;

    HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(MiniGameData::new(session))))
}

fn notify_no_scenarios(ui: &mut crate::ui::state::UIState) {
    ui.notifications
        .push(Notification::new(NotificationType::Info {
            message: t!("minigame.no_scenarios_available").to_string(),
        }));
}

/// Handle launching a specific mini-game mode
///
/// Creates a session with the selected mode and transitions to the mini-game screen.
pub(in crate::ui::state) fn handle_launch_minigame_mode(
    ctx: &mut HandlerContext<'_>,
    mode: crate::minigame::MiniGameMode,
) -> HandlerOutcome {
    use crate::config::Scenario;

    let today = ctx.progress.today();
    if mode.is_challenge() && !ctx.progress.profile.challenge_progress.can_attempt(today) {
        ctx.ui
            .notifications
            .push(Notification::new(NotificationType::Info {
                message: t!("minigame.challenge_no_attempts_left").to_string(),
            }));
        return HandlerOutcome::Stay;
    }

    // Collect scenarios
    let scenarios: Vec<Scenario> = ctx
        .game
        .scenario_collection
        .filtered()
        .into_iter()
        .cloned()
        .collect();

    if scenarios.is_empty() {
        notify_no_scenarios(ctx.ui);
        return HandlerOutcome::Stay;
    }

    // Only consume an attempt once the launch is actually going to succeed -
    // an empty scenario pool above must not burn one of today's 3 attempts.
    if mode.is_challenge() {
        ctx.progress.profile.challenge_progress.start_attempt(today);
        // Save immediately (not debounced): a crash/quit within the debounce
        // window would otherwise refund the attempt just consumed in memory.
        ctx.ui.effects.push(Effect::SaveProfile);
    }

    // Clone tracker for session's internal weighted selection
    let tracker_clone = Some(ctx.progress.performance_tracker.clone());

    // Create session with the selected mode
    let mut session = MiniGameSession::with_mode(Arc::new(scenarios), tracker_clone, mode);
    session.start(); // Begin countdown
    ctx.ui.show_key_history = false;

    HandlerOutcome::Transition(Box::new(TypedScreen::MiniGame(MiniGameData::new(session))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Difficulty;
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{ConfigState, GameState, MiniGameModeKind, ProgressState, UIState};
    use std::assert_matches;

    fn create_test_scenario(id: &str) -> crate::config::Scenario {
        ScenarioBuilder::new()
            .id(id)
            .setup_content("line 1\nline 2\n")
            .setup_cursor(1, 0)
            .target_content("line 1\n")
            .target_cursor(1, 0)
            .optimal_count(1)
            .difficulty(Difficulty::Beginner)
            .build()
    }

    fn create_test_context() -> (UIState, GameState, ProgressState, ConfigState) {
        (
            UIState::new(),
            GameState::new(vec![]),
            ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            ConfigState::default(),
        )
    }

    fn create_test_context_with_scenarios() -> (UIState, GameState, ProgressState, ConfigState) {
        let scenarios = vec![
            create_test_scenario("s1"),
            create_test_scenario("s2"),
            create_test_scenario("s3"),
        ];
        (
            UIState::new(),
            GameState::new(scenarios),
            ProgressState::new(
                UserProfile::new(),
                PerformanceTracker::new(),
                ProfileStorage::for_test(),
            ),
            ConfigState::default(),
        )
    }

    #[test]
    fn test_mode_selection_up() {
        let mut data = ModeSelectionData {
            selected_mode: crate::ui::state::TopMode::Arcade,
            minigame_mode_selection: None,
        };

        let outcome = handle_mode_selection_up(&mut data);

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, crate::ui::state::TopMode::Training); // Should move to Training
    }

    #[test]
    fn test_mode_selection_up_at_top() {
        let mut data = ModeSelectionData::default(); // Already at top (0)

        let outcome = handle_mode_selection_up(&mut data);

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, crate::ui::state::TopMode::Training); // Should stay at 0
    }

    #[test]
    fn test_mode_selection_down() {
        let mut data = ModeSelectionData::default(); // Start at 0 (Training)

        let outcome = handle_mode_selection_down(&mut data);

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, crate::ui::state::TopMode::Arcade); // Should move to Arcade
    }

    #[test]
    fn test_mode_selection_down_at_bottom() {
        let mut data = ModeSelectionData {
            selected_mode: crate::ui::state::TopMode::Arcade,
            minigame_mode_selection: None,
        };

        let outcome = handle_mode_selection_down(&mut data);

        assert!(outcome.is_stay());
        assert_eq!(data.selected_mode, crate::ui::state::TopMode::Arcade); // Should stay at 1
    }

    #[test]
    fn test_select_training_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_select_training_mode(&mut ctx);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::Menu(_));
        }
    }

    #[test]
    fn test_select_arcade_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_select_arcade_mode(&mut ctx);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::MiniGame(_));
        }
    }

    #[test]
    fn test_select_arcade_mode_without_scenarios_stays_and_notifies() {
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_select_arcade_mode(&mut ctx);

        assert!(outcome.is_stay());
        assert_eq!(
            ctx.ui.notifications.visible().next().unwrap().message(),
            rust_i18n::t!("minigame.no_scenarios_available").to_string()
        );
    }

    #[test]
    fn test_select_arcade_mode_hides_key_history() {
        // Regression test for S1: the handler must reset the flag itself rather than
        // relying on a caller to do it.
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        ui.show_key_history = true;
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        handle_select_arcade_mode(&mut ctx);

        assert!(!ctx.ui.show_key_history);
    }

    #[test]
    fn test_launch_minigame_mode_hides_key_history() {
        // Regression test for S1 on the second live dispatch path.
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        ui.show_key_history = true;
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::default();
        handle_launch_minigame_mode(&mut ctx, mode);

        assert!(!ctx.ui.show_key_history);
    }

    #[test]
    fn test_mode_selection_select_training() {
        let mut data = ModeSelectionData::default(); // Default is 0 (Training)
        let (mut ui, mut game, mut progress, config) = create_test_context();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&mut data, &mut ctx);

        assert!(outcome.is_transition());
        if let HandlerOutcome::Transition(screen) = outcome {
            assert_matches!(*screen, TypedScreen::Menu(_));
        }
    }

    #[test]
    fn test_mode_selection_select_arcade_opens_mode_menu() {
        let mut data = ModeSelectionData {
            selected_mode: crate::ui::state::TopMode::Arcade,
            minigame_mode_selection: None,
        };
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let outcome = handle_mode_selection_select(&mut data, &mut ctx);

        // Should stay on screen but open mode selection
        assert!(outcome.is_stay());
        assert!(data.minigame_mode_selection.is_some());
    }

    #[test]
    fn test_minigame_mode_selection_navigation() {
        let mut data = ModeSelectionData {
            selected_mode: crate::ui::state::TopMode::Arcade,
            minigame_mode_selection: Some(MiniGameModeSelection::new()),
        };

        // Start at 0 (Arcade)
        assert_eq!(
            data.minigame_mode_selection.as_ref().unwrap().selected,
            MiniGameModeKind::Arcade
        );

        // Move down to Survival
        handle_mode_selection_down(&mut data);
        assert_eq!(
            data.minigame_mode_selection.as_ref().unwrap().selected,
            MiniGameModeKind::Survival
        );

        // Move down to Challenge
        handle_mode_selection_down(&mut data);
        assert_eq!(
            data.minigame_mode_selection.as_ref().unwrap().selected,
            MiniGameModeKind::Challenge
        );

        // Wrap to Arcade
        handle_mode_selection_down(&mut data);
        assert_eq!(
            data.minigame_mode_selection.as_ref().unwrap().selected,
            MiniGameModeKind::Arcade
        );

        // Move up to Challenge (wrap)
        handle_mode_selection_up(&mut data);
        assert_eq!(
            data.minigame_mode_selection.as_ref().unwrap().selected,
            MiniGameModeKind::Challenge
        );
    }

    #[test]
    fn test_minigame_mode_selection_back_closes_menu() {
        let mut data = ModeSelectionData {
            selected_mode: crate::ui::state::TopMode::Arcade,
            minigame_mode_selection: Some(MiniGameModeSelection::new()),
        };

        handle_mode_selection_back(&mut data);

        assert!(data.minigame_mode_selection.is_none());
    }

    #[test]
    fn test_launch_survival_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode =
            crate::minigame::MiniGameMode::Survival(crate::minigame::SurvivalConfig::default());
        let outcome = handle_launch_minigame_mode(&mut ctx, mode);

        let HandlerOutcome::Transition(screen) = outcome else {
            panic!("expected a transition to the mini-game screen");
        };
        let TypedScreen::MiniGame(data) = *screen else {
            panic!("expected the mini-game screen");
        };
        assert!(data.session.mode().is_survival());
        assert_eq!(data.session.stats().lives(), 1);
    }

    #[test]
    fn test_launch_challenge_mode() {
        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::Challenge(
            crate::minigame::ChallengeConfig::for_date(chrono::Utc::now().date_naive()),
        );
        let outcome = handle_launch_minigame_mode(&mut ctx, mode);

        let HandlerOutcome::Transition(screen) = outcome else {
            panic!("expected a transition to the mini-game screen");
        };
        let TypedScreen::MiniGame(data) = *screen else {
            panic!("expected the mini-game screen");
        };
        assert_eq!(data.session.stats().lives(), 3);
    }

    #[test]
    fn test_launch_challenge_mode_enforces_daily_attempt_cap() {
        use crate::constants::CHALLENGE_MAX_ATTEMPTS;

        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let today = ctx.progress.today();
        let mode = crate::minigame::MiniGameMode::Challenge(
            crate::minigame::ChallengeConfig::for_date(today),
        );

        for _ in 0..CHALLENGE_MAX_ATTEMPTS {
            let outcome = handle_launch_minigame_mode(&mut ctx, mode.clone());
            assert!(outcome.is_transition());
        }

        // One more attempt beyond the cap must be rejected.
        let outcome = handle_launch_minigame_mode(&mut ctx, mode);
        assert!(outcome.is_stay());
        assert_eq!(
            ctx.progress.profile.challenge_progress.attempts_used_today,
            CHALLENGE_MAX_ATTEMPTS
        );
        assert_eq!(
            ctx.progress
                .profile
                .challenge_progress
                .total_challenges_attempted,
            u32::from(CHALLENGE_MAX_ATTEMPTS)
        );

        // The player must be told why the launch was refused, with the actual
        // localized copy, not just a matching notification variant.
        assert_eq!(ctx.ui.notifications.count(), 1);
        assert_eq!(
            ctx.ui.notifications.visible().next().unwrap().message(),
            rust_i18n::t!("minigame.challenge_no_attempts_left").to_string()
        );
        assert_matches!(
            ctx.ui
                .notifications
                .visible()
                .next()
                .unwrap()
                .notification_type,
            crate::ui::notification::NotificationType::Info { .. }
        );
    }

    /// Regression guard: exhausting Daily Challenge attempts must not affect
    /// other mini-game modes, which have no attempt cap.
    #[test]
    fn test_launch_survival_mode_unaffected_by_exhausted_challenge_attempts() {
        use crate::constants::CHALLENGE_MAX_ATTEMPTS;

        let (mut ui, mut game, mut progress, config) = create_test_context_with_scenarios();
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let today = ctx.progress.today();
        let challenge_mode = crate::minigame::MiniGameMode::Challenge(
            crate::minigame::ChallengeConfig::for_date(today),
        );
        for _ in 0..CHALLENGE_MAX_ATTEMPTS {
            handle_launch_minigame_mode(&mut ctx, challenge_mode.clone());
        }
        assert!(!ctx.progress.profile.challenge_progress.can_attempt(today));

        let survival_mode =
            crate::minigame::MiniGameMode::Survival(crate::minigame::SurvivalConfig::default());
        let outcome = handle_launch_minigame_mode(&mut ctx, survival_mode);

        assert!(outcome.is_transition());
    }

    /// Regression test for critique S1: an attempt must not be consumed when
    /// the launch cannot actually start (empty scenario pool) - and, since
    /// consuming an attempt is what triggers the immediate save, no save must
    /// happen either.
    #[test]
    fn test_launch_challenge_mode_with_no_scenarios_does_not_consume_attempt() {
        use crate::gamification::ProfileStorage;
        use tempfile::TempDir;

        let (mut ui, mut game, mut progress, config) = create_test_context(); // No scenarios
        let temp_dir = TempDir::new().unwrap();
        let profile_path = temp_dir.path().join("profile.json");
        progress.storage = ProfileStorage::with_path(&profile_path);
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::Challenge(
            crate::minigame::ChallengeConfig::for_date(ctx.progress.today()),
        );
        let outcome = handle_launch_minigame_mode(&mut ctx, mode);

        assert!(outcome.is_stay());
        assert!(
            !profile_path.exists(),
            "no save should be triggered when the launch never consumes an attempt"
        );
        assert_eq!(
            ctx.progress.profile.challenge_progress.attempts_used_today,
            0
        );
        assert_eq!(
            ctx.progress
                .profile
                .challenge_progress
                .total_challenges_attempted,
            0
        );
    }

    /// Regression test: the attempt cap resets on a new calendar day, exercised
    /// through the handler (not just at the `ChallengeProgress` type level) via
    /// the injected `Clock`.
    #[test]
    fn test_launch_challenge_mode_resets_on_new_day_via_handler() {
        use crate::constants::CHALLENGE_MAX_ATTEMPTS;
        use crate::gamification::{ProfileStorage, UserProfile};
        use crate::learning::PerformanceTracker;
        use crate::time::FakeClock;
        use std::sync::Arc;

        let scenarios = vec![create_test_scenario("s1")];
        let mut ui = UIState::new();
        let mut game = GameState::new(scenarios);
        let config = ConfigState::default();

        let day1_clock = Arc::new(FakeClock::at("2026-01-15T12:00:00Z"));
        let mut progress = ProgressState::with_clocks(
            UserProfile::new(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
            day1_clock,
            Arc::new(crate::time::SystemMonotonicClock),
        );

        {
            let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);
            let today = ctx.progress.today();
            let mode = crate::minigame::MiniGameMode::Challenge(
                crate::minigame::ChallengeConfig::for_date(today),
            );
            for _ in 0..CHALLENGE_MAX_ATTEMPTS {
                handle_launch_minigame_mode(&mut ctx, mode.clone());
            }
            let outcome = handle_launch_minigame_mode(&mut ctx, mode);
            assert!(outcome.is_stay());
        }

        // Advance to the next calendar day and rebuild the context with the
        // same profile - the cap must have reset.
        let day2_clock = Arc::new(FakeClock::at("2026-01-16T12:00:00Z"));
        progress = ProgressState::with_clocks(
            progress.profile.clone(),
            PerformanceTracker::new(),
            ProfileStorage::for_test(),
            day2_clock,
            Arc::new(crate::time::SystemMonotonicClock),
        );
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);
        let today = ctx.progress.today();
        let mode = crate::minigame::MiniGameMode::Challenge(
            crate::minigame::ChallengeConfig::for_date(today),
        );
        let outcome = handle_launch_minigame_mode(&mut ctx, mode);
        assert!(outcome.is_transition());
        assert_eq!(
            ctx.progress.profile.challenge_progress.attempts_used_today,
            1
        );
    }

    #[test]
    fn test_launch_no_scenarios() {
        let (mut ui, mut game, mut progress, config) = create_test_context(); // No scenarios
        let mut ctx = HandlerContext::new(&mut ui, &mut game, &mut progress, &config);

        let mode = crate::minigame::MiniGameMode::default();
        let outcome = handle_launch_minigame_mode(&mut ctx, mode);

        // Should stay on screen and explain why when no scenarios are available
        assert!(outcome.is_stay());
        assert_eq!(ctx.ui.notifications.count(), 1);
    }
}
