# Review feedback: Stage 06 HNSW increment v1

## Review scope

Independent cumulative review of the frozen 100K × 1024 `hnsw_rs` candidate implementation, lifecycle evidence, shared-schema report, and exact-vs-ANN decision.

## Findings

### Critical/high

None.

### Medium — production facade fallback is intentionally absent

The benchmark detects identity/completeness failures and reload errors, but the production `ZlfDatabase` facade does not yet attempt ANN and fall back to exact RocksDB. This is correctly disclosed and blocks production routing, not acceptance of the isolated engine evaluation.

**Disposition:** accepted follow-up; exact remains the only production vector path.

### Medium — immutable rebuild is the only correct mutation policy

`hnsw_rs 0.3.4` has no in-place update/delete API. The implementation correctly avoids duplicate-ID pseudo-updates and proves update/delete/insert through replacement-generation rebuild and reopen.

**Disposition:** accepted; any later facade integration must preserve this policy.

### Medium — resource and portability costs are material

The candidate consumes approximately 4.55× the exact backend's measured peak RSS, builds much more slowly, and its dump is not claimed portable across architectures or byte-identical across parallel rebuilds.

**Disposition:** accepted and documented; this prevents an unconditional default-backend decision.

### Low — single-run ANN quality variance

The report records one frozen run. Parallel randomized HNSW construction may vary Recall@k on rebuild. Before production defaulting, repeat-build quality should enforce a minimum gate rather than assuming the exact reported decimals.

**Disposition:** follow-up; does not invalidate persisted-dump reopen or the measured candidate qualification.

## Verification reviewed

- Frozen input checksums match the exact baseline.
- Exact RocksDB generated top-k truth for every workload.
- Canonical mapping validates all 100,000 IDs.
- 100/100 self queries return the canonical source at rank 1.
- Recall@10/100, all bounded `ef_search` values, filtered behavior, latency/QPS, build/reopen, RSS, and disk are present.
- Rebuild lifecycle probe passes update/delete/insert/dump/reopen.
- `cargo test -p zlf-index` passes.
- Workspace all-target strict Clippy passes.
- Python benchmark tests, formatting, source-size policy, report validation, and diff checks pass.

## Decision

**Accepted for the Stage 06 ANN evaluation increment, with production cutover explicitly deferred.**

`M=48`, `ef_construction=400`, `ef_search=2048` is a qualified quality-oriented candidate on this frozen corpus. Exact RocksDB remains the production backend, oracle, source of truth, and fallback until generation rebuild orchestration and facade-level corrupt/missing-ANN fallback are implemented and reviewed.
