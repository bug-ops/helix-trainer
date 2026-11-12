---
aliases:
  - Review Event Log
  - Per-review event capture
tags:
  - sdd
  - spec
  - learning-scheduler
  - fsrs
  - persistence
  - status/proposed
created: 2026-11-12
status: proposed
related:
  - "[[constitution]]"
  - "[[fsrs-7-model-adoption/spec]]"
  - "[[fsrs-review-rating-and-session/spec]]"
  - "[[MOC-specs]]"
---

# Feature: Local Review-Event Log for a Real FSRS Evaluation Corpus

> [!info] Metadata
> **Source**: #627 (design), follow-up of the DEFER record in [[fsrs-7-model-adoption/spec]] section 11
> **Status**: Proposed (design complete, not implemented)
> **Depth**: Lightweight spec. This package changes shipped code and adds a persisted file, which section 8 of [[fsrs-7-model-adoption/spec]] forbids for the evaluation itself; capture is therefore specified here, not there.

## 1. Overview

### Problem Statement

`profile.json` stores the current FSRS state per command, not the history of reviews, so the FSRS-6 versus FSRS-7 evaluation has no real corpus and existing history cannot be reconstructed. Every production attempt (review answer in `src/learning/session.rs`, scenario commands in `src/learning/scheduler.rs`) goes through one chokepoint, `PerformanceTracker::try_record_attempt` (`src/learning/performance.rs`), so a per-review event can be captured at a single hook point.

### Goal

Capture one small, model-neutral event per recorded attempt into a local append-only file, so that a later offline run can replay it under both models. The application never reads the log back; only a development example does.

### Out of Scope

- Reading the log in the application (startup cost stays unchanged).
- A settings toggle to disable capture (needs user approval; the design ships capture always on, local only).
- Volunteer submission or pooling of logs across users (conflicts with the offline, no-transmission rule).
- Reconstructing history from before capture existed.
- The evaluation itself and its protocol (defined in [[fsrs-7-model-adoption/spec]] section 11).

## 2. User Stories

### US-001: Maintainer collects a real corpus

AS A maintainer
I WANT every practice attempt recorded locally with its time and rating
SO THAT the FSRS-7 adoption decision can be re-run on real data

**Acceptance criteria:**
```
GIVEN a writable profile
WHEN  an attempt is recorded for a command through either call site
THEN  exactly one line with at, command and rating is appended to review_log.jsonl
```
```
GIVEN a read-only second instance (ProfilePersistence::is_writable() is false)
WHEN  attempts are recorded
THEN  nothing is written to review_log.jsonl
```

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | WHEN `PerformanceTracker::try_record_attempt` runs THE tracker SHALL create one event from the same `now` and `rating` it already computes, BEFORE the FSRS step, so an FSRS error still logs the attempt. The event holds `at` (UTC, truncated to whole seconds), `command` (`CommandId`) and `rating` (`Rating`). Recall is derived (`rating != Again`, guaranteed by `Rating::for_attempt`) and never stored | must |
| FR-002 | THE event SHALL contain nothing model-derived (no elapsed time, stability, difficulty or interval), so the log serves both FSRS-6 and FSRS-7 replay. Duration, reference time, scenario id and source are NOT stored (privacy and minimalism; the rating already encodes duration under the FR-003 mapping of [[fsrs-review-rating-and-session/spec]]) | must |
| FR-003 | THE type `learning::review_log::ReviewEvent { at, command, rating }` SHALL have private fields with getters, a `pub(crate)` constructor used only by the tracker, and a public `Deserialize` so the evaluation example parses the same type (no mirrored struct). `Rating` gains `Serialize` and `Deserialize` with `snake_case` names (`again`, `hard`, `good`, `easy`) | must |
| FR-004 | A pure parser `parse_review_log(BufRead) -> ParsedReviewLog { events, malformed }` SHALL live beside the type (no I/O in the `learning` layer). A malformed line, including a torn last line after a crash, is skipped and counted | must |
| FR-005 | THE tracker SHALL keep `pending: Vec<ReviewEvent>` (not serialized into `profile.json`) capped at 1,024 events; on overflow the event is dropped and a counter is logged once. The buffer is drained where the profile save takes its snapshot (`save_immediate`, `save_debounced` in `src/ui/state/substates.rs`) and handed to the same save job | must |
| FR-006 | A `ReviewLogStorage`, beside `ProfileStorage` in `src/gamification/storage.rs`, SHALL append JSON Lines (one object per line, `OpenOptions::append`, never rewritten) to `<data_dir>/review_log.jsonl` obtained through `DataDir::file`. Nothing is written when `ProfilePersistence::is_writable()` is false | must |
| FR-007 | THE file size SHALL be bounded by `MAX_REVIEW_LOG_FILE_SIZE = 4 MiB` in `src/limits.rs` (about 70,000 events at roughly 60 bytes per line). Before an append, if `metadata.len() + batch_bytes` exceeds the cap, the batch is dropped and a warning is logged once per session; capture stops and the file is never truncated or rotated, because the oldest history is the most valuable for multi-day events. The parser reads through the existing capped reader with the same cap | must |
| FR-008 | THE log SHALL stay local, in the same directory as `profile.json`, and never be transmitted. It holds command ids, second-resolution timestamps and ratings, and no editor text, keystrokes or durations. It is independent of the profile lifecycle (`start_fresh` and quarantine do not touch it); deleting it is always safe (an absent file is an empty log). The README data section documents it | must |
| FR-009 | THE line format SHALL carry no version field; a breaking change to the format uses a new file name | should |
| FR-010 | THE application SHALL NOT read `review_log.jsonl` at startup or at any other time; only the evaluation example reads it | must |

