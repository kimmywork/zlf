# Change Note 002: Remove TypeScript CLI, Update Architecture Documentation

## Linked Work

- Requirements / track: docs/track/zlf/prd-v1.md (REQ-014: CLI Application)
- Solution design: docs/track/zlf/solution-design-v1.md
- Plan: docs/track/zlf/plan-v1.md (Slice 8, 9)
- Delivery record: docs/track/zlf/delivery-record-v1.md
- Change note: docs/track/zlf/change-note-001.md (FFI strategy change)

## Discovery Phase

record

## Original Decision

TypeScript CLI (`cli/`) as thin wrapper around TypeScript SDK, which calls Rust via napi-rs FFI.

## Problem Found

After Change Note 001 (FFI strategy change to JSON-over-STDIO):
1. TypeScript CLI became redundant - Rust CLI binary (`crates/zlf-cli`) handles all CLI functionality directly
2. TypeScript SDK (`packages/zlf`) calls Rust CLI binary via `child_process`
3. `cli/` directory contained unused code that could cause confusion

## New Decision

1. **Remove `cli/` directory** - No longer needed
2. **Architecture**:
   - User → TypeScript SDK → Rust CLI Binary → Rust Core
   - User → Rust CLI Binary → Rust Core (direct CLI usage)
3. **Updated documentation** to reflect new architecture

## Impact

- User behavior: No change (same functionality, just different implementation)
- Modules/files:
  - Removed: `cli/` directory (TypeScript CLI)
  - Modified: `docs/track/zlf/solution-design-v1.md` (architecture update)
  - Modified: `docs/track/zlf/plan-v1.md` (slice descriptions updated)
- Data/contracts: No change (JSON-over-STDIO protocol unchanged)
- Tests/verification: All tests still pass
- Cross-feature knowledge to update in `docs/knowledge`: Architecture changed
- Risks: None (functionality unchanged)

## Approval / Rationale

Autonomous change - cleanup of redundant code after architecture change in Change Note 001.

## Verification Update

- `cargo test` → All 94 Rust tests passing
- `cd packages/zlf && npm test` → All 14 TypeScript tests passing
- `bash cli/tests/e2e.sh` → Updated to reflect new structure

## Scope Reduction

- Original scope items removed: TypeScript CLI (`cli/` directory)
- Reason: Redundant after Rust CLI binary implementation
- Impact on later phases: None
- Deferred decisions: None
- Revisit trigger: None

## Files Changed

| Action | File | Description |
|--------|------|-------------|
| Removed | `cli/` | Entire TypeScript CLI directory |
| Modified | `docs/track/zlf/solution-design-v1.md` | Updated architecture, CLI interface, dependencies |
| Modified | `docs/track/zlf/plan-v1.md` | Updated Slice 8-9 descriptions |
