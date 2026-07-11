# Review Feedback: Stage 01 Increment I6

## Scope

Cumulative self-review of Stage 01 requirements/design and I6 coordinator/fake target. No independent reviewer subagent was available.

## Findings

### I6-R1
- **Severity:** major
- **Type:** correctness
- **Description:** The initial job selector could skip a dead/not-yet-retryable earlier job and apply later events, violating ordered target convergence.
- **Resolution:** fixed: completed/stale jobs are skipped, but dead, active lease, and delayed retry states block later sequences.

### I6-R2
- **Severity:** major
- **Type:** correctness
- **Description:** A crash after the target write but before job acknowledgment can replay the operation.
- **Resolution:** fake target writes idempotently by event sequence; retry detects the existing target record and then advances the contiguous watermark. Dedicated failure injection passes.

### I6-R3
- **Severity:** major
- **Type:** lifecycle
- **Description:** Outbox deletion based on one target could strand another active target.
- **Resolution:** compaction floor is the minimum published watermark across every persisted registered target; two-target tests pass.

## Decision

**Pass for I6.** No unresolved critical/major findings. Retry limits, dead letters, stale suppression, metrics, reopen, and compaction are verified. Proceed to I7 generations/status/wait/retention.
