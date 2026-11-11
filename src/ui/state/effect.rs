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

/// An [`Effect`] that needs the async runtime, handed back to the event loop by
/// [`Effect::execute`]; matching on it there is exhaustive, so a new variant cannot be
/// silently mistaken for another one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncEffect {
    /// Replace the quarantined profile with a fresh one (#518)
    StartFreshProfile,
}

impl Effect {
    /// Run the effects that only need the in-memory progress state.
    ///
    /// Returns the [`AsyncEffect`] when the effect needs the async runtime, so the event loop
    /// can run it; a failed save is logged, never fatal.
    #[must_use = "an unhandled effect is dropped silently"]
    pub fn execute(self, progress: &mut ProgressState) -> Option<AsyncEffect> {
        match self {
            Self::Play(effect) => progress.sound_manager.play(effect),
            Self::SaveProfile => {
                if let Err(e) = progress.save_immediate() {
                    tracing::error!("Failed to save profile: {:?}", e);
                }
            }
            Self::StartFreshProfile => return Some(AsyncEffect::StartFreshProfile),
        }
        None
    }
}
