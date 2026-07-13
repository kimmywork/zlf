# Review Feedback: Stage 04 Temporal Implementation v1

## Decision

**Accept.** Stage 04 meets its semantic, lifecycle, index, provider, provenance, and focused-scale requirements. No blocking or high-severity findings remain.

## Cumulative findings

### Semantics and contracts

- Event instants and validity intervals are distinct record types and key spaces.
- UTC signed microseconds, explicit-offset/date parsing, UTC-day conversion, half-open ranges, open ends, leap days, DST offsets, duplicates, and integer extremes have independent oracle coverage.
- Empty validity/query intervals are rejected; adjacency does not produce false overlap.

### Physical indexes and query behavior

- Generation/time, generation/start, generation/end, open-end, and document keys use ordered binary encoding and bounded seeks.
- Validity queries select a start/end endpoint from persisted generation statistics and merge open records.
- Candidate counts and access-path provenance are exposed; bounded result heaps avoid unbounded result accumulation.
- Atomic batch replacement deletes old keys before writing the same record identity and refreshes validity statistics once per affected generation.

### Lifecycle and runtime

- Canonical graph mutation and durable outbox processing own temporal publication.
- Profile-declared scalar/array extraction, manifests, source versions, generations, update/delete/replay, reopen, and profile retirement are covered.
- The former creation-date scan prototype was removed rather than retained as a second runtime path.
- WAM exposes separate event and validity predicates through the shared read-only provider architecture; planner explain identifies the corresponding physical access class.

### Verification and scale

- Differential 1K/10K/100K uniform, skewed, and long-open runs match independent oracles.
- Worst measured 100K p99 is 61.93 ms; peak RSS is 531.8 MB and per-distribution disk remains below 97 MB.
- Full workspace tests, clippy with warnings denied and `too_many_lines`, formatting, source-size policy, and diff hygiene pass.

## Non-blocking follow-up

- Stage 05 owns cumulative graph/rule/text/vector/time composition, top-k behavior, cut/tabling interaction, and bounded materialization contracts.
- Stage 06 should rerun frozen temporal budgets in the combined KB stress harness and distinguish true full-result costs from selective-query regressions.
- Add a bucket or interval-tree derivative only if broader workload evidence violates frozen candidate/latency budgets; current evidence does not justify that complexity.
