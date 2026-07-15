---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Canonical code graph stage

Define and persist language-neutral repository, file, module, type, function/method, variable, and import entities plus containment, definition/reference, call, inheritance/implementation, and dependency edges with source provenance and confidence.

## Acceptance

- IDs remain stable across unchanged re-imports and distinguish repositories/languages/scopes/overloads.
- Every inferred edge names its extraction rule, source range, and confidence/certainty class.
- File update/delete atomically supersedes all owned symbols and edges.
- Graph/Prolog fixtures independently verify canonical facts and bounded traversal.

## Non-goals

Full type checking, dynamic dispatch completeness, or replacing language-specific compiler tooling.
