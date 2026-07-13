---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
decision: event time plus valid-time half-open intervals; temporal_* for events and valid_* for validity; full bitemporal deferred
---

# Stage 04 Requirements: Temporal Semantics and Indexes

## Goal

Replace creation-date scans with explicitly defined temporal data and ordered indexes suitable for enterprise records, agent memories, and Prolog joins.

## Confirmed semantic model

The first delivery supports two distinct temporal record kinds:

1. **Event time** — a UTC instant when an event occurred.
2. **Valid time** — a half-open UTC interval `[valid_from, valid_to)` during which knowledge is considered true; `valid_to = none` means open-ended.

Storage versions remain internal transaction history. Full bitemporal query algebra is deferred. Event and validity records use distinct identities and physical key families so a query cannot silently mix them.

## Requirements

- Use UTC instants internally with explicit parsing/time-zone behavior; date-only input has a documented zone and boundary conversion.
- Define open-ended intervals and point/range inclusivity precisely.
- `temporal_on(Date, Node)` finds events in the UTC day `[DateT00:00:00Z, next-dayT00:00:00Z)`.
- `temporal_between(Start, End, Node)` finds event instants in the half-open range `[Start, End)`.
- `valid_at(Instant, Node)` finds validity intervals containing `Instant`.
- `valid_overlaps(Start, End, Node)` finds validity intervals overlapping `[Start, End)`.
- Before/after support is defined separately for event instants and interval boundaries.
- Use ordered key encoding and bounded prefix/range seeks at target scale; do not scan the complete temporal database for normal bound queries.
- Preserve multiple temporal records per node/field rather than keying only by node and start date.
- Update/delete/supersede behavior follows Stage 01 source versions and generations.
- Expose temporal result provenance and planner access path.
- Temporal predicates compose with graph, text, vector, rules, cut, and optional tabling under documented mutation semantics.

## Verification

- Independent interval-filter oracle covers equal endpoints, empty intervals, open ends, leap days, time zones, DST input conversion, duplicate timestamps, update/delete, and restart.
- Scale reports candidate counts, p50/p95/p99, throughput, peak RSS, and index size for point and overlap distributions.
- Include skewed histories and long-lived intervals, not only uniformly random timestamps.

## Non-goals

- Full bitemporal algebra, temporal logic theorem proving, Allen algebra in full, or automatic interpretation of arbitrary string properties as dates.

The predicate split and half-open boundary semantics were confirmed by the product owner on 2026-07-10.
