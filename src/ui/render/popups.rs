//! Popup rendering (hints, success, key history, notifications)

use super::helpers::{centered_popup, inner_rect, popup_block};
use crate::constants::{
    BIG_TEXT_CHAR_WIDTH_CELLS, BIG_TEXT_HEIGHT_LINES, HINT_POPUP_MAX_HEIGHT, HINT_POPUP_MAX_WIDTH,
    KEY_HISTORY_DISPLAY_SIZE, KEY_HISTORY_MIN_WIDTH, POPUP_BORDER_CELLS,
};
use crate::gamification::QuarantineOutcome;
use crate::ui::state::{AppState, ProfilePersistence, TypedScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use rust_i18n::t;
use std::collections::VecDeque;
use tui_big_text::{BigText, PixelSize};

/// Render a centered hint popup
pub(super) fn render_hint_popup(frame: &mut Frame, state: &AppState) {
    // Only render if we're on Task screen
    let TypedScreen::Task(task_data) = &state.screen else {
        return;
    };

    let area = frame.area();

    // Calculate popup dimensions with constraints
    let popup_width = HINT_POPUP_MAX_WIDTH.min(area.width.saturating_sub(4));
    let popup_height = HINT_POPUP_MAX_HEIGHT.min(area.height.saturating_sub(4));

    // Create centered popup area
    let popup_area = centered_popup(area, popup_width, popup_height);

    // Render popup background with border
    let hint_title = t!("hint.title").to_string();
    let background = popup_block(Some(&hint_title), Color::White);
    frame.render_widget(&background, popup_area);

    // Render hint text inside popup
    if let Some(hint) = task_data.hint.text() {
        let inner = inner_rect(popup_area);

        let hint_text = Paragraph::new(hint)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);

        frame.render_widget(hint_text, inner);
    }
}

/// Longest newest-first prefix of `key_history` (capped at
/// `KEY_HISTORY_DISPLAY_SIZE`) whose big-text rendering fits in `max_cells`.
///
/// Returns `None` when even the newest key alone is too wide, so the caller
/// can fall back to plain text instead of drawing clipped glyphs.
fn big_text_fitting(key_history: &VecDeque<String>, max_cells: u16) -> Option<String> {
    let mut text = String::new();
    let mut glyphs = 0usize;
    for (idx, key) in key_history
        .iter()
        .take(KEY_HISTORY_DISPLAY_SIZE)
        .enumerate()
    {
        let added = key.chars().count() + usize::from(idx > 0);
        if (glyphs + added) * BIG_TEXT_CHAR_WIDTH_CELLS > usize::from(max_cells) {
            break;
        }
        glyphs += added;
        if idx > 0 {
            text.push(' ');
        }
        text.push_str(key);
    }
    (!text.is_empty()).then_some(text)
}

