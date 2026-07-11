---
status: done
scope_type: stage
parent_id: 2026-07-10-03-hybrid-knowledge-retrieval
created: 2026-07-11
version: 1
---

# Stage 01 Delivery Record v1: Index Identity and Lifecycle

## Decision

**Delivered.** Stage 01 lifecycle foundations are accepted. This does not mark the parent hybrid retrieval track delivered.

## Accepted capabilities

- Atomic canonical node/edge mutation sequence, entity state/tombstones, durable outbox, cascade ordering, and strict first-version schema.
- Equivalent node/edge property patching across Rust, JSON, Prolog, generic property writes, stable edge IDs, and selective table invalidation.
- Resumable bulk sessions with atomic checkpoint and one rebuild-required marker.
- Content-addressed immutable profiles, JSON/Prolog activation, deterministic baseline chunkers, fingerprints, ranges, and durable manifests.
- Durable target jobs with leases/retry/dead/stale handling, contiguous watermarks, metrics, safe compaction, and crash-idempotent fake target.
- Validated generation build/checkpoint/activation/rollback, status, wait timeout, reopen, and retention.

## Acceptance evidence

- Insert, update, property removal, edge update, node/edge delete, cascade, and replay converge to exact fake indexed documents.
- Old events cannot resurrect newer content; stale jobs are explicitly counted.
- Failure after target write recovers idempotently; retry limits and permanent dead letters block publication.
- Failed generation validation leaves the previous active generation readable; activation is one atomic configuration commit.
- Reopen preserves outbox, jobs, sessions, manifests, profiles, generations, and watermarks.
- Raw graph/lifecycle key bypass is rejected; bulk loading uses the explicit rebuild marker.

## Fresh verification

- `cargo fmt --all -- --check` → pass.
- `python3 scripts/check-rust-size.py` → pass.
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines` → pass.
- `RUST_TEST_THREADS=1 cargo test --workspace` → no failures through CLI/core/index/prolog/storage and into final query suite before the 300-second harness limit.
- `cargo test -p zlf-query` immediately after timeout → all remaining coordinator/generation/manifest/profile/kernel/planner/table tests pass.
- Ollama and wiki tests remain intentionally ignored because they require local external services/data and are not lifecycle acceptance dependencies.
- `git diff --check` → pass.

## Rollback

All delivered index state is projection/generation scoped. Primary graph commits are never rolled back for indexing failure. Backend consumers can be disabled, prior validated generations remain available, and outbox/jobs retain replay state.

## Next

Begin Stage 02 BM25 with the approved function-first Tantivy baseline, using these lifecycle contracts rather than the prototype direct-write path.
