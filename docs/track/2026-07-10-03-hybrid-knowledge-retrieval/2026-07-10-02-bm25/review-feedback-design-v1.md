# Review Feedback v1: Stage 02 BM25 Design

## Scope

Cumulative review of parent artifacts, Stage 01 lifecycle design, and Stage 02 requirements/design/plan. No independent reviewer subagent was available; this is a separate self-review against current `zlf-index` behavior and acceptance criteria.

## Findings

### B-D1
- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** Corpus statistics are ambiguous unless the logical document and partition are fixed.
- **Evidence:** requirements explicitly require this decision; Stage 01 permits fields/chunks.
- **Suggested fix:** define one field chunk as the document and partition stats by compatible generation/analyzer/field.
- **Resolution:** fixed in the design scope and scoring contract.

### B-D2
- **Origin phase:** solution design
- **Severity:** major
- **Type:** scope
- **Description:** Prematurely selecting a text engine would violate the required comparative spike.
- **Evidence:** parent review D3 and Stage 02 design decision.
- **Suggested fix:** common contract, bounded 10K evidence, reviewed selection change note.
- **Resolution:** fixed in design and plan B1.

### B-D3
- **Origin phase:** solution design
- **Severity:** minor
- **Type:** unclear
- **Description:** A candidate budget can make top-k inexact.
- **Suggested fix:** return explicit truncation/exactness metadata rather than silently claiming exact results.
- **Resolution:** fixed in lifecycle/bounded-search design.

## Decision

**Pass.** No unresolved critical/major findings. Implementation is dependency-gated on Stage 01; B0 oracle work may proceed once shared identity contracts compile. Backend selection remains intentionally open, not a blocker to contract/oracle implementation.
