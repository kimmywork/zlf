# Temporal local scale baseline — 2026-07-11

## Method

The release benchmark builds equal event and validity populations at 1K, 10K, and 100K records per kind for three distributions:

- `uniform`: minute-spaced events and one-day finite intervals;
- `skewed`: duplicate events in 100 timestamps, one-third open and otherwise year-long intervals;
- `long_open`: sparse events, 80% open intervals and 20% thousand-day intervals.

Each tier runs 100 event-range, `valid_at`, and overlap queries, replaces 100 records of each kind, and records candidates, p50/p95/p99, throughput, RSS, and disk. Every measured query is checked against the independent oracle.

```bash
cargo run --release -p zlf-index --example temporal_benchmark -- 1000
cargo run --release -p zlf-index --example temporal_benchmark -- 10000
cargo run --release -p zlf-index --example temporal_benchmark -- 100000
```

Machine: Apple M2 Pro, 32 GiB. Raw machine-readable reports are adjacent JSON files.

## 100K summary

| Distribution | Build 200K records | Event candidates / p99 | Valid-at candidates / p99 | Overlap candidates / p99 | Disk |
|---|---:|---:|---:|---:|---:|
| uniform | 407.7 ms | 1,440 / 0.44 ms | 50,001 / 13.54 ms | 51,439 / 13.80 ms | 96.6 MB |
| skewed | 434.2 ms | 100,000 / 33.41 ms | 1,000 / 0.60 ms | 100,000 / 61.93 ms | 95.5 MB |
| long-open | 377.9 ms | 144 / 0.06 ms | 50,001 / 27.68 ms | 50,144 / 24.34 ms | 94.1 MB |

Process peak RSS across the 100K run was 531.8 MB. Replacing 200 total records took 25.3–29.5 ms depending on distribution.

## Decision

No bucket or interval-tree derivative is added in Stage 04:

- selective event and estimated endpoint queries remain strongly bounded;
- full candidate scans occur when the fixture deliberately makes every record a true match, where an accelerator cannot avoid returning/materializing the requested full result set;
- worst 100K p99 remained below 65 ms and RSS below the first 512 MiB binary budget (`536,870,912` bytes).

Frozen local regression budgets:

- 100K build of 200K records: <= 2 seconds;
- selective query p99: <= 50 ms;
- all-match skew overlap p99: <= 150 ms;
- 100K run RSS: <= 536,870,912 bytes;
- each distribution disk: <= 150 MB;
- measured results must exactly match the independent oracle.

These synthetic distributions verify semantics and scale behavior, not enterprise workload prevalence.
