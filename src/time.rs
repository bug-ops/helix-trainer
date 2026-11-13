//! Clock abstraction for testable time-dependent logic.
//!
//! Production code obtains the current time through a [`Clock`] implementation instead of
//! calling `Utc::now()` directly, so tests can inject a controllable clock to drive
//! day-boundary and scheduling behavior deterministically. The controllable clocks,
//! `FakeClock` and `FakeMonotonicClock`, exist in unit tests and behind the `test-util` feature.

use chrono::{DateTime, FixedOffset, Local, NaiveDate, Offset, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Instant;
#[cfg(any(test, feature = "test-util"))]
use {chrono::Duration, std::sync::Mutex, std::time::Duration as StdDuration};

/// A calendar day in the player's local time zone.
///
/// Every "daily" rule (quests, streaks, the daily challenge, scenario history) compares
/// these, never UTC dates. Persisted instants stay UTC and become a `PlayerDay` through
/// [`Clock::day_of`] at read time. Serializes as a `YYYY-MM-DD` string.
///
/// # Examples
///
/// ```
/// use helix_trainer::time::PlayerDay;
///
/// let monday = PlayerDay::from_ymd(2026, 1, 12).unwrap();
/// let tuesday = PlayerDay::from_ymd(2026, 1, 13).unwrap();
/// assert!(tuesday > monday);
/// assert_eq!(tuesday.days_since(monday), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerDay(NaiveDate);

impl PlayerDay {
    /// The player day of a calendar date.
    #[must_use]
    pub const fn from_date(date: NaiveDate) -> Self {
        Self(date)
    }

    /// The player day of year, month and day, or `None` for an impossible date.
    #[must_use]
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Option<Self> {
        NaiveDate::from_ymd_opt(year, month, day).map(Self)
    }

    /// The calendar date.
    #[must_use]
    pub const fn date(self) -> NaiveDate {
        self.0
    }

    /// Days from `earlier` to `self`; negative when `earlier` is later.
    #[must_use]
    pub fn days_since(self, earlier: Self) -> i64 {
        self.0.signed_duration_since(earlier.0).num_days()
    }
}

/// The time zone the player's days are counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The operating system's local zone; undeterminable zones behave as UTC.
    Local,
    /// A fixed offset from UTC, for tests.
    Fixed(FixedOffset),
}

impl Zone {
    /// The player day of `instant` in this zone.
    #[must_use]
    pub fn day_of(self, instant: DateTime<Utc>) -> PlayerDay {
        let offset = match self {
            Self::Local => instant.with_timezone(&Local).offset().fix(),
            Self::Fixed(offset) => offset,
        };
        // A persisted instant near the edge of the calendar can overflow the local date;
        // such a value reads in UTC instead of panicking.
        PlayerDay(
            instant
                .naive_utc()
                .checked_add_offset(offset)
                .map_or_else(|| instant.date_naive(), |local| local.date()),
        )
    }
}

/// An instant together with the zone the player's days are counted in.
///
/// Daily rules take a `Moment` so that "today" and the day of a stored instant are read in
/// one zone. A bare `DateTime<Utc>` converts in the system's local zone.
///
/// # Examples
///
/// ```
/// use chrono::Utc;
/// use helix_trainer::time::Moment;
///
/// let moment = Moment::from(Utc::now());
/// assert_eq!(moment.day(), moment.day_of(moment.instant()));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    instant: DateTime<Utc>,
    zone: Zone,
}

impl Moment {
    /// `instant` seen in `zone`.
    #[must_use]
    pub const fn new(instant: DateTime<Utc>, zone: Zone) -> Self {
        Self { instant, zone }
    }

    /// The UTC instant.
    #[must_use]
    pub const fn instant(&self) -> DateTime<Utc> {
        self.instant
    }

    /// The player day of this moment.
    #[must_use]
    pub fn day(&self) -> PlayerDay {
        self.zone.day_of(self.instant)
    }

