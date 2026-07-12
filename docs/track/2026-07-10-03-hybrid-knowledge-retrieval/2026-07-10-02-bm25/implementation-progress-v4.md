# Stage 02 Implementation Progress v4

## Increment B3 — Physical generation publication and diagnostics

**Status:** completed on 2026-07-11.

### Delivered

- BM25 indexes now live under `bm25/generations/<generation-id>`.
- Fresh databases atomically bootstrap a validated empty generation; reopen resolves only the active generation from `GenerationManager`.
- Profile activation and explicit `rebuild_bm25_generation` build a separate Tantivy directory, project the current corpus with generation-scoped manifests, checkpoint, validate document count/schema checksum, atomically activate, and then swap the in-process reader.
- Build failures before activation are recorded as failed and leave the previous active generation readable.
- Old reader `Arc`s remain valid for concurrent in-flight queries during publication.
- Fresh-process reopen tests verify the active physical generation and query results.
- Explanations are now structured `Bm25Explanation` values with document/average length, field weight, and per-term TF/DF/IDF/score components rather than backend debug strings.
- Repeated analyzed query terms are deduplicated before scoring.

### Verification evidence

- `cargo test -p zlf-index --test bm25_backend`
- `cargo test -p zlf-query --test bm25_lifecycle`
- `cargo clippy -p zlf-index -p zlf-query --all-targets -- -D warnings -W clippy::too_many_lines`
- `cargo fmt --all`
- `python3 scripts/check-rust-size.py`

### Next

B4 local quality/scale evidence and B5 cumulative acceptance remain. The function-first track decision allows large public benchmark orchestration to remain in Stage 06, but Stage 02 still needs a reproducible local corpus smoke report and lifecycle differential evidence before acceptance.
