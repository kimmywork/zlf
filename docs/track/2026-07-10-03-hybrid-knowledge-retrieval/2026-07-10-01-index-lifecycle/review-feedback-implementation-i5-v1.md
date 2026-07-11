# Review Feedback: Stage 01 Increment I5

## Scope

Cumulative self-review of Stage 01 requirements/design and I5 profiles/chunking/manifests. No independent reviewer subagent was available.

## Findings

### I5-R1
- **Severity:** major
- **Type:** correctness
- **Description:** Accepting an arbitrary nonempty `source_hash` would not make profile artifacts content-addressed.
- **Resolution:** canonical profile content now has a SHA-256 hash; empty hashes are populated by the single store/lowering path and mismatched hashes are rejected.

### I5-R2
- **Severity:** major
- **Type:** correctness
- **Description:** Profile activation stored in a separate backend could race graph mutation sequence ordering.
- **Resolution:** artifacts and active pointers commit through primary storage together with a `ConfigurationChanged` outbox event.

### I5-R3
- **Severity:** minor
- **Type:** correctness
- **Description:** Mixed Chinese/English fixed windows cannot rely on whitespace tokenization alone.
- **Resolution:** baseline tokenizer groups ASCII words and treats non-ASCII non-punctuation characters as deterministic tokens; UTF-8 ranges have goldens.

## Decision

**Pass for I5.** No unresolved critical/major findings. Rust, JSON, and Prolog profile entry points lower through one immutable store. Proceed to I6 coordinator/fake target.
