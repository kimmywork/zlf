# Stage 06 implementation progress v8

## Increment

S3 real `bge-m3` quality runs for FiQA and MIRACL English/Chinese.

## Delivered

- Generalized the public retrieval release benchmark with explicit dataset/language identity.
- Corrected vector content fingerprints to hash the transformed document text rather than source ID.
- Added a configurable Unicode-safe embedding input cap.
- A 4,096-character Chinese preflight/run exposed an Ollama context-length HTTP 400; 2,048 characters passed and was fixed consistently for all final datasets.
- Added shared-schema report packaging and three machine-readable reports.
- Added a compact comparative report and sourced dataset adoption record.

## Quality summary

| dataset | best MRR/nDCG@10 | hybrid versus BM25 |
|---|---|---|
| FiQA 10K/100q | vector 0.670866/0.593081 | +0.123041/+0.133751 |
| MIRACL en shard 0/30q | vector 0.878030/0.895256 | +0.117322/+0.096961 |
| MIRACL zh shard 0/99q | vector 0.852790/0.879932 | +0.206826/+0.207527 |

Hybrid improves over BM25 on all reported metrics in these three bounded datasets, but exact vector remains strongest. No universal fusion superiority claim is made.

## Performance

Hybrid p99 was 67.53 ms FiQA, 43.30 ms MIRACL English, and 52.76 ms MIRACL Chinese. Document embedding took 721.3, 548.2, and 475.6 seconds respectively and is excluded from retrieval latency.

## Evidence

- `research/fiqa-10k-quality-2026-07-14.json`
- `research/miracl-en-shard0-quality-2026-07-14.json`
- `research/miracl-zh-shard0-quality-2026-07-14.json`
- `research/public-quality-local-2026-07-14.md`
- `research/dataset-adoption-v1.md`

## Next

S3 multilingual/public quality is complete. Decide and execute the smallest auditable HotpotQA/KILT multi-hop integration, or publish a sourced deferral if it would only proxy LLM answer quality. Then run Stage 06 cumulative stress/review.
