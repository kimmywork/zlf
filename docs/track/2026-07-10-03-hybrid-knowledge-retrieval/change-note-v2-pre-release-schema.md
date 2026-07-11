---
status: in_progress
scope_type: parent
created: 2026-07-11
version: 2
---

# Change Note v2: Pre-Release Schema Policy

## Decision

zlf has not published a first version. This track does not preserve old database files, prototype index formats, serialized record layouts, or backward-compatible API behavior.

## Plan impact

- Remove legacy database fixtures, migration guards, bootstrap markers for pre-lifecycle databases, and old-format open tests.
- Storage and index schemas may change directly to the clean first-version design.
- Prototype BM25/vector/temporal databases are discarded and rebuilt, not migrated or kept readable.
- Existing predicate/API names may be changed directly when the approved first-version contract requires it; do not add compatibility aliases.
- Reopen, backup, generation rollback, corruption handling, and schema-version checks still apply to databases created by the new implementation.

## Next action

Stage 01 I0 legacy compatibility work is cancelled. Continue directly with I2 atomic canonical mutation and durable outbox.