    /// The player day of another instant, in this moment's zone.
    #[must_use]
    pub fn day_of(&self, instant: DateTime<Utc>) -> PlayerDay {
        self.zone.day_of(instant)
    }
}

impl From<DateTime<Utc>> for Moment {
    fn from(instant: DateTime<Utc>) -> Self {
        Self::new(instant, Zone::Local)
    }
}

impl fmt::Display for PlayerDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
/// holders of `Arc<dyn Clock>` are shared across tokio tasks. Callers may assume `player_day()`
/// is consistent with `now()` (same instant, seen in the player's local time zone).
pub trait Clock: Send + Sync {
    /// Returns the current UTC time.
    fn now(&self) -> DateTime<Utc>;

    /// The zone the player's days are counted in.
    fn zone(&self) -> Zone;

    /// The player's local calendar day at `instant`, in the zone in effect now.
    fn day_of(&self, instant: DateTime<Utc>) -> PlayerDay {
        self.zone().day_of(instant)
    }

    /// The player's current local calendar day, derived from [`Clock::now`].
    fn player_day(&self) -> PlayerDay {
        self.day_of(self.now())
    }

    /// The current time as a [`Moment`] in the clock's zone.
    fn moment(&self) -> Moment {
        Moment::new(self.now(), self.zone())
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

    fn zone(&self) -> Zone {
        Zone::Local
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
    zone: Mutex<FixedOffset>,
}

#[cfg(any(test, feature = "test-util"))]
impl FakeClock {
    /// Creates a fake clock fixed at the given time.
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            now: Mutex::new(now),
            zone: Mutex::new(Utc.fix()),
        }
    }

    /// Creates a fake clock fixed at `rfc3339` whose player lives `hours` east of UTC
    /// (negative for west).
    ///
    /// # Panics
    ///
    /// Panics for an invalid timestamp or an offset of a day or more.
    pub fn at_offset(rfc3339: &str, hours: i32) -> Self {
        let clock = Self::at(rfc3339);
        clock.set_utc_offset_hours(hours);
        clock
    }

    /// Moves the player to a zone `hours` east of UTC (negative for west).
    ///
    /// # Panics
    ///
    /// Panics for an offset of a day or more.
    pub fn set_utc_offset_hours(&self, hours: i32) {
        *self
            .zone
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            FixedOffset::east_opt(hours * 3600).expect("offset within a day");
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

    fn zone(&self) -> Zone {
        Zone::Fixed(
            *self
                .zone
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
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
        assert_eq!(
            clock.player_day(),
            PlayerDay::from_ymd(2026, 1, 15).unwrap()
        );
    }

    #[test]
    fn fake_clock_advance_days_moves_today() {
        let clock = FakeClock::at("2026-01-15T12:00:00Z");
        clock.advance_days(1);
        assert_eq!(
            clock.player_day(),
            PlayerDay::from_ymd(2026, 1, 16).unwrap()
        );
    }

    #[test]
    fn fake_clock_zone_decides_the_player_day() {
        let west = FakeClock::at_offset("2026-01-16T00:30:00Z", -8);
        let east = FakeClock::at_offset("2026-01-15T23:30:00Z", 9);

        assert_eq!(west.player_day(), PlayerDay::from_ymd(2026, 1, 15).unwrap());
        assert_eq!(east.player_day(), PlayerDay::from_ymd(2026, 1, 16).unwrap());
    }

    #[test]
    fn a_dst_length_day_is_still_one_day() {
        let day = |d| PlayerDay::from_ymd(2026, 3, d).unwrap();
        assert_eq!(day(9).days_since(day(8)), 1);
    }

    #[test]
    fn player_day_serializes_as_a_plain_date() {
        let day = PlayerDay::from_ymd(2026, 1, 15).unwrap();
        assert_eq!(serde_json::to_string(&day).unwrap(), "\"2026-01-15\"");
        assert_eq!(
            serde_json::from_str::<PlayerDay>("\"2026-01-15\"").unwrap(),
            day
        );
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
