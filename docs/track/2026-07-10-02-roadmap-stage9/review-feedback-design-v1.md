# Review Feedback v1: Roadmap Stage 9 Parent Design

## Review scope

Cumulative review of:

- `requirements-v1.md`
- `scope-map.md`
- `solution-design-v1.md`
- `plan-v1.md`
- source roadmap and predecessor Stage 0–8 delivery record

## Findings

### Blocking

None for creating the parent track.

### Major observations

1. **The scope is intentionally too broad for one implementation change.** The plan correctly requires independently approved child tracks for every slice and identifies S0/S1 as the only immediate work. This boundary must be enforced.
2. **WFS cannot be represented as another positive-table state.** The design correctly requires fuller SLG suspension/delayed-literal work and a separately versioned three-valued table format.
3. **Delete-delta maintenance is substantially harder than stale recomputation.** Support/provenance metadata and mandatory fallback are required; performance targets must not weaken correctness.
4. **The roadmap's virtual-address prescription is not currently justified.** Existing typed call-time providers already provide lazy indexed loading without persisting process-local pointers. The benchmark gate and rollback requirement are appropriate.
5. **Mode inference can become unsound across goals.** Child S2 must specify conservative transfer rules and tests proving that an `In` access path is never selected from a merely possible binding.

### Minor observations

- Child tracks must replace qualitative claims such as “zero overhead” and “bounded” with workload-specific thresholds.
- Probability needs duplicate-proof and independence semantics before APIs are frozen.
- The initial stratified-negation slice should choose structured errors over warnings for persisted non-stratified programs to avoid ambiguous execution.
- The 7x24-hour memory run should be preceded by short deterministic tiers and should record commit, allocator, dataset, limits, and peak RSS.

## Requirement coverage

| Area | Covered | Notes |
|---|---|---|
| Predecessor compatibility | yes | One WAM runtime, read-only providers, compiled rule artifacts retained |
| Stratified NAF | yes | Signed graph, SCC rejection, strata, mutation |
| Modes/pushdown | yes | Conservative inference and plan visibility |
| Memory/GC/virtual address | yes | Evidence-gated rather than mechanism-first |
| Delta tabling | yes | Insert/delete, support metadata, restart, fallback |
| Type/constraint modules | yes | Optional and isolated |
| Probability/MIL | yes | Proof/meta layer, bounded, review-gated |
| WFS | yes | Separate high-risk milestone |
| Research-only roadmap items | yes | Explicitly deferred |
| Verification | yes | Oracles, persistence, stress, workspace gates |

## Decision

**Pass for parent-track creation.** Proceed to S0/S1 requirement discovery only. Do not treat this review as approval to implement S2–S11 or to announce full Stage 9 completion.
