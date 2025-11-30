//! Mini-game state machine
//!
//! Defines the state machine for mini-game flow control.

use serde::{Deserialize, Serialize};

/// Mini-game state machine
///
/// Controls the flow of the mini-game from countdown to game over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiniGameState {
    /// Showing 3-2-1 countdown before start
    ///
    /// The countdown displays before gameplay begins.
    /// Remaining value counts down from 3 to 1.
    Countdown { remaining: u8 },

    /// Active gameplay
    ///
    /// Player is actively solving scenarios with timer running.
    Playing,

    /// Brief pause between scenarios
    ///
    /// Short transition period after completing one scenario
    /// before the next one appears.
    Transition,

    /// Game paused by user
    ///
    /// Player pressed pause, timer stopped, can resume or quit.
    Paused,

    /// Game over (no lives left)
    ///
    /// Final state when player runs out of lives.
    /// Shows final score and statistics.
    GameOver,
}

impl MiniGameState {
    /// Check if game is in active playing state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameState;
    ///
    /// let state = MiniGameState::Playing;
    /// assert!(state.is_playing());
    ///
    /// let state = MiniGameState::Paused;
    /// assert!(!state.is_playing());
    /// ```
    pub fn is_playing(&self) -> bool {
        matches!(self, Self::Playing)
    }

    /// Check if game is paused
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameState;
    ///
    /// let state = MiniGameState::Paused;
    /// assert!(state.is_paused());
    /// ```
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused)
    }

    /// Check if game is over
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameState;
    ///
    /// let state = MiniGameState::GameOver;
    /// assert!(state.is_game_over());
    /// ```
    pub fn is_game_over(&self) -> bool {
        matches!(self, Self::GameOver)
    }

    /// Check if in countdown state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameState;
    ///
    /// let state = MiniGameState::Countdown { remaining: 3 };
    /// assert!(state.is_countdown());
    /// ```
    pub fn is_countdown(&self) -> bool {
        matches!(self, Self::Countdown { .. })
    }

    /// Check if in transition state
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameState;
    ///
    /// let state = MiniGameState::Transition;
    /// assert!(state.is_transition());
    /// ```
    pub fn is_transition(&self) -> bool {
        matches!(self, Self::Transition)
    }

    /// Get countdown remaining value
    ///
    /// Returns Some(remaining) if in Countdown state, None otherwise.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use helix_trainer::minigame::MiniGameState;
    ///
    /// let state = MiniGameState::Countdown { remaining: 2 };
    /// assert_eq!(state.countdown_remaining(), Some(2));
    ///
    /// let state = MiniGameState::Playing;
    /// assert_eq!(state.countdown_remaining(), None);
    /// ```
    pub fn countdown_remaining(&self) -> Option<u8> {
        match self {
            Self::Countdown { remaining } => Some(*remaining),
            _ => None,
        }
    }
}

impl Default for MiniGameState {
    fn default() -> Self {
        Self::Countdown { remaining: 3 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_playing() {
        assert!(MiniGameState::Playing.is_playing());
        assert!(!MiniGameState::Paused.is_playing());
        assert!(!MiniGameState::GameOver.is_playing());
        assert!(!MiniGameState::Countdown { remaining: 3 }.is_playing());
        assert!(!MiniGameState::Transition.is_playing());
    }

    #[test]
    fn test_is_paused() {
        assert!(MiniGameState::Paused.is_paused());
        assert!(!MiniGameState::Playing.is_paused());
    }

    #[test]
    fn test_is_game_over() {
        assert!(MiniGameState::GameOver.is_game_over());
        assert!(!MiniGameState::Playing.is_game_over());
    }

    #[test]
    fn test_is_countdown() {
        assert!(MiniGameState::Countdown { remaining: 3 }.is_countdown());
        assert!(MiniGameState::Countdown { remaining: 1 }.is_countdown());
        assert!(!MiniGameState::Playing.is_countdown());
    }

    #[test]
    fn test_is_transition() {
        assert!(MiniGameState::Transition.is_transition());
        assert!(!MiniGameState::Playing.is_transition());
    }

    #[test]
    fn test_countdown_remaining() {
        assert_eq!(
            MiniGameState::Countdown { remaining: 3 }.countdown_remaining(),
            Some(3)
        );
        assert_eq!(
            MiniGameState::Countdown { remaining: 1 }.countdown_remaining(),
            Some(1)
        );
        assert_eq!(MiniGameState::Playing.countdown_remaining(), None);
    }

    #[test]
    fn test_default_state() {
        let state = MiniGameState::default();
        assert_eq!(state, MiniGameState::Countdown { remaining: 3 });
    }
}
