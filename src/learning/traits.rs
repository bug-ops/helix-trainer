//! Common traits for learning and progression systems
//!
//! These traits provide unified interfaces for:
//! - Progression tiers (Learning → Proficient → Mastered)
//! - Progress tracking (0.0 - 1.0 completion)

/// Trait for types representing multi-tier progression levels
///
/// Implemented by mastery systems that have distinct progression tiers,
/// such as scenario mastery (Learning/Proficient/Mastered) or command
/// mastery (Beginner/Intermediate/Advanced/Master).
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::learning::ProgressionTier;
///
/// fn display_mastery<T: ProgressionTier>(tier: &T) {
///     println!("{} {}", tier.emoji(), tier.name());
/// }
/// ```
pub trait ProgressionTier: Copy + Eq {
    /// Get display name (e.g., "Learning", "Beginner")
    fn name(&self) -> &'static str;

    /// Get emoji representation for UI
    fn emoji(&self) -> &'static str;

    /// Get numeric tier level (0 = lowest, higher = more advanced)
    fn tier_level(&self) -> u32;

    /// Check if this is the maximum/final tier
    fn is_max_tier(&self) -> bool;

    /// Get XP modifier description (e.g., "(-50% XP)" for reduced rewards)
    fn xp_description(&self) -> &'static str {
        "" // Default: no description
    }
}

/// Trait for types that track numeric progress toward a goal
///
/// Implemented by quest types, achievements, and other goal-oriented
/// tracking systems that measure progress as a fraction (0.0 to 1.0).
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::learning::ProgressTracker;
///
/// fn render_progress_bar<T: ProgressTracker>(tracker: &T) {
///     let pct = (tracker.progress() * 100.0) as u32;
///     println!("[{:>3}%] {}", pct, if tracker.is_complete() { "✓" } else { "" });
/// }
/// ```
pub trait ProgressTracker {
    /// Get current progress as fraction (0.0 to 1.0)
    ///
    /// Returns value clamped to [0.0, 1.0] range.
    fn progress(&self) -> f64;

    /// Get current value (numerator)
    fn current(&self) -> u64;

    /// Get target value (denominator)
    fn target(&self) -> u64;

    /// Check if progress is complete (>= 100%)
    fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }

    /// Get progress as percentage (0 to 100)
    fn progress_percent(&self) -> u32 {
        (self.progress() * 100.0).min(100.0) as u32
    }
}

/// Trait for types that modify numeric values (XP multipliers, score bonuses)
///
/// Used for streak bonuses, mastery penalties, difficulty modifiers, etc.
///
/// # Examples
///
/// ```ignore
/// use helix_trainer::learning::Modifier;
///
/// fn apply_all_modifiers(base: u64, modifiers: &[&dyn Modifier]) -> u64 {
///     modifiers.iter().fold(base, |acc, m| m.apply(acc))
/// }
/// ```
pub trait Modifier {
    /// Get modification factor (1.0 = no change, 2.0 = double, 0.5 = half)
    fn factor(&self) -> f64;

    /// Apply modification to a value
    fn apply(&self, value: u64) -> u64 {
        (value as f64 * self.factor()).round() as u64
    }

    /// Check if this is a beneficial modifier (factor > 1.0)
    fn is_beneficial(&self) -> bool {
        self.factor() > 1.0
    }

    /// Check if this is a penalty modifier (factor < 1.0)
    fn is_penalty(&self) -> bool {
        self.factor() < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation for ProgressionTier
    #[allow(dead_code)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum TestTier {
        Low,
        Mid,
        High,
    }

    impl ProgressionTier for TestTier {
        fn name(&self) -> &'static str {
            match self {
                TestTier::Low => "Low",
                TestTier::Mid => "Mid",
                TestTier::High => "High",
            }
        }

        fn emoji(&self) -> &'static str {
            match self {
                TestTier::Low => "🔰",
                TestTier::Mid => "⭐",
                TestTier::High => "🏆",
            }
        }

        fn tier_level(&self) -> u32 {
            match self {
                TestTier::Low => 0,
                TestTier::Mid => 1,
                TestTier::High => 2,
            }
        }

        fn is_max_tier(&self) -> bool {
            matches!(self, TestTier::High)
        }
    }

    #[test]
    fn test_progression_tier_methods() {
        let low = TestTier::Low;
        let high = TestTier::High;

        assert_eq!(low.name(), "Low");
        assert_eq!(low.emoji(), "🔰");
        assert_eq!(low.tier_level(), 0);
        assert!(!low.is_max_tier());

        assert!(high.is_max_tier());
        assert_eq!(high.tier_level(), 2);
    }

    // Test implementation for ProgressTracker
    struct TestProgress {
        current: u64,
        target: u64,
    }

    impl ProgressTracker for TestProgress {
        fn progress(&self) -> f64 {
            if self.target == 0 {
                0.0
            } else {
                (self.current as f64 / self.target as f64).min(1.0)
            }
        }

        fn current(&self) -> u64 {
            self.current
        }

        fn target(&self) -> u64 {
            self.target
        }
    }

    #[test]
    fn test_progress_tracker_methods() {
        let progress = TestProgress {
            current: 50,
            target: 100,
        };

        assert!((progress.progress() - 0.5).abs() < f64::EPSILON);
        assert_eq!(progress.progress_percent(), 50);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_progress_tracker_complete() {
        let complete = TestProgress {
            current: 100,
            target: 100,
        };

        assert!(complete.is_complete());
        assert_eq!(complete.progress_percent(), 100);
    }

    #[test]
    fn test_progress_tracker_overflow() {
        let over = TestProgress {
            current: 150,
            target: 100,
        };

        // Progress should be clamped to 1.0
        assert!((over.progress() - 1.0).abs() < f64::EPSILON);
        assert_eq!(over.progress_percent(), 100);
    }

    // Test implementation for Modifier
    struct TestModifier {
        factor: f64,
    }

    impl Modifier for TestModifier {
        fn factor(&self) -> f64 {
            self.factor
        }
    }

    #[test]
    fn test_modifier_apply() {
        let double = TestModifier { factor: 2.0 };
        let half = TestModifier { factor: 0.5 };
        let neutral = TestModifier { factor: 1.0 };

        assert_eq!(double.apply(100), 200);
        assert_eq!(half.apply(100), 50);
        assert_eq!(neutral.apply(100), 100);
    }

    #[test]
    fn test_modifier_classification() {
        let bonus = TestModifier { factor: 1.5 };
        let penalty = TestModifier { factor: 0.8 };
        let neutral = TestModifier { factor: 1.0 };

        assert!(bonus.is_beneficial());
        assert!(!bonus.is_penalty());

        assert!(penalty.is_penalty());
        assert!(!penalty.is_beneficial());

        assert!(!neutral.is_beneficial());
        assert!(!neutral.is_penalty());
    }
}
