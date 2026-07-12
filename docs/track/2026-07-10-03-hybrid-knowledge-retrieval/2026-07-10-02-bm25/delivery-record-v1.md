# Delivery Record v1: BM25 Correctness and Scale

## Summary

Stage 02 replaces the prototype token-count path with a real Tantivy BM25 backend. Chunks are logical documents with canonical entity/field/chunk/language identity. The implementation provides versioned multilingual analysis, bounded filtered top-k, weights and structured explanations, exact replace/delete batches, durable outbox/manifest convergence, physical generation validation/activation/rollback/reopen, and a reproducible 1K/10K local baseline.

## Source Artifacts

- Requirements: `requirements-v1.md`
- Solution design: `solution-design-v1.md`
- Plan: `plan-v1.md`
- Change notes:
  - parent `change-note-v1-function-first.md`
  - parent `change-note-v2-pre-release-schema.md`
  - parent `change-note-v3-tantivy-bm25-parameters.md`

## Changed Areas

- `crates/zlf-index`: lexical contracts/oracle, Tantivy schema/scoring/filtering/explanations, batch mutation, benchmark.
- `crates/zlf-query`: lifecycle target, target-scoped manifests, synchronous facade convergence, generation build/publication/rollback/reopen.
- `crates/zlf-prolog` / `crates/zlf-cli`: read provider remains on WAM; direct production BM25 write bypasses removed.
- Stage documentation: progress records, cumulative review, local JSON/Markdown evidence.

## Acceptance Results

| Requirement | Result | Evidence |
|---|---|---|
| Real corpus BM25 statistics/formula | pass | `bm25_oracle.rs`; production differential fixture in `bm25_backend.rs` |
| Versioned `k1`/`b` | pass with approved narrowing | change note v3 pins and validates Tantivy 1.2/0.75 |
| Bounded top-k and field/language filters | pass | `search_document_top_k_filtered`; backend tests |
| Chunk-as-document identity | pass | canonical `IndexDocumentId`; field/chunk lifecycle tests |
| Chinese/English deterministic analysis | pass | analyzer goldens and WAM provider test |
| Exact replace/update/delete | pass | batch backend, manifest, lifecycle, stale/replay tests |
| Stable tie-break | pass | canonical ID tie test |
| Score diagnostics | pass | structured TF/DF/IDF/length/weight assertions |
| No prototype production write path | pass | cumulative review; direct CLI/writer bypass removed |
| Generation validation/activation/rollback/reopen | pass | `bm25_lifecycle.rs`, `index_generations.rs` |
| 1K/10K local evidence | pass | `research/bm25-local-2026-07-11.{json,md}` |
| Public quality / true cold benchmark | approved Stage 06 deferral | function-first change note and local report limitations |

## Verification Evidence

Fresh acceptance run on 2026-07-11:

- `cargo fmt --all -- --check` — pass
- `python3 scripts/check-rust-size.py` — pass
- `git diff --check` — pass
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines` — pass
- `cargo test --workspace` — pass; only external Ollama/wiki tests remain intentionally ignored
- Release benchmark 1K and 10K — pass against frozen budgets; machine-readable evidence checked in

The workspace test run included CLI subprocess integration, storage lifecycle, query lifecycle/generations/profiles, Prolog WAM providers, backend/oracle/chunk/manifest tests, and doc tests. Slow CLI tests emitted only duration notices and completed successfully.

## Review Results

### Spec Fit

pass

All functional requirements are covered. Parameter configurability and public benchmark scope changes are recorded in approved change notes rather than silently omitted.

### Format Fit (software)

pass

Code follows the WAM/provider and Stage 01 lifecycle architecture, adds only required dependencies, has explicit errors/schema checks, remains within source-size policy, and carries automated and manual benchmark evidence.

## Known Risks

- Tantivy 0.22 supports only the pinned BM25 constants; adding arbitrary values requires a verified custom scorer, not approximate reranking.
- The facade currently performs synchronous coordinator catch-up for correctness. Background scheduling and throughput tuning can follow after all retrieval modes converge.
- Local quality fixtures are deliberately synthetic; general-quality claims require Stage 06 datasets.

## Follow-ups

- Stage 03 exact vector/embedding implementation.
- Stage 04 ordered temporal implementation.
- Stage 05 bounded hybrid WAM composition.
- Stage 06 public quality, true cold-process, and larger stress orchestration.

## Final Status

delivered
