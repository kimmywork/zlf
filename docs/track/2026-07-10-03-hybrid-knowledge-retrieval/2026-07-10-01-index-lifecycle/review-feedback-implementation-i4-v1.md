# Review Feedback: Stage 01 Increment I4

## Scope

Cumulative self-review of Stage 01 requirements/design and I4 bulk session implementation. No independent reviewer subagent was available.

## Findings

### I4-R1
- **Severity:** major
- **Type:** correctness
- **Description:** The previous loader wrote a record batch and progress marker in separate commits, so a crash could replay a batch without a matching checkpoint.
- **Resolution:** fixed by `write_bulk_plan`, which atomically commits records and session checkpoint.

### I4-R2
- **Severity:** major
- **Type:** correctness
- **Description:** Public raw and record-plan APIs could bypass canonical mutation/outbox behavior.
- **Resolution:** removed untracked `write_record_plans`; raw APIs now reject graph, lifecycle, outbox, and bulk-session key namespaces.

### I4-R3
- **Severity:** minor
- **Type:** operability
- **Description:** Reopen needed an inventory of unfinished sessions.
- **Resolution:** added durable session state and `list_bulk_sessions`; reopen/resume tests pass.

## Decision

**Pass for I4.** No unresolved critical/major findings. Bulk completion publishes exactly one idempotent `RebuildRequired` event. Proceed to I5 profiles/chunking.
