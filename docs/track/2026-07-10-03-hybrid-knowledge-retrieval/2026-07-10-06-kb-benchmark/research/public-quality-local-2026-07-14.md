# Public retrieval quality baselines — 2026-07-14

All runs use exact cosine vectors, `bge-m3:latest`, 1,024 dimensions, RRF `k=60`, candidate/answer limit 100, and a 2,048-character embedding input cap. Document embedding time is separate from retrieval latency. These are bounded prepared-corpus runs; MIRACL results are shard-0 judged pools, not full-corpus leaderboard results.

| dataset | queries | retriever | MRR | nDCG@10 | Recall@10 | Recall@100 |
|---|---:|---|---:|---:|---:|---:|
| FiQA 10K | 100 | lexical | 0.483525 | 0.405182 | 0.470241 | 0.692655 |
| FiQA 10K | 100 | vector | 0.670866 | 0.593081 | 0.656337 | 0.855667 |
| FiQA 10K | 100 | hybrid | 0.606566 | 0.538933 | 0.624337 | 0.827000 |
| MIRACL en shard 0 | 30 | lexical | 0.713486 | 0.752705 | 0.916667 | 0.950000 |
| MIRACL en shard 0 | 30 | vector | 0.878030 | 0.895256 | 0.966667 | 1.000000 |
| MIRACL en shard 0 | 30 | hybrid | 0.830808 | 0.849666 | 0.950000 | 1.000000 |
| MIRACL zh shard 0 | 99 | lexical | 0.462622 | 0.499475 | 0.696970 | 0.905724 |
| MIRACL zh shard 0 | 99 | vector | 0.852790 | 0.879932 | 1.000000 | 1.000000 |
| MIRACL zh shard 0 | 99 | hybrid | 0.669448 | 0.707003 | 0.922559 | 1.000000 |

## FiQA 10K

- Embedding: 721.3 s documents, 24.6 s queries.
- Retrieval p99: BM25 10.10 ms, vector 63.41 ms, hybrid end-to-end 67.53 ms.
- Peak RSS: 238.6 MiB; disk: 50.7 MiB.
- Hybrid versus BM25: MRR +0.123041, nDCG@10 +0.133751, Recall@10 +0.154095, Recall@100 +0.134345.

## MIRACL English shard 0

- Embedding: 548.2 s documents, 5.6 s queries.
- Retrieval p99: BM25 4.19 ms, vector 39.64 ms, hybrid end-to-end 43.30 ms.
- Peak RSS: 247.5 MiB; disk: 49.5 MiB.
- Hybrid versus BM25: MRR +0.117322, nDCG@10 +0.096961, Recall@10 +0.033333, Recall@100 +0.050000.

## MIRACL Chinese shard 0

- Embedding: 475.6 s documents, 18.8 s queries.
- Retrieval p99: BM25 2.58 ms, vector 47.73 ms, hybrid end-to-end 52.76 ms.
- Peak RSS: 241.0 MiB; disk: 49.9 MiB.
- Hybrid versus BM25: MRR +0.206826, nDCG@10 +0.207527, Recall@10 +0.225589, Recall@100 +0.094276.

## Interpretation

- FiQA: hybrid improves every reported metric over BM25 in this sampled test subset, while exact vector is strongest.
- MIRACL English/Chinese: vector is strongest on the bounded judged pools; RRF improves over BM25 but does not exceed vector MRR/nDCG.
- No universal fusion claim is made. Dataset sampling scope, 2,048-character cap, model, and qrels remain attached to every comparison.
- The initial 4,096-character Chinese attempt failed with an Ollama context-length HTTP 400. A 2,048-character Unicode-safe cap was then fixed and applied consistently to all three final runs.
