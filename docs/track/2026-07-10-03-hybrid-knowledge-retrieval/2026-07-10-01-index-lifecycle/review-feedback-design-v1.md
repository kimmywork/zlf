# Review Feedback v1: Stage 01 Index Lifecycle Design

## Scope

Cumulative review of the parent requirements/design/plan, Stage 01 requirements, current-state investigation, and Stage 01 `solution-design-v1.md` plus `plan-v1.md`.

Reviewer limitation: no independent reviewer subagent was available. This was performed as a separate self-review pass against repository source, existing tests, architecture constraints, and the delivery-loop gates.

## Accuracy pass

- `Storage` currently owns one RocksDB database and graph writes are spread across independent puts/deletes; consolidating read-modify-write under an in-process mutex and one `WriteBatch` matches the current single-process/exclusive-open model.
- Current `Edge` and `Node` values are directly bincode-compatible records. The design leaves both layouts unchanged and adds external lifecycle metadata.
- WAM dynamic facts lower through `StorageFactWriter`; generic property writes currently create/update nodes, while edge property updates are absent. The proposed resolution and explicit predicates address the confirmed gap below the facade.
- API-created UUID edges can share a triple, while Prolog-created edge IDs are deterministic. Defining `edge_id/4` as an ordered multi-row relation is consistent with both paths.
- Bulk record plans and raw storage methods currently permit graph-adjacent writes outside ordinary mutation methods. The plan explicitly audits and routes graph bulk loading instead of claiming raw metadata APIs can never write arbitrary keys.

## Findings and resolutions

### S1

- **Origin phase:** solution design
- **Severity:** major
- **Type:** correctness
- **Description:** The initial draft did not specify how profile activation receives an effective mutation sequence without racing graph writes. A profile store in a separate DB could observe a sequence and activate while another mutation commits.
- **Evidence:** parent design requires activation history with an effective mutation sequence; current indexes are separate RocksDB databases and have no cross-DB atomic transaction.
- **Suggested fix:** persist validated opaque artifact envelopes and activation records through a storage-neutral primary-DB operation that allocates a sequence and emits a target-agnostic configuration event in the same batch.
- **Resolution:** fixed in `solution-design-v1.md` sections 1 and 5 and in plan I2/I5. `zlf-storage` does not import `zlf-index` types.

### S2

- **Origin phase:** solution design
- **Severity:** major
- **Type:** inconsistent
- **Description:** The initial broad statement that all no-op operations emit no event conflicted with the existing `update_node` compatibility test, which expects an explicit full replacement with equal properties to create a version.
- **Evidence:** `crates/zlf-storage/tests/storage.rs` contains `test_update_with_same_properties_creates_version`; Stage 01 only requires remove to be idempotent and patch semantics to be explicit.
- **Suggested fix:** restrict event-free no-op behavior to idempotent patch/label operations and preserve explicit full-replacement update behavior.
- **Resolution:** fixed in design principles, canonical mutation semantics, and plan I2.

### S3

- **Origin phase:** solution design
- **Severity:** minor
- **Type:** unclear
- **Description:** “Exactly one event” could be read as one event for an entire cascade, which would hide individual edge tombstones from index consumers.
- **Evidence:** Stage acceptance requires node deletion/cascade to produce exact live documents; edge profiles need stable edge deletion events.
- **Suggested fix:** state that a cascade commits one event per deleted edge and one for the node in one atomic batch.
- **Resolution:** already explicit in section 2 and reflected in I2 verification.

### S4

- **Origin phase:** planning
- **Severity:** minor
- **Type:** scope
- **Description:** Parent P0 asks for baseline failures, but committing default failing tests would break every quality gate.
- **Evidence:** workspace acceptance requires `cargo test --workspace`; current limitations are known prototypes.
- **Suggested fix:** use fixtures, characterization tests, or explicit ignored limitation probes, and reserve red tests for the active test-first increment where they are immediately resolved.
- **Resolution:** plan I0 explicitly prohibits a red default suite.

## Validity and consistency pass

- The stage remains within P0–P2 and does not select BM25 or ANN implementations.
- Storage owns source mutation order but not backend logic; `FactProvider` remains read-only.
- Profile activation, graph mutations, and bulk rebuild markers share one ordered event stream without introducing cross-RocksDB atomicity.
- Stale suppression uses current entity tombstones/source versions, not wall-clock timestamps or mutable edge serialization.
- Outbox retention is safety-gated by active target watermarks; diagnostic payload retention is bounded separately.
- Generation failure cannot move the active pointer, and retention keeps one rollback generation.
- The plan maps every Stage 01 acceptance criterion to executable integration/state-machine tests.

## Decision

**Pass.** No unresolved critical or major findings remain. Stage 01 design is executable, and implementation may begin at I0/I1 only, then follow the declared dependency gates.
