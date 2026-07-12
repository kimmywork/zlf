# Delivery Record v1: Vector and Embedding Retrieval

## Summary

Stage 03 delivers model-safe canonical exact vector retrieval and a durable asynchronous embedding pipeline. Vectors are keyed by generation/model/document identity, validated strictly, searched with bounded exact cosine/dot top-k, maintained from canonical lifecycle events, and exposed through WAM source-document similarity and async query/document embedding facades. Optional ANN is explicitly deferred with evidence; exact remains production and oracle.

## Source Artifacts

- `requirements-v1.md`
- `solution-design-v1.md`
- `plan-v1.md`
- parent `change-note-v1-function-first.md`
- parent `change-note-v2-pre-release-schema.md`
- parent `change-note-v4-defer-vector-ann.md`

## Acceptance Results

| Requirement | Result | Evidence |
|---|---|---|
| Generation/model/document multi-vector identity | pass | `vector_contracts.rs`, canonical key tests |
| Dimension/finite/metric/revision/normalization validation | pass | contract and exact tests |
| Exact cosine/dot oracle and bounded top-k | pass | `vector_exact.rs`, independent f64 fixture |
| Filters, threshold and deterministic ties | pass | exact backend tests |
| Update/delete/replay/reopen | pass | exact, job, lifecycle and facade tests |
| Source and query-text/vector retrieval | pass | exact API, async facade, `vector_similar/3` tests |
| Durable batch embedding lifecycle | pass | job/worker tests: lease, retry, dead, stale, batch, replay |
| Versioned model registry and transforms | pass | model/profile store tests |
| Ollama `bge-m3` deterministic/network-independent CI | pass | mock OpenAI-compatible protocol test |
| Opt-in local Ollama gate | pass | 1024-dimensional single and four-document batch smoke evidence |
| ANN | approved defer | change note v4; exact remains mandatory fallback |
| 1K/10K local exact evidence | pass | `research/vector-exact-local-2026-07-11.{json,md}` |
| Prototype replacement | pass | node-only backend/CLI/writers/queues removed |

## Verification Evidence

Fresh 2026-07-11 evidence:

- `cargo test --workspace` — pass, no failures
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines` — pass
- `cargo fmt --all -- --check` — pass after formatting
- `python3 scripts/check-rust-size.py` — pass
- `git diff --check` — pass
- `cargo run --release -p zlf-index --example vector_exact_benchmark -- 1000/10000` — pass
- `cargo run --release -p zlf-embed --example ollama_openai_smoke` — pass against local Ollama

Local Ollama batch: provider `ollama_openai_compatible`, model `bge-m3:latest`, batch 4, 81 characters, 1024 dimensions, 2550.45 ms, 1.568 docs/s, zero failures/retries, local cost unavailable.

## Review Results

### Spec Fit

pass

The functional exact path and durable provider pipeline satisfy the stage. ANN is optional by confirmed policy and its evidence-based deferral is documented rather than hidden.

### Format Fit (software)

pass

The implementation follows Stage 01 lifecycle ownership, WAM/provider boundaries, source-size policy, strict errors, deterministic tests, and reproducible evidence. Obsolete paths were removed rather than shimmed.

## Known Risks

- Exact search is linear and intended for the approved first functional/local tier; ANN may be reconsidered with Stage 06 evidence.
- Ollama aliases such as `latest` require deployment-side revision resolution/versioning to prevent silent model drift.
- Synchronous graph-write catch-up enqueues jobs for correctness; production background scheduling remains an operational optimization.

## Follow-ups

- Stage 04 ordered temporal semantics/index.
- Stage 05 bounded hybrid graph/WAM composition.
- Stage 06 public semantic-quality, true cold-process, 1024-dimensional larger-tier, and optional ANN evidence.

## Final Status

delivered
