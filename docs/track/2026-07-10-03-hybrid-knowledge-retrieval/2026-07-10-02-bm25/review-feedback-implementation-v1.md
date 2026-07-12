# Review Feedback Report

## Metadata

- **Reviewer**: self-performed; no independent reviewer subagent was available
- **Phase reviewed**: Stage 02 BM25 cumulative implementation B0–B4
- **Artifacts inspected**: requirements/design/plan/change notes, `zlf-index` lexical/Tantivy code, query lifecycle/generation integration, Prolog/CLI surfaces, tests, benchmark report
- **Prior phases considered**: Stage 01 lifecycle delivery and Stage 02 design review
- **Review date**: 2026-07-11

## Summary

- **Total open issues**: 0
- **Critical**: 0
- **Major**: 0
- **Minor**: 0
- **Fix-in-place**: 0
- **Roll-back**: 0
- **Verdict**: pass

## Resolved during review

1. The old JSON/query `index_text` path and BM25 option on `IndexedStorageFactWriter` bypassed profiles, manifests, outbox ordering, and generations. They were removed; production graph writes now reach BM25 only through canonical storage mutation and `Bm25IndexTarget`. The low-level `BM25Index::index_text` remains only as a backend convenience for isolated provider tests.
2. Generation activation had no explicit rollback operation. `GenerationManager::rollback` and `ZlfDatabase::rollback_bm25_generation` now republish only validated retired generations, swap readers, replay current state, and are covered by reopen/lifecycle tests.
3. The accepted query contract mentioned optional language filtering but documents had no language identity. Optional profile/document language metadata, schema storage, filtered backend/query APIs, and tests were added.

## Accuracy pass

- Tantivy 0.22 uses the documented fixed defaults (`k1=1.2`, `b=0.75`); unsupported values are rejected under change note v3 rather than ignored.
- Production scores are compared to the independent formula on an equal-length corpus; structured explanations expose TF/DF/IDF/length/weight components.
- Benchmark claims match the checked-in machine-readable report and explicitly avoid general-quality or true cold-cache claims.

## Validity pass

- Canonical mutation → outbox → target → backend → manifest ordering is replay-safe: the manifest is saved only after backend mutation succeeds.
- Generation builds occur in separate directories and only validated generations become active; failures before activation preserve the previous pointer.
- Bounded top-k collection and deterministic canonical-ID tie-breaking are retained with field/language filters and weights.

## Consistency pass

- No compatibility alias or second query runtime was introduced.
- BM25 profile/analyzer/backend schema versions are explicit and incompatible schemas fail open.
- Stage 01 identities, manifests, coordinator, waits, and generation metadata remain the shared lifecycle path.

## Positive observations

- Backend, lifecycle, generation, Prolog-provider, CLI, workspace, analyzer, oracle, and local-scale evidence are cumulative.
- Source files remain within repository size policy and responsibilities are split across backend support, target, runtime, and generation rollback modules.

## Open questions

None for Stage 02 acceptance. Public-dataset quality and true OS-cold benchmarks remain explicitly deferred to Stage 06 by the function-first change note.
