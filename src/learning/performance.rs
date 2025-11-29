use chrono::{DateTime, Utc};
use fsrs::{FSRS, MemoryState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_DIFFICULTY: f32 = 5.0; // Middle difficulty (0-10 scale)
const DEFAULT_DESIRED_RETENTION: f32 = 0.9; // 90% target retention

/// Card state tracked by the application (FSRS doesn't track this)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardState {
    New,
    Learning,
    Review,
    Relearning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPerformance {
    pub command: String,

    // FSRS state (stored as Option<MemoryState>)
    pub stability: f32,   // Memory stability (days)
    pub difficulty: f32,  // Item difficulty (0-10)
    pub state: CardState, // Card state (application-managed)
    pub reps: u32,        // Total repetitions
    pub lapses: u32,      // Number of lapses (forgot)

    // Performance metrics (application-specific)
    pub attempts: u32,
    pub successes: u32,
    #[serde(with = "duration_serde")]
    pub total_time: Duration,
    #[serde(with = "duration_serde")]
    pub avg_time: Duration,

    // Scheduling
    pub last_review: DateTime<Utc>,
    pub due: DateTime<Utc>,
    pub scheduled_days: u32,

    // Derived fields
    pub mastery_level: MasteryLevel,
    pub retrievability: f32, // Current recall probability (0.0-1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MasteryLevel {
    Beginner,
    Intermediate,
    Advanced,
    Master,
}

impl CommandPerformance {
    pub fn new(command: String) -> Self {
        let now = Utc::now();
        Self {
            command,
            stability: 0.0,
            difficulty: DEFAULT_DIFFICULTY,
            state: CardState::New,
            reps: 0,
            lapses: 0,
            attempts: 0,
            successes: 0,
            total_time: Duration::ZERO,
            avg_time: Duration::ZERO,
            last_review: now,
            due: now,
            scheduled_days: 0,
            mastery_level: MasteryLevel::Beginner,
            retrievability: 1.0, // New card, perfect retrievability
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }

    pub fn update_mastery_level(&mut self) {
        self.mastery_level = match self.state {
            CardState::New | CardState::Learning => MasteryLevel::Beginner,
            CardState::Review | CardState::Relearning => match (self.stability, self.difficulty) {
                (s, d) if s > 30.0 && d < 5.0 => MasteryLevel::Master,
                (s, _) if s > 7.0 => MasteryLevel::Advanced,
                _ => MasteryLevel::Intermediate,
            },
        };
    }

    /// Get MemoryState for FSRS (None if New)
    pub fn memory_state(&self) -> Option<MemoryState> {
        if self.reps == 0 {
            None
        } else {
            Some(MemoryState {
                stability: self.stability,
                difficulty: self.difficulty,
            })
        }
    }

    /// Update from FSRS NextState and determine card state
    pub fn update_from_next_state(
        &mut self,
        memory: MemoryState,
        interval: f32,
        rating_index: usize,
    ) {
        // Determine new state based on rating and current state
        self.state = match (self.state, rating_index) {
            // Again (0) - failed
            (_, 0) => {
                self.lapses += 1;
                if self.reps > 0 {
                    CardState::Relearning
                } else {
                    CardState::Learning
                }
            }
            // Hard (1) - struggled but passed
            (CardState::New, 1) | (CardState::Learning, 1) => CardState::Learning,
            (CardState::Relearning, 1) => CardState::Relearning,
            (CardState::Review, 1) => CardState::Review,
            // Good (2) or Easy (3) - succeeded well
            (CardState::New, _) | (CardState::Learning, _) => {
                if interval >= 1.0 {
                    CardState::Review
                } else {
                    CardState::Learning
                }
            }
            (CardState::Relearning, _) => CardState::Review,
            (CardState::Review, _) => CardState::Review,
        };

        self.stability = memory.stability;
        self.difficulty = memory.difficulty;
        self.scheduled_days = interval.round().max(1.0) as u32;
        self.last_review = Utc::now();

        // For first review (reps == 0), make command immediately due
        // This allows users to practice new commands right away
        // Subsequent reviews use FSRS scheduling with 1+ day intervals
        if self.reps == 0 {
            self.due = self.last_review; // Immediately due
        } else {
            self.due = self.last_review + chrono::Duration::days(self.scheduled_days as i64);
        }

        self.reps += 1;
    }
}

// Helper module for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

pub struct PerformanceTracker {
    stats: HashMap<String, CommandPerformance>,
    fsrs: FSRS, // FSRS scheduler instance
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            fsrs: FSRS::new(Some(&[])).unwrap(), // Use default parameters
        }
    }

    pub fn record_attempt(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
    ) {
        // Get or create performance entry
        let perf = self
            .stats
            .entry(command.to_string())
            .or_insert_with(|| CommandPerformance::new(command.to_string()));

        // Update attempt counters
        perf.attempts += 1;
        if success {
            perf.successes += 1;
        }

        perf.total_time += duration;
        perf.avg_time = perf.total_time / perf.attempts;

        // Update FSRS state
        self.update_fsrs_state(command, duration, success, optimal_time);

        // Update mastery level
        if let Some(perf) = self.stats.get_mut(command) {
            perf.update_mastery_level();
        }
    }

    /// Calculate next FSRS state based on rating
    fn calculate_next_fsrs_state(
        &self,
        memory_state: Option<MemoryState>,
        elapsed_days: u32,
        rating_index: usize,
    ) -> fsrs::ItemState {
        let next_states = self
            .fsrs
            .next_states(memory_state, DEFAULT_DESIRED_RETENTION, elapsed_days)
            .unwrap();

        match rating_index {
            0 => next_states.again,
            1 => next_states.hard,
            2 => next_states.good,
            3 => next_states.easy,
            _ => unreachable!("rating_index must be 0-3"),
        }
    }

    /// Calculate elapsed days since last review
    fn elapsed_days_since_review(perf: &CommandPerformance) -> u32 {
        if perf.reps == 0 {
            0
        } else {
            (Utc::now() - perf.last_review).num_days().max(0) as u32
        }
    }

    fn update_fsrs_state(
        &mut self,
        command: &str,
        duration: Duration,
        success: bool,
        optimal_time: Duration,
    ) {
        let perf = self.stats.get(command).unwrap();

        // Convert performance to FSRS rating (0-3 index for again/hard/good/easy)
        let rating_index = Self::calculate_rating_index(duration, success, optimal_time);

        // Get current memory state and elapsed time
        let memory_state = perf.memory_state();
        let elapsed_days = Self::elapsed_days_since_review(perf);

        // Calculate next state from FSRS
        let next_state = self.calculate_next_fsrs_state(memory_state, elapsed_days, rating_index);

        // Update performance with new FSRS state
        let perf = self.stats.get_mut(command).unwrap();
        perf.update_from_next_state(next_state.memory, next_state.interval, rating_index);

        // Update retrievability (current recall probability)
        perf.retrievability = fsrs::current_retrievability(
            perf.memory_state().unwrap_or(MemoryState {
                stability: 0.0,
                difficulty: DEFAULT_DIFFICULTY,
            }),
            elapsed_days as f32,
            -0.5, // Default decay parameter
        );
    }

    fn calculate_rating_index(duration: Duration, success: bool, optimal_time: Duration) -> usize {
        if !success {
            return 0; // Again (complete failure)
        }

        let ratio = duration.as_secs_f64() / optimal_time.as_secs_f64();

        match ratio {
            r if r <= 1.5 => 3, // Easy - instant recall (< 1.5x optimal)
            r if r <= 3.0 => 2, // Good - normal recall (1.5-3x optimal)
            r if r <= 5.0 => 1, // Hard - struggled (3-5x optimal)
            _ => 0,             // Again - essentially forgot (> 5x optimal)
        }
    }

    pub fn get_performance(&self, command: &str) -> Option<&CommandPerformance> {
        self.stats.get(command)
    }

    pub fn get_weak_commands(&self) -> Vec<String> {
        self.stats
            .iter()
            .filter(|(_, perf)| {
                perf.difficulty > 7.0
                    || perf.success_rate() < 0.7
                    || perf.mastery_level == MasteryLevel::Beginner
                    || perf.lapses > 2
            })
            .map(|(cmd, _)| cmd.clone())
            .collect()
    }

    pub fn all_commands(&self) -> Vec<&str> {
        self.stats.keys().map(String::as_str).collect()
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_command_defaults() {
        let perf = CommandPerformance::new("dd".to_string());
        assert_eq!(perf.stability, 0.0);
        assert_eq!(perf.difficulty, DEFAULT_DIFFICULTY);
        assert_eq!(perf.reps, 0);
        assert_eq!(perf.state, CardState::New);
        assert_eq!(perf.mastery_level, MasteryLevel::Beginner);
        assert_eq!(perf.retrievability, 1.0);
        assert!(perf.memory_state().is_none());
    }

    #[test]
    fn test_perfect_performance_increases_stability() {
        let mut tracker = PerformanceTracker::new();

        // First attempt (perfect)
        tracker.record_attempt("dd", Duration::from_secs(1), true, Duration::from_secs(1));

        let perf = tracker.get_performance("dd").unwrap();
        assert!(perf.stability > 0.0);
        assert_eq!(perf.reps, 1);
        assert!(matches!(
            perf.state,
            CardState::Learning | CardState::Review
        ));

        let first_stability = perf.stability;

        // Second attempt (perfect)
        tracker.record_attempt("dd", Duration::from_secs(1), true, Duration::from_secs(1));

        let perf = tracker.get_performance("dd").unwrap();
        assert!(perf.stability > first_stability); // FSRS increases stability
        assert_eq!(perf.reps, 2);
    }

    #[test]
    fn test_failure_increases_lapses() {
        let mut tracker = PerformanceTracker::new();

        // Build up some stability
        tracker.record_attempt("dd", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("dd", Duration::from_secs(1), true, Duration::from_secs(1));

        let lapses_before = tracker.get_performance("dd").unwrap().lapses;

        // Fail
        tracker.record_attempt("dd", Duration::from_secs(10), false, Duration::from_secs(1));

        let perf = tracker.get_performance("dd").unwrap();
        // Lapse count increases when failing
        assert_eq!(perf.lapses, lapses_before + 1);
        assert!(matches!(
            perf.state,
            CardState::Relearning | CardState::Learning
        ));
    }

    #[test]
    fn test_difficulty_bounds() {
        let mut tracker = PerformanceTracker::new();

        // Spam failures - difficulty should stay within 0-10
        for _ in 0..100 {
            tracker.record_attempt("dd", Duration::from_secs(10), false, Duration::from_secs(1));
        }

        let perf = tracker.get_performance("dd").unwrap();
        assert!(perf.difficulty >= 0.0);
        assert!(perf.difficulty <= 10.0);
    }

    #[test]
    fn test_rating_conversion() {
        // Test performance → FSRS rating conversion
        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(1),
                true,
                Duration::from_secs(1)
            ),
            3 // Easy
        );

        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(2),
                true,
                Duration::from_secs(1)
            ),
            2 // Good
        );

        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(4),
                true,
                Duration::from_secs(1)
            ),
            1 // Hard
        );

        assert_eq!(
            PerformanceTracker::calculate_rating_index(
                Duration::from_secs(10),
                false,
                Duration::from_secs(1)
            ),
            0 // Again
        );
    }

    #[test]
    fn test_memory_state_conversion() {
        let mut perf = CommandPerformance::new("dd".to_string());
        assert!(perf.memory_state().is_none());

        perf.stability = 5.0;
        perf.difficulty = 6.0;
        perf.reps = 1;
        perf.state = CardState::Review;

        let memory = perf.memory_state().unwrap();
        assert_eq!(memory.stability, 5.0);
        assert_eq!(memory.difficulty, 6.0);
    }

    #[test]
    fn test_mastery_level_progression() {
        let mut perf = CommandPerformance::new("dd".to_string());
        assert_eq!(perf.mastery_level, MasteryLevel::Beginner);

        // Simulate progression through states
        perf.state = CardState::Review;
        perf.stability = 5.0;
        perf.update_mastery_level();
        assert_eq!(perf.mastery_level, MasteryLevel::Intermediate);

        perf.stability = 10.0;
        perf.update_mastery_level();
        assert_eq!(perf.mastery_level, MasteryLevel::Advanced);

        perf.stability = 35.0;
        perf.difficulty = 4.0;
        perf.update_mastery_level();
        assert_eq!(perf.mastery_level, MasteryLevel::Master);
    }

    #[test]
    fn test_weak_commands_detection() {
        let mut tracker = PerformanceTracker::new();

        // Add a weak command (low success rate)
        for _ in 0..10 {
            tracker.record_attempt(
                "weak",
                Duration::from_secs(10),
                false,
                Duration::from_secs(1),
            );
        }

        // Add a strong command
        for _ in 0..10 {
            tracker.record_attempt(
                "strong",
                Duration::from_secs(1),
                true,
                Duration::from_secs(1),
            );
        }

        let weak = tracker.get_weak_commands();
        assert!(weak.contains(&"weak".to_string()));
        // "strong" might still be beginner mastery level initially, so don't assert it's not weak
    }

    #[test]
    fn test_all_commands() {
        let mut tracker = PerformanceTracker::new();

        tracker.record_attempt("dd", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("yy", Duration::from_secs(1), true, Duration::from_secs(1));
        tracker.record_attempt("p", Duration::from_secs(1), true, Duration::from_secs(1));

        let all = tracker.all_commands();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"dd"));
        assert!(all.contains(&"yy"));
        assert!(all.contains(&"p"));
    }
}
