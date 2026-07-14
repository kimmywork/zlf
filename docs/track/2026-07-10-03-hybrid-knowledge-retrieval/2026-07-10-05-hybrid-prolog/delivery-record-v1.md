# Delivery Record: Stage 05 Hybrid Retrieval and Prolog Composition v1

## Outcome

**Accepted and delivered on 2026-07-14.**

Stage 05 composes Tantivy BM25, exact model-safe vectors, temporal indexes, graph/property/label predicates, and persisted Prolog rules through one bounded prepared retrieval path in the existing WAM architecture.

## Delivered capabilities

- Structured lexical/vector/hybrid requests and hits with stable identity, provenance, generation/watermark, source range, strategy, exactness, and exhaustion metadata.
- Explicit finite top-k, candidate, page, page-count, and answer budgets.
- Fixed rank-only RRF baseline with `k=60`, deterministic ties, and no raw-score addition.
- Async pre-WAM query preparation, embedding, normalization, model/generation validation, and immutable process-local handles.
- No remote embedding calls in provider/WAM execution.
- Bounded BM25/vector/event/validity provider materialization with answer metrics and preserved backtracking, `once/1`, cut, and proof behavior.
- Stable ranked pages and backend bound-entity pushdown for Tantivy, exact vectors, event records, and validity records.
- Prepared `retrieve/4` with lexical, vector, and hybrid modes; source/field constraints; document/entity aggregation; temporal filters; and ordinary WAM graph/property/label/rule filtering.
- Compact retrieval-specific `ProofKind::Index` leaves.
- Safe retrieval tabling for bound prepared handle/options, explicit rejection of live/unbound combinations, and selective publication/generation/mutation invalidation.
- Per-target minimum-watermark waits with finite timeout and typed errors before WAM execution.
- Bound canonical `property/3` direct storage lookup, discovered and verified by the combined workload.
- Real public-judgment SciFact quality evidence and deterministic EnterpriseKB ACL/temporal/mutation scale evidence.

## Acceptance evidence

Passed on the delivery tree:

```bash
cargo fmt --all -- --check
python3 scripts/check-rust-size.py
git diff --check
cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines
cargo test --workspace
```

Focused retrieval contracts, preparation/execution, bounded provider, proof, tabling, storage provider, temporal, BM25, vector, and planner tests also pass.

### SciFact quality

On the deterministic 1,000-document/100-query subset with official qrels:

| retriever | MRR | nDCG@10 | Recall@10 | Recall@100 |
|---|---:|---:|---:|---:|
| BM25 | 0.816469 | 0.821813 | 0.880667 | 0.966667 |
| exact `bge-m3` | 0.760906 | 0.782273 | 0.881000 | 0.970000 |
| RRF hybrid | 0.801855 | 0.816503 | 0.904667 | 0.990000 |

Hybrid improves recall but not BM25 MRR/nDCG on this fixture. No general claim of hybrid superiority is made.

### EnterpriseKB composition

- 1K and 10K tiers each matched 128/128 independent filtered-order oracles.
- Stale-result count was zero and permission mutation invalidated table dependencies.
- Query p99 was 1.05 ms at 1K and 4.71 ms at 10K.
- Candidate/answer limits were 256/10; peak materialized answers remained 10.

Machine-readable and explanatory reports are under `research/scifact-h6-*` and `research/enterprise-kb-h6-*`.

## Deferred by scope

- WAM-owned provider cursor without evidence that bounded materialization is insufficient.
- ANN runtime integration; exact RocksDB remains production backend and oracle.
- Full public-dataset, multilingual, multi-hop, agent-memory, mutation/restart, and 100K combined orchestration, owned by Stage 06.
- Mandatory security/tenant isolation claims; ACL-style rules validate composition only.

## Commits

- `d7f6cf2` structured retrieval and RRF contracts.
- `5dccbce` async pre-WAM prepared retrieval.
- `f7c6ae7` bounded index provider answers.
- `b718dbf` bound retrieval paging and pushdown.
- `749e776` prepared hybrid execution and `retrieve/4`.
- `2207b96` retrieval proof, table dependencies, and freshness.
- `73314d3` deterministic SciFact preparation handoff.
- `434bbc8` SciFact quality baseline.
- `0873034` EnterpriseKB composition baseline and bound `property/3` pushdown.
