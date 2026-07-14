# Stage 06 implementation progress v11

## Increment

Evaluate and qualify `hnsw_rs` on the frozen 100K × 1024 vector corpus without replacing the exact RocksDB oracle/fallback.

## Delivered

- Added `hnsw_rs 0.3.4` as an experiment-only `zlf-index` development dependency.
- Added a release benchmark consuming the unchanged frozen document, query, group, and self-ID bytes.
- Added identity-scoped immutable HNSW publication with:
  - generation/model/version and dataset checksum binding;
  - HNSW parameter binding;
  - atomic temporary-directory rename;
  - durable, validated canonical ID mapping;
  - graph/data/mapping completeness checks and reopen.
- Compared five bounded `ef_search` settings for top-10/top-100, unfiltered, 10%, and 1% filters.
- Used exact RocksDB top-k results as the Recall@k oracle.
- Added a rebuild lifecycle probe covering update, delete, insert, dump, and reopen.
- Added shared-schema report packaging and compact JSON/Markdown evidence.

## Result

The selected quality-oriented setting is `M=48`, `ef_construction=400`, and `ef_search=2048`:

- Recall@10/100: `0.9810/0.9801` unfiltered.
- Unfiltered p99: `137.17/106.43 ms` versus exact `742.94/795.36 ms`.
- Sequential QPS: `10.18/10.46` versus exact `1.56/1.51`.
- Filter recall: `0.9950–0.9975`; filtered p99: `275.57–399.17 ms`.
- Self-query canonical top-1: `100/100`.
- Build/reopen: `549.12 s / 817.27 ms`.
- Index/RSS: `571.5 MiB / 1,271.6 MiB`.

## Lifecycle decision

`hnsw_rs` exposes no in-place update/delete operation. The accepted mutation policy for this candidate is therefore replacement of an immutable generation built from exact RocksDB. The probe passed update/delete/insert plus fresh reload. Exact RocksDB remains the source of truth, correctness oracle, and fallback.

The candidate is qualified, but this increment does not route the production `ZlfDatabase` facade to it. That cutover still requires facade-level rebuild orchestration and a tested corrupt/missing-ANN fallback; the measured 4.55× exact-backend RSS and much slower build also require an explicit deployment choice.

## Evidence

- `research/vector-hnsw-frozen-100k-2026-07-14.json`
- `research/vector-hnsw-frozen-100k-2026-07-14.md`
- exact comparison: `research/vector-exact-frozen-100k-2026-07-14.{json,md}`

## Next

Resume the minimum HotpotQA/KILT multi-hop adoption/deferral decision, then run cumulative Stage 06 review. ANN production routing remains a separate explicit increment rather than an implicit replacement of exact vectors.
