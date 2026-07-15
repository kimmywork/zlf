---
status: pending
scope_type: stage
created: 2026-07-15
parent_id: 2026-07-15-01-code-indexing
version: 1
---

# Canonical code graph stage

Define and persist language-neutral repository, file, module, type, function/method, variable, and import entities plus containment, definition/reference, call, inheritance/implementation, and dependency edges with source provenance and confidence.

## Confirmed symbol identity

Every concrete symbol definition is a separate node. Definitions with the same simple name are never merged. Stable identity is derived from repository identity, language, module/package/namespace, enclosing symbol path, symbol kind, and normalized signature/overload discriminator. `simple_name` and `qualified_name` remain indexed attributes. Repository revision, source fingerprint, and index generation version the definition without normally changing its logical ID. Cross-language implementations connect through separate contract/external-symbol nodes.

## Relation evidence

Relations may originate from Tree-sitter, build/dependency metadata, compiler/language tooling, LSP, shared contracts, or manual mappings. Every relation records provider, provider version, certainty (`resolved`, `specified`, `declared`, `inferred`, or `unresolved`), source revision, and source range/configuration provenance.

## Acceptance

- IDs remain stable across unchanged re-imports and distinguish repositories/languages/scopes/overloads.
- Optional build/LSP evidence can upgrade an unresolved or inferred relation without deleting its provenance history.
- Every inferred edge names its extraction rule, source range, and confidence/certainty class.
- File update/delete atomically supersedes all owned symbols and edges.
- Graph/Prolog fixtures independently verify canonical facts and bounded traversal.

## Non-goals

Full type checking, dynamic dispatch completeness, or replacing language-specific compiler tooling.
