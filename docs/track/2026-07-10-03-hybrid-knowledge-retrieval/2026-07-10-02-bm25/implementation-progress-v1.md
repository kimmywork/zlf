# Stage 02 Implementation Progress v1

## Increment B0 — BM25 contracts and independent oracle

**Status:** completed on 2026-07-11

### Delivered

- Versioned `Bm25Config`, bounded query shape, lexical hit, generation, and score-explanation contracts.
- Chunk-as-document scoring semantics inherited from Stage 01 `IndexDocumentId`.
- Documented BM25 IDF/TF/length-normalization formula in executable Rust.
- Versioned deterministic `unicode_jieba_v1` analyzer contract and multilingual goldens.
- Dependency-free Python reference scorer with deterministic ID tie-breaking.

### Verification

- Verification: hand-calculated term fixture → Rust score matches `0.5908617` within `1e-6` → **pass**.
- Verification: TF/length normalization ranking, empty corpus, invalid k1/b/top-k/candidate limits → **pass**.
- Verification: English, Chinese, and mixed analyzer goldens → **pass**.
- Verification: `python3 scripts/bm25_reference.py` → deterministic ranked fixture output → **pass**.
- Verification: format, Rust size, and diff checks → **pass**.

### Next

B1 integrates Tantivy behind the lexical contract with generation directories, replace/delete, reopen, and bounded top-k.
