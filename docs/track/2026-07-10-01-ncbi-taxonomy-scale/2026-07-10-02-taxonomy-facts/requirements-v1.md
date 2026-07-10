---
status: done
scope_type: stage
created: 2026-07-10
parent_id: 2026-07-10-01-ncbi-taxonomy-scale
version: 1
---

# Taxonomy Facts Stage

Deliver a deterministic streaming converter from the local NCBI dump into sharded canonical ground facts, with taxon nodes, grouped names, parent edges, merged IDs, and deleted IDs. Acceptance includes source checksums/counts, bounded memory, fixture snapshots, and query-visible taxonomy metadata. Taxonomy distance is tree-edge distance through LCA.
