# Delivery Record v1: Kernel Enhancements Stages 0–8

## Summary

Stages 0–8 are implemented on the active WAM runtime path. Stage 4 uses canonical cons lists and call-time WAM builtin execution. Stage 5 provides opt-in proof capture. Stage 6 provides deterministic positive two-level tabling. Stage 7 persists exact fact/predicate/rule/table dependencies and selectively invalidates/recomputes affected tables. Stage 8 adds call-time provider pushdown, mode-aware query plans, persistent key layouts, bounded memory controls, and the WAM memory-root/lazy-loading design inventory.

## Source Artifacts

- Requirements: `requirements-v1.md`
- Design: `solution-design-v1.md`
- Plan: `plan-v1.md`
- ISO research: `research/iso-prolog-compatibility.md`

## Changed Areas

- WAM builtin dispatch, arithmetic, terms, conversions, list library rules, control/meta-call, and dynamic database operations.
- Canonical `[]` / `'.'/2` list lowering and integer/float/string constant identity.
- Parser support for list tails, quoted atoms, directives, and Stage 4 operators.
- Storage-backed fact/rule assertion, retraction, clause inspection, and predicate enumeration.
- Optional proof state, stable clause identities, proof markers, choice-point proof checkpoints, and `WamRuntime::query_all_with_proof`.
- Category-level ISO tests in `crates/zlf-prolog/tests/` and facade integration tests in `crates/zlf-query/tests/`.
- Variant table keys, SCC grouping, semi-naive delta variants, direct nested tabled subgoals, memory/RocksDB table manager, declaration persistence, stale recomputation, and NCBI taxonomy stress tools.

## Acceptance Results

| Acceptance / Req | Result | Evidence |
|---|---|---|
| Stage 4 canonical list matching | pass | `iso_terms::canonical_cons_unification_uses_the_wam_unifier` |
| Stage 4 arithmetic and numeric types | pass | `iso_arithmetic` test target |
| Stage 4 type/term builtins | pass | `iso_terms` test target |
| Stage 4 list and conversion subset | pass | `iso_lists` test target |
| Stage 4 control and call/1..8 | pass | `iso_control` test target |
| Stage 4 dynamic facts/rules, clause, current_predicate | pass | `iso_dynamic`, `stage4_iso_integration` |
| Stage 4 facade regressions | pass | `kernel_enhancements` |
| Stage 5 opt-in proof output | pass | `proof_terms::proof_capture_is_opt_in_and_records_fact_and_rule_nodes` |
| Stage 5 stable clause IDs | pass | `proof_terms::clause_ids_are_stable_for_identical_sources` |
| Stage 5 proof rollback | pass | `proof_terms::backtracking_rolls_proof_nodes_back_to_the_choice_point` |
| Stage 6 cyclic positive recursion | pass | `tabling::positive_recursive_tabling_terminates_on_cycles` |
| Stage 6 nested tables and SCC | pass | `nested_tabled_subgoals_join_complete_variant_answers`, `mutually_recursive_tabled_predicates_complete_as_one_component` |
| Stage 6 memory/RocksDB two-level store | pass | `table_persistence` and `tabling_integration` |
| Stage 6 full-data scale | pass | `../2026-07-10-01-ncbi-taxonomy-scale/research/full-stress-findings-v1.md` |
| Stage 7 exact/selective invalidation | pass | `table_persistence`, `tabling_integration` exact retract/restart/dependent-table tests |
| Stage 8 bound pushdown/query plans | pass | `bound_storage_provider`, `query_plan` |
| Stage 8 memory/lazy-loading design | pass | `research/wam-memory-and-lazy-loading.md` |

## Verification Evidence

- `cargo fmt --all -- --check`: pass.
- `python3 scripts/check-rust-size.py`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines`: pass.
- `cargo test --workspace`: pass; ignored Ollama/wiki tests remain opt-in.

## Review Results

### Spec Fit

pass for Stages 0–8. Optional Stage 9 modules remain deferred and are not required for this parent track.

### Format Fit (software)

pass. Builtin semantics are in `zlf-prolog`; providers remain read-side external relation sources. No query- or CLI-level Prolog builtin implementation remains.

Independent reviewer/subagent was unavailable; verification used a fresh self-review plus full focused crate gates.

## Known Risks

- Proof output records stable clause IDs, predicate/kind, parent links, per-node argument substitutions, and final answer bindings; large proofs therefore remain an opt-in memory cost.
- Stage 6 supports deterministic explicit positive tabling, not negation/WFS, aggregation, answer subsumption, or persisted live continuations.
- Insert/delete delta maintenance remains a future optimization; Stage 7 currently performs selective stale marking followed by lazy full-table recomputation.

## Follow-ups

- [x] Stage 6: deterministic positive tabling MVP with two-level storage.
- [x] Stage 7: persisted dependency-driven selective invalidation and lazy recomputation.
- [x] Stage 8: storage pushdown, query plans, bounded memory foundations, and memory-root design.
- [ ] Optional Stage 9 modules should use a separate approved track if pursued.

## Final Status

complete — required Stages 0–8 delivered; optional Stage 9 remains deferred.
