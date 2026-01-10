//! Sound effect types

/// Sound effect types for mini-games
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
    /// Success chime on scenario completion
    ScenarioComplete,
    /// Failure buzz on timeout
    ScenarioFailed,
    /// Rising tone when multiplier increases
    MultiplierUp,
    /// Fanfare when difficulty level increases
    LevelUp,
    /// Alert sound when losing a life
    LifeLost,
    /// Jingle when game ends
    GameOver,
    /// Tick sound for 3-2-1 countdown
    Countdown,
    /// Warning when <25% time remaining
    TimerWarning,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_sound_effect_variants() {
        // Test that all variants can be used as HashMap keys
        let mut map: HashMap<SoundEffect, &str> = HashMap::new();

        map.insert(SoundEffect::ScenarioComplete, "complete");
        map.insert(SoundEffect::ScenarioFailed, "failed");
        map.insert(SoundEffect::MultiplierUp, "multiplier");
        map.insert(SoundEffect::LevelUp, "levelup");
        map.insert(SoundEffect::LifeLost, "lifelost");
        map.insert(SoundEffect::GameOver, "gameover");
        map.insert(SoundEffect::Countdown, "countdown");
        map.insert(SoundEffect::TimerWarning, "warning");

        assert_eq!(map.len(), 8);
        assert_eq!(map.get(&SoundEffect::ScenarioComplete), Some(&"complete"));
    }

    #[test]
    fn test_sound_effect_clone_copy() {
        let effect = SoundEffect::LevelUp;
        // Use explicit copy instead of clone for Copy types
        let copied1 = effect;
        let copied2 = effect;

        assert_eq!(effect, copied1);
        assert_eq!(effect, copied2);
    }

    #[test]
    fn test_sound_effect_debug() {
        let effect = SoundEffect::GameOver;
        let debug_str = format!("{:?}", effect);
        assert_eq!(debug_str, "GameOver");
    }

    #[test]
    fn test_sound_effect_exhaustive_match() {
        // This will fail to compile if a new variant is added
        fn assert_all_variants(effect: SoundEffect) -> &'static str {
            match effect {
                SoundEffect::ScenarioComplete => "complete",
                SoundEffect::ScenarioFailed => "failed",
                SoundEffect::MultiplierUp => "multiplier",
                SoundEffect::LevelUp => "levelup",
                SoundEffect::LifeLost => "lifelost",
                SoundEffect::GameOver => "gameover",
                SoundEffect::Countdown => "countdown",
                SoundEffect::TimerWarning => "warning",
            }
        }
        assert_eq!(assert_all_variants(SoundEffect::GameOver), "gameover");
    }
}
