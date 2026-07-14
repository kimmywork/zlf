# H6 dataset preparation v1

## Scope

H6 uses two datasets with separate claims:

1. **SciFact H6 subset** — real public judgments for BM25/vector/RRF quality comparison.
2. **EnterpriseKB-v1** — generated graph, ACL, temporal, mutation, and top-k correctness/scale fixture. It is not evidence of real semantic quality.

Raw/generated data stays under ignored `data/benchmarks/`. Only scripts, manifests/checksums, attribution notes, and compact reports belong in Git.

## Prerequisites

```bash
python3 --version        # Python 3.10+
curl --version           # optional; the script uses Python urllib
unzip -v                 # optional; the script uses Python zipfile
ollama --version
ollama pull bge-m3:latest
curl -fsS http://localhost:11434/api/tags >/dev/null
```

Keep at least 5 GiB free for raw data, prepared subsets, embeddings, and temporary indexes. The H6 subset itself is much smaller, but index and rerun headroom is intentional.

## Prepare SciFact

From the repository root:

```bash
python3 scripts/prepare-scifact.py \
  --output data/benchmarks/scifact \
  --documents 1000 \
  --queries 100 \
  --seed zlf-h6-scifact-v1
```

The script:

- downloads the BEIR SciFact archive from the UKP-hosted BEIR dataset endpoint;
- verifies the published archive MD5 before extraction;
- selects 100 judged test queries by deterministic SHA-256 ordering;
- retains every positive-qrel document for those queries;
- fills the corpus to 1,000 documents with deterministic hash-ranked distractors;
- writes UTF-8 JSONL/TSV plus a SHA-256 manifest.

Expected layout:

```text
data/benchmarks/scifact/
├── raw/
│   ├── scifact.zip
│   └── extracted/scifact/
│       ├── corpus.jsonl
│       ├── queries.jsonl
│       └── qrels/test.tsv
└── h6-1000d-100q-v1/
    ├── corpus.jsonl
    ├── queries.jsonl
    ├── qrels.tsv
    └── manifest.json
```

Verify counts and checksums:

```bash
wc -l data/benchmarks/scifact/h6-1000d-100q-v1/{corpus,queries}.jsonl
wc -l data/benchmarks/scifact/h6-1000d-100q-v1/qrels.tsv
python3 -m json.tool \
  data/benchmarks/scifact/h6-1000d-100q-v1/manifest.json >/dev/null
shasum -a 256 \
  data/benchmarks/scifact/h6-1000d-100q-v1/{corpus,queries}.jsonl \
  data/benchmarks/scifact/h6-1000d-100q-v1/qrels.tsv
```

Expected first two counts are exactly 1,000 documents and 100 queries. `qrels.tsv` includes one header row and a dataset-dependent number of judgments. Do not replace missing judgments with handcrafted relevance labels.

Rerunning the same command deletes and recreates only the prepared subset directory. The downloaded archive is reused and reverified.

## License and attribution check

Before publishing a report:

1. Preserve the SciFact/BEIR source URL and archive checksums from `manifest.json`.
2. Review the upstream SciFact and BEIR dataset cards/license statements at download time.
3. Add the confirmed attribution/license text to the Stage 06 research report.
4. Do not commit or redistribute the downloaded archive or extracted corpus in this repository.

The preparation script records provenance but does not substitute for upstream license review.

## Prepare EnterpriseKB-v1

EnterpriseKB requires no external download. The H6 implementation increment will provide a deterministic generator with these target tiers:

```text
data/benchmarks/enterprise-kb/v1-1k/
data/benchmarks/enterprise-kb/v1-10k/
```

The generator will freeze seed, documents/chunks, users/groups/grants, classifications, event/validity records, mutations, retrieval queries, and independent ACL/temporal oracles. Do not hand-author this corpus before the generator lands: manually produced data would not satisfy reproducibility or oracle requirements.

## Embedding policy

For the SciFact quality run use the real configured provider:

```bash
export ZLF_EMBED_PROVIDER=ollama
export ZLF_EMBED_ENDPOINT=http://localhost:11434
export ZLF_EMBED_MODEL=bge-m3:latest
export ZLF_EMBED_DIMENSION=1024
```

Record embedding build time separately from retrieval latency. Do not include remote embedding time in query p50/p95/p99. Exact RocksDB vector search remains the H6 backend.

EnterpriseKB correctness/scale runs may use a deterministic provider for repeatability, but any such result must be labeled generated-oracle correctness rather than semantic quality. A separate small real-`bge-m3` smoke run may be reported.

## H6 quality outputs

All three retrievers must use the same SciFact corpus, queries, and qrels:

```text
BM25
exact bge-m3 vector
RRF hybrid (k=60)
```

Required metrics:

```text
MRR
nDCG@10
Recall@10
Recall@100
hybrid minus lexical/vector deltas
candidate/page counts
p50/p95/p99 retrieval latency
embedding build time (separate)
RSS and index size
```

Fusion is reported as better only when these measured metrics show an improvement.

## Handoff checklist

Before asking the agent to continue H6, provide either:

```bash
test -f data/benchmarks/scifact/h6-1000d-100q-v1/manifest.json
python3 -m json.tool data/benchmarks/scifact/h6-1000d-100q-v1/manifest.json
```

or the exact error emitted by `scripts/prepare-scifact.py`. Do not send or commit the raw corpus.
