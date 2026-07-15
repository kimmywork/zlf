# Implementation review v1

## Findings

No critical or major findings.

- HNSW is a strategy, not a replacement; exact remains authoritative.
- Disabled behavior is explicit and tested rather than silently degrading hybrid requests.
- Background rebuild never holds query locks; requests coalesce and stale ANN falls back to exact.
- Publication identity is checked against current exact records on reopen; missing/corrupt dumps fall back.
- Native `hnsw_rs` dump lifetime is isolated behind a documented ownership assumption with mmap disabled and crate version pinned.

## Decision

Passed. The optional vector strategy increment is accepted for delivery.