## 4. Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Startup | No log read at startup, so the startup cost is unchanged |
| NFR-002 | Privacy | Local file only, never transmitted (FR-008) |
| NFR-003 | Layering | `learning` (layer 4) defines the event and parser without I/O; `gamification` (layer 5) owns the storage and may depend on `learning` |
| NFR-004 | Type safety | `ReviewEvent` cannot be constructed outside the tracker; `Rating` and `CommandId` are typed, not strings or integers |

## 5. Edge Cases and Error Handling

| Scenario | Expected Behavior |
|----------|-------------------|
| Crash during an append | The last line may be torn; the parser skips and counts it (FR-004) |
| Log file absent | Treated as an empty log |
| File reaches the cap | Batches are dropped with one warning per session; the file is left as is (FR-007) |
| More than 1,024 events between saves | The overflow events are dropped and counted (FR-005) |
| FSRS step returns an error | The attempt is still logged (FR-001) |
| Read-only second instance | No write (FR-006) |

## 6. Success Criteria

| ID | Metric | Target |
|----|--------|--------|
| SC-001 | One log line per attempt from both call sites (review answer and scenario commands) | 1:1 |
| SC-002 | Nothing written in read-only mode | 0 bytes |
| SC-003 | Cap respected, no truncation | tested |
| SC-004 | Torn last line skipped and counted | tested |
| SC-005 | `Rating` serde round trip for all four values | tested |
| SC-006 | No startup read of the log | verified in code review |

## 7. Agent Boundaries

### Always (without asking)
- Take the event at the single tracker chokepoint, before the FSRS step.
- Keep the log model-neutral and local.

### Ask First
- Any settings toggle for capture, or any change to the stored fields.
- Using the maintainer's log as an evaluation corpus (rule of section 8 of [[fsrs-7-model-adoption/spec]]).

### Never
- Transmit, pool or upload the log.
- Store editor text, keystrokes, durations or model-derived values in it.
- Rewrite, truncate or rotate the file.

## 8. Open Questions

> [!question] Unresolved
> - None for the design. Implementation is #638, the evaluation example change (`--log` mode, whole-day verdict form, fixed-seed bootstrap) is #639, and the re-run decision on a real corpus is #640.

## 9. See Also

- [[constitution]] — project principles
- [[MOC-specs]] — all specifications
- [[fsrs-7-model-adoption/spec]] — decision record and re-run protocol this log feeds
- [[fsrs-review-rating-and-session/spec]] — attempt to rating mapping
