# Review Feedback v1: Stage 05 Hybrid Prolog Design

## Scope

Cumulative review of parent artifacts, Stage 01–04 designs, and Stage 05 requirements/design/plan. No independent reviewer subagent was available; this is a separate self-review against current materialized provider choice points and WAM architecture.

## Findings

### H-D1
- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** A cursor borrowing a temporary provider cannot safely survive WAM backtracking.
- **Evidence:** current providers are stack-scoped during `execute_terms`; choice points outlive individual calls.
- **Suggested fix:** cursor owns request-scoped state/identity and is closed on cut/error/drop; prove with fake provider before adapting indexes.
- **Resolution:** fixed in cursor design and hard gate H2.

### H-D2
- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** Applying top-k before ACL/graph filtering can omit valid high-ranked results.
- **Suggested fix:** filter-first when selective; otherwise progressive paging to accepted top-k or reported exhaustion/budget.
- **Resolution:** fixed in filtering design and H4.

### H-D3
- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** Tabled retrieval can become indefinitely stale if generation/watermark is absent from dependencies.
- **Suggested fix:** include exact dependencies or reject as non-tableable.
- **Resolution:** fixed in proof/tabling design and H5.

### H-D4
- **Origin phase:** planning
- **Severity:** minor
- **Type:** scope
- **Description:** Stage 05 cannot begin provider integration merely because designs exist.
- **Suggested fix:** dependency-gate H0–H7 on validated Stage 01–04 contracts/backends.
- **Resolution:** explicit in scope and plan graph.

## Decision

**Pass.** No unresolved critical/major findings. The design is executable when dependencies pass; no remote embedding or second evaluator is introduced.
