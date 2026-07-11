---
status: in_progress
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
source_requirements:
  - docs/track/2026-07-10-03-hybrid-knowledge-retrieval/2026-07-10-04-temporal/requirements-v1.md
---

# Stage 04 Solution Design v1: Temporal Semantics and Indexes

## Scope and dependency

Stage 04 consumes Stage 01 document/profile/generation lifecycle. It replaces creation-date prototype scans with distinct event and valid-time records. Storage transaction history is not exposed as bitemporal data.

## Semantics

```text
Event { id, document_id, at }
Validity { id, document_id, from, to? } // [from,to)
```

Instants are UTC signed microseconds. Date-only event input is a UTC calendar day. Query ranges are half-open and must satisfy `start < end`; empty/reversed ranges return typed validation errors. Open validity end is positive infinity. `valid_at(t)` means `from <= t && (to is none || t < to)`. Overlap means `record.from < query.end && (record.to is none || record.to > query.start)`.

Before/after APIs are namespaced by event versus validity boundary and are not overloaded into ambiguous existing predicates.

## Encoding and indexes

Signed microseconds are transformed by flipping the sign bit and encoded big-endian, preserving chronological byte order. Record/document IDs use canonical length-prefixed bytes.

Generation key families:

```text
event/by-time/<instant>/<record-id>
event/by-entity/<document-id>/<record-id>
valid/by-start/<from>/<record-id>
valid/by-end/<finite-end>/<record-id>
valid/open-end/<record-id>
valid/by-entity/<document-id>/<record-id>
```

Event point/range queries seek directly from encoded start to end. Validity queries choose the smaller estimated endpoint side using generation counts/histograms, intersect as needed, and report scanned candidates. Open ends are merged explicitly. Normal bound queries never start an unbounded full-DB iterator.

Coarse buckets or an interval-tree derivative are deferred. They may be added only if skew/open-interval evidence violates accepted budgets, through a reviewed change note.

## Lifecycle and provenance

Profiles explicitly mark event or validity fields; arbitrary strings are never auto-parsed. One field may produce multiple stable temporal records. Stage 01 source versions/manifests reconcile superseded records and deletion. Generations include temporal schema, parser, timezone policy, snapshot sequence, counts, and checksum.

Hits return record/document/entity identity, kind, instant or interval, generation/watermark, and access-path/candidate counts. Existing `temporal_on/2` and `temporal_between/3` retain names but adopt the approved UTC half-open event semantics. New `valid_at/2` and `valid_overlaps/3` never return event records.

## Parsing

Rust/JSON accepts RFC3339 instants and ISO dates; offsets are converted to UTC. Prolog literal contracts are versioned and reject missing/ambiguous timezone forms except date-only UTC days. DST behavior is tested through explicit offset inputs, not host-local timezone state.

## Verification

- Independent straightforward interval filter oracle.
- Ordered-encoding tests around negative/positive epoch, microsecond bounds, leap days, offsets, and extremes.
- Boundary matrix: equal endpoints, adjacent intervals, open end, duplicate timestamps, empty ranges.
- Update/delete/replay/rebuild/reopen and generation incompatibility tests.
- 1K/10K/100K uniform, skewed, and long-open distributions recording candidates, latency percentiles, QPS, RSS, and disk.
- WAM joins and planner access-path tests are completed with Stage 05.

## Risks and rollback

- **Skewed interval candidates:** measure endpoint scans; add a derived acceleration structure only with evidence.
- **Timestamp overflow/parsing drift:** checked conversion and versioned parser metadata fail closed.
- **Semantic replacement:** prototype temporal data is discarded. Rollback applies only between validated generations created by the new implementation.
