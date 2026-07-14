# Review Feedback: Stage 06 Benchmark Design v1

## Decision

**Pass.** The design is executable, bounded by the approved M2 Pro/32 GiB and 100K-chunk ceiling, and preserves the single production runtime. Implementation may begin with S0.

## Cumulative review

### Requirements alignment

- Separates generated correctness, protocol smoke, and public relevance claims.
- Preserves official qrels and deterministic sampling provenance.
- Covers combined retrieval/graph/rule/temporal behavior, updates/deletes, restart, reliability, multilingual retrieval, and resource reporting.
- Keeps all limits explicit and excludes generated/raw data from Git.

### Architecture and feasibility

- Reusing Python preparation/orchestration plus release Rust executables avoids a new service or benchmark runtime.
- Immutable identity-keyed checkpoints make real embedding runs feasible without allowing stale cache reuse.
- 1K/10K smoke/local tiers and one 100K generated release tier are feasible based on existing component evidence; S1 remains high risk because canonical ingestion and combined lifecycle cost have not yet been measured at 100K.
- Public adapters and dataset adoption are isolated behind source/license/schema research, so one ambiguous dataset does not corrupt or block unrelated evidence.

### Verification quality

- Correctness and stale-result checks are hard gates from the first run.
- Numeric latency/RSS/disk thresholds are established from clean baselines rather than invented before measurement.
- Fresh-process and OS-cold terminology is explicit.
- ANN metrics are omitted while ANN is deferred.

## Resolved challenges

- A new benchmark crate/service was rejected as unnecessary technology sprawl.
- Full public datasets are not required when deterministic <=100K subsets preserve judgments and publish checksums.
- Multi-hop/memory candidates may be rejected with sourced evidence; weak or license-ambiguous data must not be forced into a quality claim.
- Real embedding latency is checkpointed and separately reported, never hidden inside retrieval latency.

## Non-blocking cautions

- Sampling reports must not be compared directly to full-corpus public leaderboards.
- The 100K generated run should first measure ingestion phases separately; the Stage 05 10K graph build indicates canonical per-entity ingestion may dominate.
- Dataset license statements must be traced to primary upstream sources at access time rather than copied from secondary benchmark summaries.
- S4 should adopt the smallest auditable multi-hop/memory subset that proves integration; it should not expand into benchmark-specific model tuning.