/// Render key history popup showing last 5 keys pressed with large text
///
/// Used by both training mode and arcade mode. `bounds` is the outer `Rect`
/// of the Target editor panel (as returned by `render_editor_pair`); the
/// popup is anchored to the bottom-right corner of `bounds`'s *inner* area
/// (i.e. inset from `bounds`'s own border) and its width is clipped to fit
/// within that inner area, so it can never draw over the Target panel's
/// border - regardless of how wide the key text grows. Oldest keys are
/// dropped until the big text fits the popup interior; when even the newest
/// key is too wide (e.g. `Right` in a narrow panel) the history is drawn as
/// plain text instead. If the inner area is smaller than the minimum legible
/// popup size, rendering is skipped for that frame.
pub(super) fn render_key_history_popup(
    frame: &mut Frame,
    key_history: &VecDeque<String>,
    bounds: Rect,
) {
    if key_history.is_empty() {
        return;
    }

    let safe_area = inner_rect(bounds);

    // HalfHeight halves the glyph height (BIG_TEXT_HEIGHT_LINES) so the popup fits
    // within the Target panel's inner area instead of spilling past its border.
    let popup_height = BIG_TEXT_HEIGHT_LINES / 2 + POPUP_BORDER_CELLS;

    // The panel's inner area is too small to fit even a minimum-width,
    // minimum-height popup - skip rather than render illegibly clipped
    // glyphs. Never shrink `popup_height`/`KEY_HISTORY_MIN_WIDTH` below what
    // `tui-big-text` needs to stay readable.
    if safe_area.width < KEY_HISTORY_MIN_WIDTH || safe_area.height < popup_height {
        return;
    }

    let big_text_source = big_text_fitting(key_history, safe_area.width - POPUP_BORDER_CELLS);

    // PixelSize::HalfHeight only halves the glyph height, not the width - each
    // character (including the space separators) is still BIG_TEXT_CHAR_WIDTH_CELLS wide
    let popup_width = match &big_text_source {
        Some(text) => u16::try_from(text.chars().count() * BIG_TEXT_CHAR_WIDTH_CELLS)
            .unwrap_or(u16::MAX)
            .saturating_add(POPUP_BORDER_CELLS)
            .max(KEY_HISTORY_MIN_WIDTH)
            .min(safe_area.width),
        None => safe_area.width,
    };

    // Anchor to the bottom-right corner of the panel's inner area - both
    // dimensions were already clipped to `safe_area`, so this rect is always
    // a subset of `safe_area` and never touches `bounds`'s border.
    let popup_area = Rect {
        x: safe_area.x + safe_area.width.saturating_sub(popup_width),
        y: safe_area.y + safe_area.height.saturating_sub(popup_height),
        width: popup_width,
        height: popup_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if let Some(text) = big_text_source {
        let big_text = BigText::builder()
            .pixel_size(PixelSize::HalfHeight)
            .style(Style::default().fg(Color::Cyan))
            .lines(vec![text.into()])
            .centered()
            .build();
        frame.render_widget(big_text, inner_area);
    } else {
        let plain = key_history
            .iter()
            .take(KEY_HISTORY_DISPLAY_SIZE)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        let row = Rect {
            y: inner_area.y + inner_area.height / 2,
            height: 1,
            ..inner_area
        };
        let paragraph = Paragraph::new(plain)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, row);
    }
}

/// Render success popup when scenario is completed
pub(super) fn render_success_popup(frame: &mut Frame) {
    render_result_popup(
        frame,
        t!("success.title").as_ref(),
        t!("success.message").as_ref(),
        Color::Green,
    );
}

/// Render a result popup with customizable title, message, and color
///
/// Used for both training mode (SUCCESS!) and arcade mode (SUCCESS!/TIME'S UP!)
pub(super) fn render_result_popup(frame: &mut Frame, title: &str, message: &str, color: Color) {
    let area = frame.area();

    // Create centered popup area
    let popup_area = centered_popup(area, 40, 7);

    // Result message
    let result_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(message, Style::default().fg(Color::White))),
        Line::from(""),
    ];

    let result_paragraph = Paragraph::new(result_text)
        .alignment(Alignment::Center)
        .block(popup_block(None, color));

    frame.render_widget(result_paragraph, popup_area);
}

/// Render notification popups in the top-right corner
///
/// Displays up to 3 notifications stacked vertically. Notifications auto-expire
/// after their configured duration (default 3 seconds).
pub(super) fn render_notifications(frame: &mut Frame, state: &AppState) {
    if state.ui.notifications.is_empty() {
        return;
    }

    let area = frame.area();

    // Notification dimensions
    let notification_width = 40.min(area.width.saturating_sub(4));
    let notification_height = 5; // Title + message + borders
    let spacing = 1; // Space between notifications

    // Position in top-right corner
    let base_x = area.width.saturating_sub(notification_width + 2);
    let base_y = 2;

    // Render each visible notification
    for (idx, notification) in state.ui.notifications.visible().enumerate() {
        let y_offset =
            base_y + (u16::try_from(idx).unwrap_or(u16::MAX) * (notification_height + spacing));

        // Don't render if it would go off-screen
        if y_offset + notification_height > area.height {
            break;
        }

        let popup_area = Rect {
            x: base_x,
            y: y_offset,
            width: notification_width,
            height: notification_height,
        };

        // Get color for this notification type
        let color = notification.color();

        // Build notification content
        let title = notification.title();
        let message = notification.message();

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                title,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(message, Style::default().fg(Color::White))),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
                    .style(Style::default().bg(Color::Black)),
            );

        frame.render_widget(paragraph, popup_area);
    }
}

