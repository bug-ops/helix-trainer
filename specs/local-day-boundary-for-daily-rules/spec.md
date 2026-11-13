---
aliases:
  - Local Day Boundary
  - PlayerDay
  - Local midnight rollover
tags:
  - sdd
  - spec
  - gamification
  - time
  - status/proposed
created: 2026-11-13
status: proposed
related:
  - "[[constitution]]"
  - "[[arcade-gamification-session-fixes/spec]]"
  - "[[challenge-attempt-limit-not-enforced/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Daily Quests, Streaks and the Daily Challenge Roll Over at Local Midnight

> [!info] Metadata
> **Issue**: #660 (P3; labels `bug`, `gamification`)
> **Status**: Proposed
> **Depth**: Lightweight spec with a recorded decision.
> **Decision (2026-11-13, user)**: the day boundary is the player's LOCAL midnight, for quests, streaks, the daily challenge and scenario history alike.

## 1. Overview

### Problem Statement

Every "daily" rule is computed on the UTC calendar date: `Clock::today()` returns the UTC date, the quest refresh compares the UTC dates of `now` and `last_quest_refresh`, streaks diff the UTC dates of two instants, and the daily challenge and scenario history roll over on the UTC date. For a player at UTC-8 the day ends at 16:00 local time; two sessions on one local evening (15:30 and 16:30) can count as two streak days, while 23:00 and 09:00 the next local day can be zero days apart. At UTC+9 the reset happens at 09:00. No package stated which midnight was meant, and `NaiveDate` records no zone.

### Goal

A "day" is one calendar date in the player's local time zone, recorded in the type system, and every daily rule uses it with one forward-only comparison.

### Out of Scope

- A per-profile time-zone setting or a UI to choose a day boundary (the OS zone is used)
- A timer that fires at midnight while the app idles (rollover is applied at the checkpoints of FR-008)
- Instants: every persisted timestamp stays a UTC `DateTime` (review log, `last_quest_refresh`, attempt times)
- Changing streak-freeze rules other than the day arithmetic they sit on

## 2. Functional Requirements

| ID | Requirement | Priority |
|----|------------|----------|
| FR-001 | THE SYSTEM SHALL define the player's day as a newtype `PlayerDay` over a calendar date in the local time zone. `Clock` provides `player_day()`; the system clock derives it from the local zone, the fake clock from a fixed offset (default UTC) so tests control the zone. Quest, streak, challenge and scenario-history code takes `PlayerDay`, never `NaiveDate` or `date_naive()` of a UTC instant | must |
| FR-002 | EVERY day-rollover check SHALL be `today > stored` on `PlayerDay` (strictly later): the quest refresh (`should_refresh_quests`), the daily challenge counter reset, and the scenario-history daily rollover. A reset happens only when today is strictly after the stored day | must |
| FR-003 | A stored day equal to or after today SHALL count as today (no reset), and a stored day only ever moves forward (`max(stored, today)`); a clock or zone moved backward never rewinds it | must |
| FR-004 | THE streak gap SHALL be the number of local player days between the last activity day and today. A gap of 0 or less counts as the same day (the streak continues unchanged, the freeze is not consumed), a gap of 1 extends the streak, a larger gap follows the existing freeze and break rules (`STREAK_FREEZE_MAX_GAP_DAYS`). The last activity day never moves backward. This replaces the "negative gap lower-bounded at 2 days" row of [[arcade-gamification-session-fixes/spec]] | must |
| FR-005 | PERSISTED instants stay UTC and are converted to a `PlayerDay` at read time with the zone in effect then. The `profile.json` wire format does not change (dates remain `YYYY-MM-DD` strings, instants remain RFC 3339 UTC) | must |
| FR-006 | UPGRADE: dates stored by the previous version (UTC calendar dates) SHALL be read as local days without migration. Consequences, accepted and stated in the CHANGELOG breaking note: west of UTC, a player who used today's challenge attempts in the evening finds them locked through the following local day, once; east of UTC, the reset comes up to the offset's hours earlier, once | must |
| FR-007 | TIME-ZONE CHANGE (travel, DST rule change, manual change) SHALL be safe: moving west cannot reset anything again that was already reset for the same stored day (FR-003); moving east and back (east, west, east) produces at most one reset per real day; a backward change keeps the streak (FR-004). A daylight-saving transition is not special: the day is derived from the local date, so a 23-hour or 25-hour day is one day | must |
| FR-008 | THE rollover SHALL be evaluated at the checkpoints where the UTC rule is evaluated today (startup, profile ready, mode selection, scenario completion, quest refresh); no new timer is introduced | must |
| FR-009 | WHEN the OS time zone cannot be determined THE SYSTEM SHALL behave as UTC (the previous behavior); this is the platform behavior of the date-time library and needs no error path | should |

