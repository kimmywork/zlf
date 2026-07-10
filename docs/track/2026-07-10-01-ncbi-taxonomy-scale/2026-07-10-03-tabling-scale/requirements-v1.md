---
status: done
scope_type: stage
created: 2026-07-10
parent_id: 2026-07-10-01-ncbi-taxonomy-scale
version: 1
---

# Tabling Scale Stage

Optimize the existing deterministic positive tabling MVP for bound variant calls over large indexed graphs. Scope includes recursive-component selection, query seeding, delta answer propagation, bind-aware provider access, long-lived hot-table ownership, resource limits, and metrics. Negation/WFS/direct continuation suspension remain excluded. Acceptance includes cycles, left recursion, duplicate paths, variant reuse, bounded storage reads, and oracle-equivalent taxonomy results.