/// Render a persistent banner at the top of the screen while the profile
/// could not be loaded and saving is disabled.
///
/// Unlike a timed notification it stays until the session ends, so the user
/// always knows their progress is not being recorded. It sits at the top so
/// it never covers the bottom-row footer (key hints, `/`, `:` and `s`
/// prompts), and wraps so the backup path stays readable on narrow terminals.
pub(super) fn render_profile_load_banner(frame: &mut Frame, state: &AppState) {
    let ProfilePersistence::Quarantined(quarantine) = state.progress.persistence() else {
        return;
    };
    let backup = &quarantine.backup;
    let area = frame.area();
    if area.width == 0 || area.height == 0 {
        return;
    }

    let backup_line = match backup {
        QuarantineOutcome::BackedUp(path) => {
            t!("profile.backup_kept", path = path.display()).to_string()
        }
        QuarantineOutcome::Failed(reason) => t!("profile.no_backup", reason = reason).to_string(),
    };
    let mut lines = vec![t!("profile.load_failed").to_string(), backup_line];
    if quarantine.can_start_fresh() {
        lines.push(t!("profile.start_fresh_action").to_string());
    }
    if quarantine.can_restore_previous() {
        lines.push(t!("profile.restore_action").to_string());
    }

    // +1 line of slack because word wrapping can need more rows than a plain
    // character count suggests.
    let width = usize::from(area.width);
    let wanted: usize = lines
        .iter()
        .map(|l| l.chars().count().div_ceil(width))
        .sum::<usize>()
        + 1;
    let height = u16::try_from(wanted)
        .unwrap_or(u16::MAX)
        .min((area.height / 2).max(1));

    let banner_area = Rect { height, ..area };
    frame.render_widget(Clear, banner_area);
    frame.render_widget(
        Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
            .wrap(Wrap { trim: false })
            .style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
        banner_area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn rows(buffer: &Buffer) -> Vec<String> {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect()
            })
            .collect()
    }

    fn render_banner(state: &AppState, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_profile_load_banner(frame, state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn quarantined_state(path: &str) -> AppState {
        let mut state = crate::testing::app_state::empty_test_app_state();
        state
            .progress
            .quarantine_profile(crate::testing::quarantine(QuarantineOutcome::BackedUp(
                path.into(),
            )));
        state
    }

    #[test]
    fn test_profile_load_banner_hidden_unless_quarantined() {
        let state = crate::testing::app_state::empty_test_app_state();
        let all = rows(&render_banner(&state, 80, 10)).join("");
        assert!(!all.contains("saving is disabled"));
    }

    #[test]
    fn test_profile_load_banner_shows_full_backup_path_when_narrow() {
        let path = "/Users/someone/Library/Application Support/helix-trainer/profile.json.corrupt-20261110T224813";
        let state = quarantined_state(path);

        let rendered = rows(&render_banner(&state, 40, 20));

        let joined: String = rendered.join("").split_whitespace().collect();
        let expected: String = path.split_whitespace().collect();
        assert!(joined.contains(&expected), "{rendered:#?}");
        assert!(rendered[0].contains("Profile load failed"));
    }

    #[test]
    fn test_profile_load_banner_keeps_its_english_text() {
        let state = quarantined_state("/tmp/p.corrupt");
        let rendered = rows(&render_banner(&state, 100, 12));
        assert!(
            rendered[0].contains("Profile load failed: saving is disabled this session."),
            "{rendered:#?}"
        );
        assert!(
            rendered[1].contains("Backup kept at: /tmp/p.corrupt"),
            "{rendered:#?}"
        );
    }

    #[test]
    fn test_profile_load_banner_offers_restore_only_with_an_available_previous_copy() {
        use crate::gamification::PreviousCopy;
        use crate::ui::state::Quarantine;

        let mut state = quarantined_state("/tmp/profile.json.corrupt-1");
        let without = rows(&render_banner(&state, 100, 12)).join("");
        assert!(without.contains("Ctrl-N"));
        assert!(!without.contains("Ctrl-R"), "{without}");

        state.progress.quarantine_profile(Quarantine {
            backup: QuarantineOutcome::BackedUp("/tmp/profile.json.corrupt-1".into()),
            previous: PreviousCopy::Available,
        });
        let with = rows(&render_banner(&state, 100, 12)).join("");
        assert!(with.contains("Ctrl-N") && with.contains("Ctrl-R"), "{with}");

        state.progress.quarantine_profile(Quarantine {
            backup: crate::testing::failed_backup(),
            previous: PreviousCopy::Available,
        });
        let no_backup = rows(&render_banner(&state, 100, 12)).join("");
        assert!(
            !no_backup.contains("Ctrl-R"),
            "no backup, no restore: {no_backup}"
        );
    }

    #[test]
    #[expect(
        clippy::assert_is_empty,
        reason = "assert!(x.is_empty()) reads better than the machine fix"
    )]
    fn test_profile_load_banner_leaves_bottom_row_untouched() {
        let state = quarantined_state("/tmp/profile.json.corrupt-1");

        let rendered = rows(&render_banner(&state, 80, 10));

        assert!(rendered.last().unwrap().trim().is_empty());
    }

    fn render_popup(width: u16, height: u16, bounds: Rect, keys: &[String]) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                // Simulate the Target panel's own border, matching what
                // `render_editor_pair` renders into `bounds` in production.
                frame.render_widget(Block::default().borders(Borders::ALL), bounds);
                render_key_history_popup(frame, &keys.iter().cloned().collect(), bounds);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_bounds_only(width: u16, height: u16, bounds: Rect) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                frame.render_widget(Block::default().borders(Borders::ALL), bounds);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    /// Assert every cell on `bounds`'s own border rectangle is unchanged
    /// between the two buffers - i.e. the popup never drew over it.
    fn assert_border_unchanged(bounds: Rect, baseline: &Buffer, other: &Buffer) {
        for x in bounds.x..bounds.x + bounds.width {
            assert_eq!(
                baseline[(x, bounds.y)],
                other[(x, bounds.y)],
                "top border cell ({x}, {}) was overwritten",
                bounds.y
            );
            let bottom_y = bounds.y + bounds.height - 1;
            assert_eq!(
                baseline[(x, bottom_y)],
                other[(x, bottom_y)],
                "bottom border cell ({x}, {bottom_y}) was overwritten"
            );
        }
        for y in bounds.y..bounds.y + bounds.height {
            assert_eq!(
                baseline[(bounds.x, y)],
                other[(bounds.x, y)],
                "left border cell ({}, {y}) was overwritten",
                bounds.x
            );
            let right_x = bounds.x + bounds.width - 1;
            assert_eq!(
                baseline[(right_x, y)],
                other[(right_x, y)],
                "right border cell ({right_x}, {y}) was overwritten"
            );
        }
    }

    #[test]
    fn test_render_key_history_popup_empty_history_skips_render() {
        let bounds = Rect::new(10, 5, 40, 12);
        let buffer = render_popup(80, 20, bounds, &[]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "empty key history must render nothing beyond the panel border"
        );
    }

    fn row_text(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect()
    }

    /// Regression test for #664: a label whose big text (`Left` = 32 cells)
    /// is wider than the popup interior (28 cells) must be shown legibly in
    /// full instead of as clipped glyphs or a blank box.
    #[test]
    fn test_render_key_history_popup_shows_wide_label_in_full() {
        let bounds = Rect::new(10, 5, 32, 12);
        let buffer = render_popup(80, 20, bounds, &["Left".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);

        assert_border_unchanged(bounds, &baseline, &buffer);
        assert!(
            (0..20).any(|y| row_text(&buffer, y).contains("Left")),
            "wide label must be drawn as plain text"
        );
    }

    #[test]
    fn test_big_text_fitting_drops_oldest_keys_until_text_fits() {
        let history: VecDeque<String> = ["a", "b", "c", "d", "e"].map(String::from).into();
        assert_eq!(big_text_fitting(&history, 40).as_deref(), Some("a b c"));
        assert_eq!(
            big_text_fitting(&history, 100).as_deref(),
            Some("a b c d e")
        );
        let wide: VecDeque<String> = ["Right".to_string()].into();
        assert_eq!(big_text_fitting(&wide, 28), None);
    }

    #[test]
    fn test_render_key_history_popup_skips_when_bounds_too_small() {
        // Inner area (after the panel's own border) is only 1x1 - too small
        // to fit a bordered popup without touching `bounds`'s own border.
        let bounds = Rect::new(10, 5, 3, 3);
        let buffer = render_popup(80, 20, bounds, &["a".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "popup must not render when bounds leave no room for it"
        );
    }

    /// Regression test: an inner area narrower than `KEY_HISTORY_MIN_WIDTH`
    /// (but still >= 3 cells) must be skipped entirely rather than rendered
    /// as an illegibly squished/clipped popup - the popup's minimum legible
    /// size must never be silently shrunk below what `tui-big-text` needs.
    #[test]
    fn test_render_key_history_popup_skips_rather_than_render_illegibly_narrow() {
        // Inner width is well above 3 cells but well below KEY_HISTORY_MIN_WIDTH.
        let bounds = Rect::new(10, 5, 20, 12);
        assert!(inner_rect(bounds).width < KEY_HISTORY_MIN_WIDTH);

        let buffer = render_popup(80, 20, bounds, &["a".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "popup must be skipped, not squished, when narrower than the minimum legible width"
        );
    }

    /// Regression test: an inner area shorter than the minimum popup height
    /// (but still >= 3 cells) must be skipped entirely rather than rendered
    /// with a vertically clipped glyph.
    #[test]
    fn test_render_key_history_popup_skips_rather_than_render_illegibly_short() {
        let min_height = BIG_TEXT_HEIGHT_LINES / 2 + 2;
        // Inner height is well above 3 cells but below the minimum popup height.
        let bounds = Rect::new(10, 5, 40, min_height);
        assert!(inner_rect(bounds).height < min_height);

        let buffer = render_popup(80, 20, bounds, &["a".to_string()]);
        let baseline = render_bounds_only(80, 20, bounds);
        assert_eq!(
            buffer, baseline,
            "popup must be skipped, not vertically clipped, when shorter than the minimum legible height"
        );
    }

    /// Regression test for #364: a growing, multi-key history (e.g. while
    /// typing a regex pattern, recording a macro, or using a named
    /// register) must never draw over the Target panel's border, no matter
    /// how wide the rendered key text gets.
    #[test]
    fn test_render_key_history_popup_never_overwrites_panel_border() {
        let bounds = Rect::new(10, 5, 40, 12);
        let wide_keys: Vec<String> = vec![
            "regex-pattern".to_string(),
            "Space".to_string(),
            "⌫".to_string(),
            "↵".to_string(),
            "Esc".to_string(),
        ];

        let buffer = render_popup(80, 20, bounds, &wide_keys);
        let baseline = render_bounds_only(80, 20, bounds);

        assert_border_unchanged(bounds, &baseline, &buffer);
        // Sanity: the popup did actually render something inside the panel.
        assert_ne!(
            buffer, baseline,
            "popup should have rendered inside the panel bounds"
        );
    }

    #[test]
    fn test_render_key_history_popup_stays_inside_bounds_inner_area() {
        let bounds = Rect::new(10, 5, 40, 12);
        let inner = inner_rect(bounds);
        let buffer = render_popup(80, 20, bounds, &["Esc".to_string(), "Space".to_string()]);

        // Every non-blank cell outside `bounds` entirely must remain blank,
        // and nothing should render between `bounds`'s edge and its inner area.
        for y in 0..20u16 {
            for x in 0..80u16 {
                let inside_inner = x >= inner.x
                    && x < inner.x + inner.width
                    && y >= inner.y
                    && y < inner.y + inner.height;
                let inside_bounds = x >= bounds.x
                    && x < bounds.x + bounds.width
                    && y >= bounds.y
                    && y < bounds.y + bounds.height;
                if inside_bounds && !inside_inner {
                    // Border ring of `bounds` - already covered by the
                    // border-unchanged test; skip here.
                    continue;
                }
                if !inside_inner {
                    let symbol = buffer[(x, y)].symbol();
                    assert!(
                        symbol == " " || symbol.is_empty(),
                        "unexpected content at ({x}, {y}) outside the panel's inner area: {symbol:?}"
                    );
                }
            }
        }
    }
}
