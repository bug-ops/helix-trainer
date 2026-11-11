//! Helper functions for common rendering patterns

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

/// XP earned inside the current level and the XP the level spans, as `(earned, needed)`
///
/// The next level is `level + 1`, saturating so a profile at the level cap renders instead of
/// overflowing (#523).
pub(super) fn level_xp_progress(profile: &crate::gamification::UserProfile) -> (u64, u64) {
    use crate::gamification::XPCalculator;

    let current_level_xp = XPCalculator::xp_for_level(profile.level);
    let next_level_xp = XPCalculator::xp_for_level(profile.level.saturating_add(1));
    (
        profile.total_xp.saturating_sub(current_level_xp),
        next_level_xp.saturating_sub(current_level_xp),
    )
}

/// Calculate centered popup area with given dimensions
///
/// # Arguments
///
/// * `parent` - The parent area to center within
/// * `width` - Desired popup width
/// * `height` - Desired popup height
///
/// # Returns
///
/// A Rect centered within the parent area
pub(super) fn centered_popup(parent: Rect, width: u16, height: u16) -> Rect {
    let popup_x = (parent.width.saturating_sub(width)) / 2;
    let popup_y = (parent.height.saturating_sub(height)) / 2;

    Rect {
        x: popup_x,
        y: popup_y,
        width,
        height,
    }
}

/// Create a standard popup block with borders
///
/// # Arguments
///
/// * `title` - Optional title for the popup
/// * `border_color` - Color for the border
///
/// # Returns
///
/// A Block with standard styling
pub(super) fn popup_block(title: Option<&str>, border_color: Color) -> Block<'_> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(Color::Black));

    if let Some(t) = title {
        block = block.title(t);
    }

    block
}

/// Calculate the inner area of a rect (excluding borders)
///
/// # Arguments
///
/// * `outer` - The outer rect with borders
///
/// # Returns
///
/// The inner rect without borders
pub(super) fn inner_rect(outer: Rect) -> Rect {
    Rect {
        x: outer.x + 1,
        y: outer.y + 1,
        width: outer.width.saturating_sub(2),
        height: outer.height.saturating_sub(2),
    }
}

// Re-export find_surrounding_brackets from helix module
pub(super) use crate::helix::find_surrounding_brackets;

/// Build a live feedback string for an in-progress `"`-register selection,
/// `:`-command-line buffer, or `s`/`S`/`/` regex prompt, so the user
/// can see what they're typing.
///
/// Returns an empty string when none is pending. Shared between Training
/// (`task.rs`) and Arcade (`minigame.rs`) rendering, since both screens'
/// `InputStateMachine` can be in any of these pending states.
pub(super) fn pending_input_indicator(
    input_state: &crate::input::typestate::InputStateMachine,
) -> String {
    if let Some(buffer) = input_state.pending_command_line() {
        return format!(" :{} ", buffer);
    }
    if let Some((kind, pattern)) = input_state.pending_regex_prompt() {
        return format!(" {}{} ", kind.prefix(), pattern);
    }
    if let Some(preview) = input_state.pending_register() {
        use crate::input::typestate::RegisterPreview;
        return match preview {
            RegisterPreview::SelectingRegister => " [Register: _] ".to_string(),
            RegisterPreview::AwaitingOperator(r) => format!(" [Register: {}] ", r),
        };
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::pending_input_indicator;
    use crate::input::typestate::InputStateMachine;

    fn press(sm: &mut InputStateMachine, keys: &str) {
        for c in keys.chars() {
            sm.process_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
    }

    #[test]
    fn search_prompt_shows_slash_and_typed_pattern() {
        let mut sm = InputStateMachine::new();
        assert_eq!(pending_input_indicator(&sm), "");
        press(&mut sm, "/su");
        assert_eq!(pending_input_indicator(&sm), " /su ");
    }

    #[test]
    fn regex_selection_prompts_keep_their_prefix() {
        let mut sm = InputStateMachine::new();
        press(&mut sm, "sa");
        assert_eq!(pending_input_indicator(&sm), " sa ");
    }
}
