---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Code indexing benchmark stage

Validate extraction, lifecycle, lexical relevance, graph composition, and scale on reproducible public/generated repositories without requiring vector embedding.

## Acceptance

- Frozen repository revisions, licenses, manifests, and checksums are recorded.
- Independent source-range and relationship fixtures measure extraction correctness.
- Initial build, incremental change/delete, reopen, and full rebuild equivalence are verified.
- Parse/index/query latency, throughput, RSS, disk, stale-result count, and bounded query quality are reported.
- Public evidence is separated from generated correctness fixtures and does not overclaim compiler precision.
