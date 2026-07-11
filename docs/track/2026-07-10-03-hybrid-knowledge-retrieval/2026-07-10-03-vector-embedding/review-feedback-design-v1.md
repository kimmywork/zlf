# Review Feedback v1: Stage 03 Vector/Embedding Design

## Scope

Cumulative review of parent artifacts, Stage 01 lifecycle design, and Stage 03 requirements/design/plan. No independent reviewer subagent was available; this is a separate self-review against current exact-vector and embedding queue limitations.

## Findings

### V-D1
- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** Source-node lookup becomes ambiguous when multiple chunks/models exist.
- **Evidence:** Stage 03 explicitly replaces one-vector-per-node storage.
- **Suggested fix:** resolve source similarity through explicit matching indexed documents/model profile; never choose an arbitrary vector.
- **Resolution:** fixed in query integration.

### V-D2
- **Origin phase:** solution design
- **Severity:** major
- **Type:** scope
- **Description:** Choosing an ANN crate before Recall/reopen/resource evidence would repeat parent finding D3.
- **Suggested fix:** exact canonical oracle, common ANN contract, benchmark, reviewed change note.
- **Resolution:** fixed in ANN decision and plan V3.

### V-D3
- **Origin phase:** solution design
- **Severity:** minor
- **Type:** correctness
- **Description:** Cosine zero-vector behavior must not produce NaN or silent filtering.
- **Suggested fix:** reject zero vectors for cosine at ingestion and test independently.
- **Resolution:** fixed in exact-store design.

## Decision

**Pass.** No unresolved critical/major findings. V0 contract work may follow Stage 01 shared contracts; ANN selection remains evidence-gated and exact search is the rollback path.