## 3. Acceptance Criteria

```
GIVEN a fake clock at UTC-8 and a streak whose last activity was today 15:30 local
WHEN  the next activity happens at 16:30 local the same day (00:30 UTC next date)
THEN  the gap is 0 days, the streak is unchanged (it was 2 UTC days apart before)
```
```
GIVEN a fake clock at UTC-8, activity at 23:00 local and the next at 09:00 local the following day
THEN  the gap is 1 day and the streak extends by one
```
```
GIVEN a fake clock at UTC+9 and quests refreshed at 08:00 local
WHEN  the time is 09:30 local the same day (00:30 UTC)
THEN  the quests are NOT refreshed (they were at 09:00 local under the UTC rule)
```
```
GIVEN the stored challenge day equals tomorrow's local date (written by the previous version west of UTC)
WHEN  the challenge is attempted today
THEN  the stored day is treated as today, attempts are not reset, and the stored day does not move backward
```
```
GIVEN a zone sequence east, west, east on one real day
THEN  the quests, the challenge counters and the scenario-history daily bucket reset exactly once
```
```
GIVEN a zone change that moves local today before last_activity
WHEN  an activity is recorded
THEN  the streak is kept, the freeze is not consumed and last_activity does not move backward
```
```
GIVEN a day with a DST transition (23 or 25 hours)
THEN  activity on that date and the next date is a gap of exactly 1 day
```

## 4. Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Activity exactly at local 00:00:00 | Belongs to the new day |
| First launch (nothing stored) | Today is the stored day; no reset or streak gap computed |
| Stored date unparseable | Existing load policy of [[challenge-attempt-limit-not-enforced/spec]] (lenient deserializer) is unchanged |
| Persisted `last_quest_refresh` instant | Converted with `player_day()`; compared per FR-002 |
| Review-event log timestamps | Unchanged (UTC instants) |

## 5. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | Fake-clock tests at UTC-8 and UTC+9 for quests, streak, challenge and scenario history (the cases of section 3) | Pass |
| SC-002 | Upgrade fixtures: stored UTC dates read as local days, including the future-day case | Pass |
| SC-003 | East-west-east toggle yields exactly one reset; backward zone change keeps the streak | Pass |
| SC-004 | `rg "date_naive\\(\\)" src/` outside `time.rs` and the clock implementations | 0 matches |
| SC-005 | CHANGELOG carries a breaking-change note for FR-006 | Present |

## 6. Agent Boundaries

### Always (without asking)

- Compare days only through `PlayerDay` and `today > stored`
- Cover every rule with a fake clock at a negative and a positive offset

### Ask First

- Adding a configurable day boundary (for example a 04:00 rollover) or a stored time-zone setting

### Never

- Rewind a stored day or `last_activity`
- Migrate or rewrite stored dates on upgrade (FR-006)

## 7. See Also

- [[constitution]] - project principles
- [[arcade-gamification-session-fixes/spec]] - streak and freeze rules this package amends (FR-007, negative-gap row)
- [[challenge-attempt-limit-not-enforced/spec]] - daily challenge attempts
- [[MOC-specs]] - all specifications
