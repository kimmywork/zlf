# Stage 06 implementation progress v4

## Increment

S1b executes the 1K combined mutation, embedding retry, watermark, generation rollback, and reopen lifecycle.

## Delivered

- Added a release lifecycle benchmark using the generated 1K mutation set.
- Activated real BM25/vector/temporal profile lifecycle over bulk-loaded graph data.
- Injected one retryable embedding failure, then recovered all document embeddings.
- Applied 10 revisions, five deletes, and five inserts.
- Waited for BM25/vector/temporal minimum watermarks and verified a bounded missing-target timeout.
- Rebuilt and rolled back the BM25 generation.
- Reopened the database and rechecked every mutation.

## Findings and fixes

1. Bulk packs did not persist `EntityState`. Every initial embedding job was therefore classified stale. Canonical bulk node plans now include typed entity state, and the loader explicitly permits only the generated `entity-state:` key family.
2. A stale-only claimed embedding batch returned zero while ready jobs remained, causing callers to stop. The worker now drains stale-only batches until it either finds current work or no ready jobs remain.

Both findings have focused regression coverage.

## Result

- Initial embeddings published: 1,000.
- Injected retry jobs: 32; final retry/dead/stale counts: zero.
- Mutation embeddings published: 15.
- Mutation oracle: 20/20 before reopen and 20/20 after reopen.
- Watermark reached for BM25/vector/temporal; missing target timed out explicitly.
- BM25 rebuild completed and rollback restored the initial generation pointer.
- Reopened BM25 returned the expected bounded sample.

## Evidence

- `research/enterprise-kb-s1-lifecycle-1k-2026-07-14.json`

## Remaining S1

Add an explicit stale-job fixture to the lifecycle report and decide whether the 150-second profile activation baseline needs bulk profile-rebuild optimization before Stage 06 final acceptance. Functional S1 mutation/retry/reopen/watermark/generation correctness is established.
