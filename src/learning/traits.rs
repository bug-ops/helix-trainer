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
/// ```
/// use helix_trainer::learning::{MasteryLevel, ProgressionTier};
///
/// fn label<T: ProgressionTier>(tier: &T) -> String {
///     format!("{} {}", tier.emoji(), tier.name())
/// }
///
/// assert!(label(&MasteryLevel::Master).ends_with("Master"));
/// assert!(MasteryLevel::Master.is_max_tier());
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
/// ```
/// use helix_trainer::gamification::QuestType;
/// use helix_trainer::learning::ProgressTracker;
///
/// let quest = QuestType::ScenarioCompletion { target: 4, current: 1 };
/// assert_eq!(quest.progress_percent(), 25);
/// assert!(!ProgressTracker::is_complete(&quest));
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
        crate::numeric::floor_percent(self.progress())
    }
}

/// Trait for types that modify numeric values (XP multipliers, score bonuses)
///
/// Used for streak bonuses, mastery penalties, difficulty modifiers, etc.
///
/// # Examples
///
/// ```
/// use helix_trainer::learning::Modifier;
///
/// struct Double;
///
/// impl Modifier for Double {
///     fn factor(&self) -> f64 {
///         2.0
///     }
/// }
///
/// assert_eq!(Double.factor(), 2.0);
/// ```
pub trait Modifier {
    /// Get modification factor (1.0 = no change, 2.0 = double, 0.5 = half)
    fn factor(&self) -> f64;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test implementation for ProgressionTier
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum TestTier {
        Low,
        High,
    }

    impl ProgressionTier for TestTier {
        fn name(&self) -> &'static str {
            match self {
                TestTier::Low => "Low",
                TestTier::High => "High",
            }
        }

        fn emoji(&self) -> &'static str {
            match self {
                TestTier::Low => "🔰",
                TestTier::High => "🏆",
            }
        }

        fn tier_level(&self) -> u32 {
            match self {
                TestTier::Low => 0,
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
        #[expect(
            clippy::cast_precision_loss,
            reason = "integer counters and scores stay far below 2^52, so the f64 value is exact"
        )]
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
}
