---
status: done
scope_type: standalone
created: 2026-07-15
version: 1
---

# Usage guide refresh

## Goal

Bring `docs/usage-guide.md` in line with the current Rust CLI, WAM-backed Prolog REPL, current predicates, optional index configuration, and persisted `IndexProfileArtifact` contract.

## Scope

- Remove the deleted TypeScript SDK/npm material and obsolete SDK examples.
- Remove obsolete CLI commands and old predicate signatures.
- Document `zlf repl [db_path]`, fact/rule/directive/query syntax, writable facts, shortcuts, current index predicates, and result behavior.
- Document IndexProfile JSON shape, matcher/field options, lifecycle commands, and embedding-disabled errors.
- Cross-check every documented JSON command against `crates/zlf-cli/src/protocol.rs`.

## Acceptance

- No active user-guide claim references a TypeScript SDK.
- All listed CLI commands exist in the current protocol.
- Prolog examples use current predicates and writable-fact forms.
- IndexProfile fields and examples match the Rust serde contract.
- Markdown links/headings and repository diff checks pass.

## Delivered

- Rewrote `docs/usage-guide.md` against the current Rust command protocol.
- Added complete REPL fact/rule/directive/query usage and current predicates.
- Added IndexProfile matcher, BM25, vector, chunking, temporal, JSON, Prolog directive, activation, and lifecycle documentation.
- Removed the obsolete TypeScript SDK/npm sections and examples.
- Fixed REPL database opening so it honors configured disabled/exact/HNSW vector strategy just like JSON/HTTP database opening.

## Verification

- `cargo test -p zlf-cli repl::tests`
- `cargo test -p zlf-cli --test index_profiles`
- workspace all-target strict Clippy
- formatting, Rust source-size policy, and `git diff --check`
