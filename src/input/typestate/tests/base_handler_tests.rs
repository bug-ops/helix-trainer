//! Tests for `BaseState` handler

use crate::grammar::find::FindMotion;
use crate::testing::normal;
use std::assert_matches;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::grammar::commands::*;
use crate::input::typestate::{BaseState, HandlerResult, InputHandler, InputState, KeyHandler};

#[test]
fn test_base_state_goto_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Transition(InputState::GotoPending));
}

#[test]
fn test_base_state_view_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Transition(InputState::ViewPending));
}

#[test]
fn test_base_state_match_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Transition(InputState::MatchPending));
}

#[test]
fn test_base_state_find_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            motion: FindMotion::FIND_FORWARD
        })
    );
}

#[test]
fn test_base_state_replace_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::ReplaceCharPending)
    );
}

#[test]
fn test_base_state_count_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE),
    );
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::CountPending {
            count: super::nz(3)
        })
    );
}

#[test]
fn test_base_state_single_key_command() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_MOVE_LEFT);
}

#[test]
fn test_base_state_escape() {
    let result =
        KeyHandler::handle_key(&BaseState, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_ESCAPE);
}

#[test]
fn test_base_state_find_backward_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('F'), KeyModifiers::SHIFT),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            motion: FindMotion::FIND_BACKWARD
        })
    );
}

#[test]
fn test_base_state_till_backward_prefix() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT),
    );
    assert_matches!(
        result,
        HandlerResult::Transition(InputState::FindCharPending {
            motion: FindMotion::TILL_BACKWARD
        })
    );
}

#[test]
fn test_base_state_unknown_key_stays() {
    // Test that unknown keys stay in base state.
    // '@' is not bound to anything - unlike 'q', which is the macro
    // replay command (see test_base_state_macro_replay_transitions below).
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('@'), KeyModifiers::NONE),
    );
    assert_matches!(result, HandlerResult::Stay);
}

#[test]
fn test_base_state_macro_toggle_executes() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT),
    );
    assert_eq!(
        result,
        HandlerResult::Execute(normal(CMD_TOGGLE_MACRO_RECORDING))
    );
}

#[test]
fn test_base_state_macro_replay_executes() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
    );
    assert_eq!(result, HandlerResult::Execute(normal(CMD_REPLAY_MACRO)));
}

#[test]
fn test_base_state_select_regex_prompt_transitions() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
    );
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::RegexPromptPending {
            kind: crate::helix::normal_command::PromptKind::SelectRegex,
            buffer: String::new(),
        })
    );
}

#[test]
fn test_base_state_split_selection_prompt_transitions() {
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT),
    );
    assert_eq!(
        result,
        HandlerResult::Transition(InputState::RegexPromptPending {
            kind: crate::helix::normal_command::PromptKind::SplitSelection,
            buffer: String::new(),
        })
    );
}

#[test]
fn test_base_state_page_movement_commands() {
    // Regression test for issue #198: Ctrl-b/f/u/d must reach the
    // page/half-page movement commands, not be swallowed by the generic
    // "ignore other modifier combinations" CONTROL fallback.
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_PAGE_UP);

    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_PAGE_DOWN);

    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_HALF_PAGE_UP);

    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_HALF_PAGE_DOWN);
}

#[test]
fn test_base_state_replace_with_yanked() {
    // Regression test for issue #198: bare 'R' must resolve to
    // replace_with_yanked instead of staying unmapped.
    let result = KeyHandler::handle_key(
        &BaseState,
        KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT),
    );
    assert_matches!(result, HandlerResult::Execute(cmd) if cmd.to_string() == CMD_REPLACE_WITH_YANKED);
}

fn base_key(c: char, modifiers: KeyModifiers) -> HandlerResult {
    KeyHandler::handle_key(&BaseState, KeyEvent::new(KeyCode::Char(c), modifiers))
}

/// #487: Alt-s must run `split_selection_on_newline`, not open the `s` prompt.
#[test]
fn test_base_state_alt_s_executes_split_selection_on_newline() {
    assert_eq!(
        base_key('s', KeyModifiers::ALT),
        HandlerResult::Execute(normal(CMD_SPLIT_SELECTION_NEWLINES))
    );
}

/// #487: no Alt chord except `Alt-K` (the remove prompt) may open a pending state.
#[test]
fn test_base_state_alt_chords_never_open_pending_state() {
    for c in "gzm[]ftrFTSQq\":sS123456789/".chars() {
        let result = base_key(c, KeyModifiers::ALT);
        assert!(
            !matches!(result, HandlerResult::Transition(_)),
            "Alt-{c} opened a pending state: {result:?}"
        );
    }
    assert_eq!(base_key('g', KeyModifiers::ALT), HandlerResult::Stay);
    assert_eq!(base_key('5', KeyModifiers::ALT), HandlerResult::Stay);
}

/// #487: other Alt-bound commands still resolve.
#[test]
fn test_base_state_alt_bound_commands_still_execute() {
    assert_matches!(
        base_key('c', KeyModifiers::ALT),
        HandlerResult::Execute(cmd) if cmd.to_string() == CMD_CHANGE_SELECTION_NOYANK
    );
    assert_matches!(
        base_key('C', KeyModifiers::ALT | KeyModifiers::SHIFT),
        HandlerResult::Execute(cmd) if cmd.to_string() == CMD_COPY_SELECTION_PREV
    );
}

fn prompt_transition(kind: crate::helix::normal_command::PromptKind) -> HandlerResult {
    HandlerResult::Transition(InputState::RegexPromptPending {
        kind,
        buffer: String::new(),
    })
}

/// #513: `?` opens the backward-search prompt (also as Shift-`/`).
#[test]
fn test_base_state_question_mark_opens_backward_search_prompt() {
    use crate::helix::normal_command::PromptKind::SearchBackward;
    assert_eq!(
        base_key('?', KeyModifiers::NONE),
        prompt_transition(SearchBackward)
    );
    assert_eq!(
        base_key('?', KeyModifiers::SHIFT),
        prompt_transition(SearchBackward)
    );
    assert_eq!(
        base_key('/', KeyModifiers::SHIFT),
        prompt_transition(SearchBackward)
    );
}

/// #510: `K` and `Alt-K` open the keep/remove prompts; `&` runs immediately.
#[test]
fn test_base_state_keep_remove_prompts_and_align() {
    use crate::helix::normal_command::PromptKind::{KeepSelections, RemoveSelections};
    assert_eq!(
        base_key('K', KeyModifiers::SHIFT),
        prompt_transition(KeepSelections)
    );
    assert_eq!(
        base_key('K', KeyModifiers::ALT | KeyModifiers::SHIFT),
        prompt_transition(RemoveSelections)
    );
    assert_eq!(
        base_key('&', KeyModifiers::SHIFT),
        HandlerResult::Execute(normal(CMD_ALIGN_SELECTIONS))
    );
}

/// #504: `/` opens the search prompt instead of executing a bare command.
#[test]
fn test_base_state_slash_opens_search_prompt() {
    assert_eq!(
        base_key('/', KeyModifiers::NONE),
        HandlerResult::Transition(InputState::RegexPromptPending {
            kind: crate::helix::normal_command::PromptKind::Search,
            buffer: String::new(),
        })
    );
}
