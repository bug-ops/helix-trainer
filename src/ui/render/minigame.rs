//! Mini-game screen rendering (Arcade Mode)

use super::editor::render_editor_pair;
use crate::constants::difficulty::{
    LEVEL_ADVANCED_MAX, LEVEL_ADVANCED_MIN, LEVEL_BEGINNER_MAX, LEVEL_BEGINNER_MIN,
    LEVEL_INTERMEDIATE_MAX, LEVEL_INTERMEDIATE_MIN,
};
use crate::game::PlayableScenario;
use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

/// Combo count threshold for highlighted display (bold + magenta)
const COMBO_HIGHLIGHT_THRESHOLD: u32 = 5;

/// Return the header title string for the current arcade sub-mode
fn mode_title(session: &crate::minigame::MiniGameSession) -> &'static str {
    use crate::minigame::MiniGameMode;
    match session.mode() {
        MiniGameMode::Survival(_) => "SURVIVAL MODE",
        MiniGameMode::Challenge(_) => "DAILY CHALLENGE",
        MiniGameMode::Arcade(_) => "ARCADE MODE",
    }
}

/// Render the mini-game screen
pub(super) fn render_minigame(frame: &mut Frame, state: &AppState) {
    // Extract MiniGameData from TypedScreen::MiniGame
    let TypedScreen::MiniGame(minigame_data) = &state.screen else {
        return; // Wrong screen type
    };

    // Get session reference
    let Some(ref session) = state.game.minigame_session else {
        render_no_session(frame);
        return;
    };

    let area = frame.area();

    // Render based on game state
    if session.state().is_countdown() {
        render_countdown(frame, area, session);
    } else if session.state().is_playing() {
        let target_area = render_playing(frame, area, session, &minigame_data.input_state);
        // Show key history popup after first keypress (reset on scenario transitions),
        // bounded to the Target panel's own inner area so it can never draw
        // over that panel's border.
        if let Some(target_area) = target_area
            && state.ui.show_key_history
        {
            super::popups::render_key_history_popup(
                frame,
                minigame_data.key_history.keys(),
                target_area,
            );
        }
    } else if session.state().is_transition() {
        render_transition(
            frame,
            area,
            session,
            minigame_data.last_xp_earned,
            &minigame_data.input_state,
        );
    } else if session.state().is_paused() {
        render_paused(frame, area, session);
    } else if session.state().is_game_over() {
        render_game_over(frame, area, session);
    }
}

/// Render "no session" error
fn render_no_session(frame: &mut Frame) {
    let area = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title("ERROR")
        .border_style(Style::default().fg(Color::Red));

    let text = Paragraph::new("No mini-game session active. Press Esc to return.")
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center)
        .block(block);

    frame.render_widget(text, area);
}

/// Render countdown screen (3, 2, 1, GO!)
fn render_countdown(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(10),   // Countdown number
            Constraint::Length(3), // Stats
        ])
        .split(area);

    // Title
    let title = Paragraph::new(mode_title(session))
        .style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Countdown number
    let remaining = session.state().countdown_remaining().unwrap_or(0);
    let countdown_text = if remaining > 0 {
        remaining.to_string()
    } else {
        "GO!".to_string()
    };

    let countdown = Paragraph::new(countdown_text)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);
    frame.render_widget(countdown, chunks[1]);

    // Stats bar
    render_stats_bar(frame, chunks[2], session);
}

