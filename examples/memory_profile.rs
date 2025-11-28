//! Memory profiling with DHAT for scenario mastery system
//!
//! Run with:
//!   cargo run --release --example memory_profile
//!
//! View results:
//!   Open dhat-heap.json at https://nnethercote.github.io/dh_view/dh_view.html

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioMastery {
    Learning,
    Proficient,
    Mastered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioCompletion {
    pub scenario_id: String,
    pub attempts: u32,
    pub best_score: u32,
    pub perfect_count: u32,
    pub total_xp_earned: u64,
    pub first_attempt: DateTime<Utc>,
    pub last_attempt: DateTime<Utc>,
    pub mastery_level: ScenarioMastery,
    pub attempts_today: u32,
    pub last_attempt_date: String,
}

impl ScenarioCompletion {
    pub fn new(scenario_id: String) -> Self {
        let now = Utc::now();
        Self {
            scenario_id,
            attempts: 0,
            best_score: 0,
            perfect_count: 0,
            total_xp_earned: 0,
            first_attempt: now,
            last_attempt: now,
            mastery_level: ScenarioMastery::Learning,
            attempts_today: 0,
            last_attempt_date: now.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn record_completion(&mut self, score: u32, xp_earned: u64) {
        let now = Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        if today != self.last_attempt_date {
            self.attempts_today = 0;
            self.last_attempt_date = today;
        }

        self.attempts += 1;
        self.attempts_today += 1;
        self.best_score = self.best_score.max(score);
        if score == 100 {
            self.perfect_count += 1;
        }
        self.total_xp_earned += xp_earned;
        self.last_attempt = now;

        self.update_mastery();
    }

    fn update_mastery(&mut self) {
        self.mastery_level = if self.perfect_count >= 2 {
            ScenarioMastery::Mastered
        } else if self.attempts >= 3 && self.best_score >= 90 {
            ScenarioMastery::Proficient
        } else {
            ScenarioMastery::Learning
        };
    }

    pub fn xp_multiplier(&self) -> f64 {
        let mastery_mult = match self.mastery_level {
            ScenarioMastery::Learning => 1.0,
            ScenarioMastery::Proficient => 0.5,
            ScenarioMastery::Mastered => 0.2,
        };

        let session_mult = match self.attempts_today {
            0 => 1.0,
            1..=2 => 0.7,
            _ => 0.3,
        };

        mastery_mult * session_mult
    }
}

pub struct ScenarioHistoryTracker {
    completions: HashMap<String, ScenarioCompletion>,
}

impl Default for ScenarioHistoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ScenarioHistoryTracker {
    pub fn new() -> Self {
        Self {
            completions: HashMap::new(),
        }
    }

    pub fn get_or_create(&mut self, scenario_id: &str) -> &mut ScenarioCompletion {
        self.completions
            .entry(scenario_id.to_string())
            .or_insert_with(|| ScenarioCompletion::new(scenario_id.to_string()))
    }

    pub fn get_xp_multiplier(&self, scenario_id: &str) -> f64 {
        self.completions
            .get(scenario_id)
            .map(|c| c.xp_multiplier())
            .unwrap_or(1.0)
    }

    pub fn record_completion(&mut self, scenario_id: &str, score: u32, base_xp: u64) -> u64 {
        let multiplier = self.get_xp_multiplier(scenario_id);
        let actual_xp = (base_xp as f64 * multiplier) as u64;

        let completion = self.get_or_create(scenario_id);
        completion.record_completion(score, actual_xp);

        actual_xp
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.completions).unwrap()
    }
}

fn main() {
    let _profiler = dhat::Profiler::new_heap();

    println!("Memory Profiling: Scenario Mastery XP Scaling");
    println!("==============================================\n");

    // Test 1: Small scenario set (typical user - 50 scenarios)
    println!("Test 1: Small scenario set (50 scenarios)");
    {
        let mut tracker = ScenarioHistoryTracker::new();
        for i in 0..50 {
            let scenario_id = format!("scenario_{:03}", i);
            // Simulate 3 attempts per scenario
            tracker.record_completion(&scenario_id, 80, 50);
            tracker.record_completion(&scenario_id, 95, 50);
            tracker.record_completion(&scenario_id, 100, 50);
        }
        let json = tracker.to_json();
        println!("  JSON size: {} bytes", json.len());
    }

    // Test 2: Medium scenario set (active user - 200 scenarios)
    println!("\nTest 2: Medium scenario set (200 scenarios)");
    {
        let mut tracker = ScenarioHistoryTracker::new();
        for i in 0..200 {
            let scenario_id = format!("scenario_{:03}", i);
            tracker.record_completion(&scenario_id, 80, 50);
            tracker.record_completion(&scenario_id, 95, 50);
            tracker.record_completion(&scenario_id, 100, 50);
        }
        let json = tracker.to_json();
        println!("  JSON size: {} bytes", json.len());
    }

    // Test 3: Large scenario set (worst case - 1000 scenarios)
    println!("\nTest 3: Large scenario set (1000 scenarios)");
    {
        let mut tracker = ScenarioHistoryTracker::new();
        for i in 0..1000 {
            let scenario_id = format!("scenario_{:03}", i);
            tracker.record_completion(&scenario_id, 80, 50);
            tracker.record_completion(&scenario_id, 95, 50);
            tracker.record_completion(&scenario_id, 100, 50);
        }
        let json = tracker.to_json();
        println!("  JSON size: {} bytes", json.len());
    }

    // Test 4: Repeated attempts (spam detection)
    println!("\nTest 4: Repeated attempts (10 scenarios, 100 attempts each)");
    {
        let mut tracker = ScenarioHistoryTracker::new();
        for i in 0..10 {
            let scenario_id = format!("scenario_{:03}", i);
            for _ in 0..100 {
                tracker.record_completion(&scenario_id, 100, 50);
            }
        }
        let json = tracker.to_json();
        println!("  JSON size: {} bytes", json.len());
    }

    // Test 5: HashMap growth pattern
    println!("\nTest 5: HashMap growth (0 -> 10000 scenarios)");
    {
        let mut tracker = ScenarioHistoryTracker::new();
        for i in 0..10000 {
            let scenario_id = format!("scenario_{:05}", i);
            tracker.record_completion(&scenario_id, 100, 50);
        }
        let json = tracker.to_json();
        println!("  JSON size: {} bytes", json.len());
    }

    println!("\nProfiling complete. Results saved to dhat-heap.json");
    println!("View at: https://nnethercote.github.io/dh_view/dh_view.html");
}
