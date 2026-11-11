//! Side effects requested by [`update`](super::update)
//!
//! `update` only mutates state; anything that touches the outside world (audio, disk, the
//! data loader) is returned as an [`Effect`] for the caller to execute. Effects pushed before a
//! failing step are still delivered, so a recoverable error never drops a save or a sound.

use crate::sound::SoundEffect;

use super::ProgressState;

/// A side effect requested by a state update
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Play a sound effect
    Play(SoundEffect),
    /// Persist the profile now
    SaveProfile,
    /// Replace the quarantined profile with a fresh one, keeping the backup (#518)
    StartFreshProfile,
}

impl Effect {
    /// Run the effects that only need the in-memory progress state.
    ///
    /// Returns the effect back when it still needs the async runtime
    /// ([`Effect::StartFreshProfile`]), so the event loop can run it with an exhaustive match;
    /// a failed save is logged, never fatal.
    #[must_use = "an unhandled effect is dropped silently"]
    pub fn execute(self, progress: &mut ProgressState) -> Option<Self> {
        match self {
            Self::Play(effect) => progress.sound_manager.play(effect),
            Self::SaveProfile => {
                if let Err(e) = progress.save_immediate() {
                    tracing::error!("Failed to save profile: {:?}", e);
                }
            }
            Self::StartFreshProfile => return Some(self),
        }
        None
    }
}