/// Render playing screen with active scenario
///
/// Returns the Target editor panel's outer `Rect` (when a scenario is
/// current), so the caller can bound the key-history popup to stay inside it.
fn render_playing(
    frame: &mut Frame,
    area: Rect,
    session: &crate::minigame::MiniGameSession,
    input_state: &crate::input::typestate::InputStateMachine,
) -> Option<Rect> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title + controls
            Constraint::Length(3), // Next scenarios queue
            Constraint::Min(8),    // Editor content
            Constraint::Length(3), // Timer bar
            Constraint::Length(3), // Stats bar
        ])
        .split(area);

    // Title + controls, with live feedback for an in-progress '"'-register
    // selection or ':'-command-line buffer (both live in Arcade mode too -
    // see the Esc-cancel and reset_input_state wiring in input/handlers.rs).
    let pending_text = super::helpers::pending_input_indicator(input_state);
    let mut title_spans = vec![
        Span::styled(
            mode_title(session),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("        "),
        Span::styled("[Esc] Pause", Style::default().fg(Color::Gray)),
    ];
    if !pending_text.is_empty() {
        title_spans.push(Span::raw("  "));
        title_spans.push(Span::styled(
            pending_text,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if session.is_recording_macro() {
        title_spans.push(Span::raw("  "));
        title_spans.push(Span::styled(
            "[q: REC]",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    let title = Paragraph::new(Line::from(title_spans))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Next scenarios queue
    render_queue(frame, chunks[1], session);

    // Editor content (current scenario)
    let target_area = session.current_scenario().map(|current| {
        render_scenario_editor(frame, chunks[2], current, &current.scenario.description)
    });

    // Timer bar
    if let Some(current) = session.current_scenario() {
        render_timer_bar(frame, chunks[3], current);
    }

    // Stats bar
    render_stats_bar(frame, chunks[4], session);

    target_area
}

/// Render scenario queue (next 3 scenarios)
fn render_queue(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    let queue = session.queue();
    let queue_text = if queue.is_empty() {
        "NEXT: [No more scenarios]".to_string()
    } else {
        let names: Vec<String> = queue
            .iter()
            .take(3)
            .map(|s| format!("[{}]", s.name))
            .collect();
        format!("NEXT: {}", names.join(" "))
    };

    let paragraph = Paragraph::new(queue_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

/// Render scenario editor view - uses shared editor rendering
///
/// Uses PlayableScenario trait to access current and target states,
/// enabling code reuse with training mode.
fn render_scenario_editor<S: PlayableScenario>(
    frame: &mut Frame,
    area: Rect,
    scenario: &S,
    description: &str,
) -> Rect {
    // Layout: task description | editor views (current + target)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Task description
            Constraint::Min(5),    // Editor views
        ])
        .split(area);

    // Task description
    let task = Paragraph::new(format!("TASK: {}", description))
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(task, chunks[0]);

    // Editor views - reuse common rendering function
    // No preview highlight in arcade mode
    render_editor_pair(frame, chunks[1], scenario, " Current ", " Target ", None)
}

/// Render timer bar
fn render_timer_bar(frame: &mut Frame, area: Rect, scenario: &crate::minigame::ActiveMiniScenario) {
    let remaining = scenario.remaining_time().as_secs_f64();
    // Clamp progress to [0.0, 1.0] - can exceed 1.0 when time expires
    let progress = scenario.progress_percent().clamp(0.0, 1.0);

    // Color based on time remaining
    let color = if progress < 0.5 {
        Color::Green
    } else if progress < 0.75 {
        Color::Yellow
    } else {
        Color::Red
    };

    let label = format!("TIME: {:.1}s", remaining);

    // ratio must be in [0.0, 1.0] for Gauge widget
    let ratio = (1.0 - progress).clamp(0.0, 1.0);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(color))
        .ratio(ratio)
        .label(label);

    frame.render_widget(gauge, area);
}

/// Render stats bar (score, lives, streak, multiplier, combo, grace, difficulty)
fn render_stats_bar(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    let stats = session.stats();
    let combo = session.combo_count();
    let grace = session.grace_remaining();
    let level = session.difficulty_level();

    // Difficulty tier name and color based on level
    let (tier_name, tier_color) = match level {
        LEVEL_BEGINNER_MIN..=LEVEL_BEGINNER_MAX => ("Beginner", Color::Green),
        LEVEL_INTERMEDIATE_MIN..=LEVEL_INTERMEDIATE_MAX => ("Intermediate", Color::Yellow),
        LEVEL_ADVANCED_MIN..=LEVEL_ADVANCED_MAX => ("Advanced", Color::Red),
        _ => ("Unknown", Color::Gray),
    };

    // Lives as hearts
    let lives_str: String = (0..5)
        .map(|i| if i < stats.lives() { '♥' } else { '♡' })
        .collect();

    // Grace indicator (shield when available)
    // Using ASCII alternative for better terminal compatibility
    let grace_str = if grace > 0 { " [G]" } else { "" };

    // Multiplier color based on value
    let mult_color = match stats.multiplier() as u32 {
        0..=1 => Color::Gray,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Magenta,
        _ => Color::Cyan,
    };

    let stats_line = Line::from(vec![
        Span::styled("SCORE: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:>6}", stats.score),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("LIVES: ", Style::default().fg(Color::Gray)),
        Span::styled(lives_str, Style::default().fg(Color::Red)),
        Span::raw("  "),
        Span::styled("MULT: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("x{:.1}", stats.multiplier()),
            Style::default().fg(mult_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(grace_str, Style::default().fg(Color::Cyan)),
    ]);

    let stats_line2 = Line::from(vec![
        Span::styled("COMBO: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:>2}", combo),
            Style::default()
                .fg(if combo >= COMBO_HIGHLIGHT_THRESHOLD {
                    Color::Magenta
                } else {
                    Color::White
                })
                .add_modifier(if combo >= COMBO_HIGHLIGHT_THRESHOLD {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ),
        Span::raw("  "),
        Span::styled("STREAK: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:>2}", stats.streak()),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Lv.{} ", level),
            Style::default().fg(tier_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(tier_name, Style::default().fg(tier_color)),
        Span::raw("  "),
        Span::styled("BEST: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.best_streak()),
            Style::default().fg(Color::Green),
        ),
    ]);

    let paragraph = Paragraph::new(vec![stats_line, stats_line2])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

/// Render transition screen (shows editor with popup overlay)
fn render_transition(
    frame: &mut Frame,
    area: Rect,
    session: &crate::minigame::MiniGameSession,
    last_xp: Option<u64>,
    input_state: &crate::input::typestate::InputStateMachine,
) {
    // Render playing screen as background (shows final editor state)
    render_playing(frame, area, session, input_state);

    // Render popup overlay on top using shared function
    let is_success = session.state().transition_success().unwrap_or(true);
    let (title, message, color) = if is_success {
        let xp_msg = match last_xp {
            Some(xp) if xp > 0 => format!("+{} XP", xp),
            _ => "Loading next scenario...".to_string(),
        };
        ("SUCCESS!", xp_msg, Color::Green)
    } else {
        (
            "TIME'S UP!",
            "Loading next scenario...".to_string(),
            Color::Red,
        )
    };

    super::popups::render_result_popup(frame, title, &message, color);
}

/// Render paused screen
fn render_paused(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .split(area);

    // Paused message
    let paused_text = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "PAUSED",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(""),
    ];

    let paused = Paragraph::new(paused_text).alignment(Alignment::Center);
    frame.render_widget(paused, chunks[0]);

    // Controls
    let controls_text = vec![
        Line::from(Span::styled(
            "Esc - Resume",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "p - View Profile",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "s - View Statistics",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "q - Back to Menu",
            Style::default().fg(Color::Gray),
        )),
    ];

    let controls = Paragraph::new(controls_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(controls, chunks[1]);

    // Stats bar
    render_stats_bar(frame, chunks[2], session);
}

/// Render game over screen
fn render_game_over(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(8),    // Stats
            Constraint::Length(3), // Controls
        ])
        .split(area);

    // Game over title
    let title = Paragraph::new("GAME OVER")
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Final stats
    let stats = session.stats();
    let best_combo = session.best_combo();
    let stats_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Final Score: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.score),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Scenarios Completed: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.scenarios_completed),
                Style::default().fg(Color::Green),
            ),
            Span::raw("   "),
            Span::styled("Failed: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.scenarios_failed),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Best Streak: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.best_streak()),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled("Best Combo: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", best_combo),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    let stats_para = Paragraph::new(stats_text).alignment(Alignment::Center);
    frame.render_widget(stats_para, chunks[1]);

    // Controls
    let controls = Paragraph::new(vec![
        Line::from(Span::styled(
            "Esc/m - Back to Menu",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled("q - Quit", Style::default().fg(Color::Gray))),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(controls, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Difficulty, Scenario};
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::minigame::MiniGameSession;
    use crate::testing::ScenarioBuilder;
    use crate::ui::state::{MiniGameData, TypedScreen};
    use ratatui::{Terminal, backend::TestBackend};
    use std::sync::Arc;

    fn create_test_scenario(id: &str) -> Scenario {
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

    #[test]
    fn test_render_minigame_no_session() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::for_test(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::MiniGame(MiniGameData::default());
        // No session - should render error

        let result = terminal.draw(|f| render_minigame(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_minigame_countdown() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenarios = Arc::new(vec![create_test_scenario("s1")]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::for_test(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::MiniGame(MiniGameData::default());
        state.game.minigame_session = Some(session);

        let result = terminal.draw(|f| render_minigame(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_minigame_playing_shows_pending_command_line() {
        use ratatui::buffer::Buffer;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let scenarios = Arc::new(vec![create_test_scenario("s1")]);
        let mut session = MiniGameSession::new(scenarios, None);
        session.start();
        session.tick_countdown();
        session.tick_countdown();
        session.tick_countdown();
        assert!(session.state().is_playing());

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::for_test(),
            PerformanceTracker::new(),
        );
        let mut minigame_data = MiniGameData::default();
        minigame_data
            .input_state
            .process_key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char(':'),
                crossterm::event::KeyModifiers::NONE,
            ));
        assert_eq!(minigame_data.input_state.pending_command_line(), Some(""));
        state.screen = TypedScreen::MiniGame(minigame_data);
        state.game.minigame_session = Some(session);

        // Regression: must not panic when a command-line/register pending
        // state is live during Arcade rendering, and must actually render
        // the indicator text somewhere in the frame.
        let result = terminal.draw(|f| render_minigame(f, &state));
        assert!(result.is_ok());

        let buffer: &Buffer = terminal.backend().buffer();
        let rendered: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(
            rendered.contains(':'),
            "expected the pending ':' command-line buffer to be visible in the rendered frame"
        );
    }

    /// Arcade-mode analog of `task.rs`'s
    /// `test_render_key_history_popup_does_not_corrupt_target_panel_border`:
    /// a wide key history must not corrupt the Target editor panel's border
    /// in the Arcade playing screen either (regression test for #364,
    /// mirroring the #272 Arcade fix this fix must not regress).
    #[test]
    fn test_render_playing_key_history_popup_does_not_corrupt_target_panel_border() {
        use ratatui::buffer::Buffer;

        let render = |wide_history: bool| -> (Buffer, Rect) {
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            let scenarios = Arc::new(vec![create_test_scenario("s1")]);
            let mut session = MiniGameSession::new(scenarios, None);
            session.start();
            session.tick_countdown();
            session.tick_countdown();
            session.tick_countdown();
            assert!(session.state().is_playing());

            let mut minigame_data = MiniGameData::default();
            if wide_history {
                for key in ["regex-pattern", "Space", "⌫", "↵", "Esc"] {
                    minigame_data.key_history.push(key.to_string());
                }
            }

            let area = Rect::new(0, 0, 80, 24);
            let mut target_area = None;
            terminal
                .draw(|f| {
                    // Mirrors `render_minigame`'s `is_playing()` branch exactly:
                    // render the playing screen, then the key-history popup
                    // bounded to the returned Target panel `Rect`.
                    target_area = render_playing(f, area, &session, &minigame_data.input_state);
                    if let Some(target_area) = target_area {
                        super::super::popups::render_key_history_popup(
                            f,
                            minigame_data.key_history.keys(),
                            target_area,
                        );
                    }
                })
                .unwrap();
            (
                terminal.backend().buffer().clone(),
                target_area.expect("playing screen must report its Target panel Rect"),
            )
        };

        let (baseline, baseline_target_area) = render(false);
        let (with_popup, target_area) = render(true);
        assert_eq!(
            baseline_target_area, target_area,
            "Target panel layout must not depend on key history content"
        );

        assert_ne!(baseline, with_popup, "expected the popup to render");

        for x in target_area.x..target_area.x + target_area.width {
            assert_eq!(
                baseline[(x, target_area.y)],
                with_popup[(x, target_area.y)],
                "Target panel top border corrupted at column {x}"
            );
            let bottom_y = target_area.y + target_area.height - 1;
            assert_eq!(
                baseline[(x, bottom_y)],
                with_popup[(x, bottom_y)],
                "Target panel bottom border corrupted at column {x}"
            );
        }
        for y in target_area.y..target_area.y + target_area.height {
            assert_eq!(
                baseline[(target_area.x, y)],
                with_popup[(target_area.x, y)],
                "Target panel left border corrupted at row {y}"
            );
            let right_x = target_area.x + target_area.width - 1;
            assert_eq!(
                baseline[(right_x, y)],
                with_popup[(right_x, y)],
                "Target panel right border corrupted at row {y}"
            );
        }
    }

    #[test]
    fn test_render_minigame_wrong_screen() {
        use crate::ui::state::MenuData;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::for_test(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::Menu(MenuData::default());

        // Should not panic when called with wrong screen
        let result = terminal.draw(|f| render_minigame(f, &state));
        assert!(result.is_ok());
    }
}
