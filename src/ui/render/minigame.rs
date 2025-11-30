//! Mini-game screen rendering (Arcade Mode)

use super::editor::render_editor_pair;
use crate::game::PlayableScenario;
use crate::ui::state::{AppState, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
};

/// Render the mini-game screen
pub(super) fn render_minigame(frame: &mut Frame, state: &AppState) {
    // Extract MiniGameData from TypedScreen::MiniGame
    let TypedScreen::MiniGame(_) = &state.screen else {
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
        render_playing(frame, area, session);
    } else if session.state().is_transition() {
        render_transition(frame, area, session);
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
    let title = Paragraph::new("ARCADE MODE")
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
fn render_playing(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
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

    // Title + controls
    let title_line = Line::from(vec![
        Span::styled(
            "ARCADE MODE",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("        "),
        Span::styled("[Esc] Pause", Style::default().fg(Color::Gray)),
    ]);
    let title = Paragraph::new(title_line)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Next scenarios queue
    render_queue(frame, chunks[1], session);

    // Editor content (current scenario)
    if let Some(current) = session.current_scenario() {
        render_scenario_editor(frame, chunks[2], current, &current.scenario.description);
    }

    // Timer bar
    if let Some(current) = session.current_scenario() {
        render_timer_bar(frame, chunks[3], current);
    }

    // Stats bar
    render_stats_bar(frame, chunks[4], session);
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
) {
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
    render_editor_pair(
        frame,
        chunks[1],
        scenario.current_state(),
        scenario.target_state(),
        " Current ",
        " Target ",
    );
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

/// Render stats bar (score, lives, streak, multiplier)
fn render_stats_bar(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    let stats = session.stats();

    // Lives as hearts
    let lives_str: String = (0..5)
        .map(|i| if i < stats.lives { '♥' } else { '♡' })
        .collect();

    let stats_line = Line::from(vec![
        Span::styled("SCORE: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:>6}", stats.score),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled("LIVES: ", Style::default().fg(Color::Gray)),
        Span::styled(lives_str, Style::default().fg(Color::Red)),
        Span::raw("    "),
        Span::styled("MULT: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("x{:.1}", stats.multiplier),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let stats_line2 = Line::from(vec![
        Span::styled("STREAK: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.streak),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("       "),
        Span::styled("LEVEL: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.level),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw("      "),
        Span::styled("BEST: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", stats.best_streak),
            Style::default().fg(Color::Green),
        ),
    ]);

    let paragraph = Paragraph::new(vec![stats_line, stats_line2])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

/// Render transition screen (shows editor with popup overlay)
fn render_transition(frame: &mut Frame, area: Rect, session: &crate::minigame::MiniGameSession) {
    // Render playing screen as background (shows final editor state)
    render_playing(frame, area, session);

    // Render popup overlay on top using shared function
    let is_success = session.state().transition_success().unwrap_or(true);
    let (title, message, color) = if is_success {
        ("SUCCESS!", "Loading next scenario...", Color::Green)
    } else {
        ("TIME'S UP!", "Loading next scenario...", Color::Red)
    };

    super::popups::render_result_popup(frame, title, message, color);
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
        ]),
        Line::from(vec![
            Span::styled("Scenarios Failed: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.scenarios_failed),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Best Streak: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}", stats.best_streak),
                Style::default()
                    .fg(Color::Cyan)
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
    use crate::config::{
        Difficulty, Scenario, ScenarioMetadata, ScoringConfig, Setup, Solution, TargetState,
    };
    use crate::gamification::{ProfileStorage, UserProfile};
    use crate::learning::PerformanceTracker;
    use crate::minigame::MiniGameSession;
    use crate::ui::state::{MiniGameData, TypedScreen};
    use ratatui::{Terminal, backend::TestBackend};
    use std::sync::Arc;

    fn create_test_scenario(id: &str) -> Scenario {
        Scenario {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "Test scenario".to_string(),
            setup: Setup {
                file_content: "line 1\nline 2\n".to_string(),
                cursor_position: (1, 0),
            },
            target: TargetState {
                file_content: "line 1\n".to_string(),
                cursor_position: (1, 0),
                selection: None,
            },
            solution: Solution {
                commands: vec!["dd".to_string()],
                description: "Delete line".to_string(),
            },
            alternatives: vec![],
            hints: vec![],
            scoring: ScoringConfig {
                optimal_count: 1,
                max_points: 100,
                tolerance: 0,
            },
            metadata: Some(ScenarioMetadata {
                difficulty: Some(Difficulty::Beginner),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn test_render_minigame_no_session() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::new(),
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
        let mut session = MiniGameSession::new(scenarios);
        session.start();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::new(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::MiniGame(MiniGameData::default());
        state.game.minigame_session = Some(session);

        let result = terminal.draw(|f| render_minigame(f, &state));
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_minigame_wrong_screen() {
        use crate::ui::state::MenuData;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut state = AppState::new(
            vec![],
            UserProfile::new(),
            ProfileStorage::new(),
            PerformanceTracker::new(),
        );
        state.screen = TypedScreen::Menu(MenuData::default());

        // Should not panic when called with wrong screen
        let result = terminal.draw(|f| render_minigame(f, &state));
        assert!(result.is_ok());
    }
}
