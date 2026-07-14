# Review Feedback: Stage 05 Hybrid Prolog Implementation v1

## Decision

**Accept.** Stage 05 meets its retrieval-contract, preparation, boundedness, composition, proof, tabling, freshness, quality, and local-scale requirements. No blocking or high-severity findings remain.

## Cumulative findings

### Contracts and fusion

- Requests explicitly bound top-k, candidates, pages, page count, and answers and reject invalid or incompatible shapes before provider execution.
- Hits preserve canonical document/entity/field/chunk identity, source ranges, per-retriever rank/score/generation/watermark, fused rank/score, and exactness/exhaustion metadata.
- RRF uses fixed `k=60`, document identity deduplication, stable ties, and rank-only fusion; raw BM25 and cosine scores are not added.

### Preparation and provider execution

- Literal semantic queries are transformed, embedded, normalized, dimension-checked, and registered before WAM execution.
- Prepared handles freeze lexical/vector/temporal generations, published watermarks, and model identity.
- Explicit vector, lexical, and source-document modes do not invoke the remote provider; tests prove prepared lookup and WAM execution make no HTTP call.
- BM25, vector, event, and validity providers apply finite backend candidate limits and finite answer materialization, with metrics for exhaustion and peak answers.
- Backtracking, `once/1`, rule cut, proof, and deterministic ordering remain covered without introducing a second runtime or WAM-owned cursor.

### Paging, filtering, and planning

- Bound entity constraints reach Tantivy, exact vector, and generation-scoped temporal entity indexes before scoring/semantic evaluation.
- `retrieve/4` supports lexical, vector, and RRF hybrid modes, source exclusion, field constraints, document/entity aggregation, temporal filtering, and ordinary WAM graph/label/property/rule goals.
- Filtering uses the existing WAM/provider/rule runtime rather than a second evaluator.
- Metadata reports pages, candidates, graph/temporal rejects, strategy, exhaustion, and whether filtered top-k is exact.
- Planner explain reports `HybridRetrieval`; bound calls report the bound-entity strategy.

### Proof, tables, and freshness

- Retrieval answers emit compact stable `ProofKind::Index` leaves without source text.
- Only calls with bound prepared handle/options are tableable; live/unbound combinations fail explicitly.
- Index publication, embedding publication, generation activation/rollback, and canonical mutation catch-up invalidate dependent retrieval tables.
- Preparation supports per-target minimum watermarks and bounded wait timeout with typed target/minimum/published errors before WAM.

### Quality and scale

- The deterministic SciFact 1K/100-query subset compares BM25, exact `bge-m3`, and RRF on identical official qrels.
- RRF improved Recall@10/100 to 0.904667/0.990000 but did not beat BM25 MRR/nDCG@10. Reports correctly avoid a general fusion-superiority claim.
- EnterpriseKB 1K/10K exercises ordinary Prolog ACL filtering, generation-scoped validity checks, candidate/top-k ordering, permission mutation, and independent oracles.
- Both tiers produced 128/128 exact filtered queries and zero stale results; 10K p99 was 4.71 ms with candidate/answer limits 256/10.
- The workload exposed canonical bound `property/3` full materialization. Direct entity/key node/edge lookup reduced initial 10K p99 from approximately 2.9 s to 4.71 ms.

### Quality gates

The cumulative delivery tree passes workspace tests, strict workspace clippy, formatting, Rust source-size policy, and diff hygiene.

## Non-blocking follow-up

- The SciFact result is a deterministic sampled-corpus baseline, not a directly comparable full-BEIR leaderboard result.
- EnterpriseKB is generated-oracle composition evidence, not semantic-quality or mandatory security-enforcement evidence.
- EnterpriseKB 10K build took 166.75 s because the benchmark includes canonical per-node graph ingestion. Stage 06 should profile and, if needed, exercise bulk ingestion separately from query correctness.
- A WAM-owned external cursor remains deferred because finite bounded materialization satisfies Stage 05. Add one only after measured evidence and the full choice-point/cut/proof ownership matrix.
- ANN, multilingual public relevance, multi-hop knowledge, and agent-memory quality remain Stage 06 scope.
