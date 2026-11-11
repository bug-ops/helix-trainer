//! Clock abstraction for testable time-dependent logic.
//!
//! Production code obtains the current time through a [`Clock`] implementation instead of
//! calling `Utc::now()` directly, so tests can inject a controllable clock to drive
//! day-boundary and scheduling behavior deterministically. The controllable clocks,
//! `FakeClock` and `FakeMonotonicClock`, exist in unit tests and behind the `test-util` feature.

use chrono::{DateTime, NaiveDate, Utc};
use std::time::Instant;
#[cfg(any(test, feature = "test-util"))]
use {chrono::Duration, std::sync::Mutex, std::time::Duration as StdDuration};

/// Source of monotonic time for elapsed and timeout logic.
///
/// Unlike [`Clock`] (wall-clock, for day boundaries) this never goes backwards, so
/// sessions measure durations with it; tests inject a `FakeMonotonicClock` instead of sleeping.
///
/// Implementors must guarantee `now()` is non-decreasing and safe to call from any thread.
pub trait MonotonicClock: Send + Sync {
    /// Returns the current monotonic instant.
    fn now(&self) -> Instant;
}

/// [`MonotonicClock`] backed by [`Instant::now`].
///
/// # Examples
///
/// ```
/// use helix_trainer::time::{MonotonicClock, SystemMonotonicClock};
///
/// let clock = SystemMonotonicClock;
/// assert!(clock.now() <= clock.now());
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemMonotonicClock;

impl MonotonicClock for SystemMonotonicClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// [`MonotonicClock`] that only moves when told to, for deterministic timer tests.
///
/// Available in unit tests and behind the `test-util` feature.
///
/// # Examples
///
/// ```
/// use helix_trainer::time::{FakeMonotonicClock, MonotonicClock};
/// use std::time::Duration;
///
/// let clock = FakeMonotonicClock::new();
/// let start = clock.now();
/// clock.advance(Duration::from_secs(5));
/// assert_eq!(clock.now() - start, Duration::from_secs(5));
/// ```
#[cfg(any(test, feature = "test-util"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
#[derive(Debug)]
pub struct FakeMonotonicClock {
    origin: Instant,
    offset: Mutex<StdDuration>,
}

#[cfg(any(test, feature = "test-util"))]
impl FakeMonotonicClock {
    /// Creates a fake clock anchored at the current real instant.
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            offset: Mutex::new(StdDuration::ZERO),
        }
    }

    /// Moves the clock forward by `duration`.
    pub fn advance(&self, duration: StdDuration) {
        *self
            .offset
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) += duration;
    }
}

#[cfg(any(test, feature = "test-util"))]
impl Default for FakeMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(test, feature = "test-util"))]
impl MonotonicClock for FakeMonotonicClock {
    fn now(&self) -> Instant {
        self.origin
            + *self
                .offset
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Source of the current time for application logic.
///
/// Implementors must guarantee `now()` is safe to call from any thread, since long-lived
/// holders of `Arc<dyn Clock>` are shared across tokio tasks. Callers may assume `today()`
/// is consistent with `now()` (same instant, just truncated to a date).
pub trait Clock: Send + Sync {
    /// Returns the current UTC time.
    fn now(&self) -> DateTime<Utc>;

    /// Returns the current UTC date, derived from [`Clock::now`].
    fn today(&self) -> NaiveDate {
        self.now().date_naive()
    }
}

/// [`Clock`] implementation backed by the system wall clock.
///
/// # Examples
///
/// ```
/// use helix_trainer::time::{Clock, SystemClock};
///
/// let clock = SystemClock;
/// let _now = clock.now();
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// [`Clock`] implementation with a manually controlled time, for tests.
///
/// Available in unit tests and behind the `test-util` feature.
///
/// # Examples
///
/// ```
/// use chrono::Duration;
/// use helix_trainer::time::{Clock, FakeClock};
///
/// let clock = FakeClock::at("2026-01-15T12:00:00Z");
/// let before = clock.now();
/// clock.advance(Duration::days(1));
/// assert!(clock.now() > before);
/// ```
#[cfg(any(test, feature = "test-util"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
#[derive(Debug)]
pub struct FakeClock {
    now: Mutex<DateTime<Utc>>,
}

#[cfg(any(test, feature = "test-util"))]
impl FakeClock {
    /// Creates a fake clock fixed at the given time.
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Creates a fake clock fixed at the given RFC 3339 timestamp.
    ///
    /// # Panics
    ///
    /// Panics if `rfc3339` is not a valid RFC 3339 timestamp. Intended for test ergonomics
    /// where a malformed literal is a test bug, not a runtime condition to handle.
    pub fn at(rfc3339: &str) -> Self {
        let now = DateTime::parse_from_rfc3339(rfc3339)
            .unwrap_or_else(|err| panic!("invalid RFC 3339 timestamp {rfc3339:?}: {err}"))
            .with_timezone(&Utc);
        Self::new(now)
    }

    /// Sets the clock to the given time.
    pub fn set(&self, now: DateTime<Utc>) {
        *self
            .now
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = now;
    }

    /// Advances the clock by the given duration.
    pub fn advance(&self, duration: Duration) {
        let mut guard = self
            .now
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard += duration;
    }

    /// Advances the clock by the given number of days.
    pub fn advance_days(&self, days: i64) {
        self.advance(Duration::days(days));
    }
}

#[cfg(any(test, feature = "test-util"))]
impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self
            .now
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_now_is_current() {
        let clock = SystemClock;
        let before = Utc::now();
        let now = clock.now();
        let after = Utc::now();
        assert!(now >= before && now <= after);
    }

    #[test]
    fn fake_clock_at_parses_rfc3339() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        assert_eq!(clock.today(), NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
    }

    #[test]
    fn fake_clock_advance_days_moves_today() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        clock.advance_days(1);
        assert_eq!(clock.today(), NaiveDate::from_ymd_opt(2026, 1, 16).unwrap());
    }

    #[test]
    fn fake_clock_set_overrides_time() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        let new_time = DateTime::parse_from_rfc3339("2027-06-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        clock.set(new_time);
        assert_eq!(clock.now(), new_time);
    }
}
