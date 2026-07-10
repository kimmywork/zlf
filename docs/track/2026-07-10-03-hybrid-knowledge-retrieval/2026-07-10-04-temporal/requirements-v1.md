---
status: blocked
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-10
version: 1
blocked_on: first-delivery temporal domain model
---

# Stage 04 Requirements: Temporal Semantics and Indexes

## Goal

Replace creation-date scans with explicitly defined temporal data and ordered indexes suitable for enterprise records, agent memories, and Prolog joins.

## Required semantic decision

Choose the first-class model before design:

1. **Event time** — a point when an event occurred.
2. **Valid time** — half-open interval `[valid_from, valid_to)` during which knowledge is considered true.
3. **Transaction time** — when zlf stored/superseded the record.
4. **Bitemporal** — valid time and transaction time together.

Recommended starting point: support event timestamps and valid-time half-open intervals as distinct record kinds; retain storage versions for transaction-history queries but postpone full bitemporal algebra unless required.

## Requirements after decision

- Use UTC instants internally with explicit parsing/time-zone behavior; date-only input has a documented zone and boundary conversion.
- Define open-ended intervals and point/range inclusivity precisely.
- Support point containment, interval overlap, before, after, and bounded range retrieval required by approved scenarios.
- Use ordered key encoding and bounded prefix/range seeks at target scale; do not scan the complete temporal database for normal bound queries.
- Preserve multiple temporal records per node/field rather than keying only by node and start date.
- Update/delete/supersede behavior follows Stage 01 source versions and generations.
- Expose temporal result provenance and planner access path.
- Temporal predicates compose with graph, text, vector, rules, cut, and optional tabling under documented mutation semantics.

## Verification

- Independent interval-filter oracle covers equal endpoints, empty intervals, open ends, leap days, time zones, DST input conversion, duplicate timestamps, update/delete, and restart.
- Scale reports candidate counts, p50/p95/p99, throughput, peak RSS, and index size for point and overlap distributions.
- Include skewed histories and long-lived intervals, not only uniformly random timestamps.

## Non-goals until approved

- Temporal logic theorem proving, Allen algebra in full, or automatic interpretation of arbitrary string properties as dates.
