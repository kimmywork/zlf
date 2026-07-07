# Change Note 001: FFI Strategy Change (napi-rs → JSON-over-STDIO)

## Linked Work

- Requirements / track: docs/track/zlf/prd-v1.md (REQ-013: TypeScript SDK)
- Solution design: docs/track/zlf/solution-design-v1.md (Option B: Rust Core + TypeScript Shell)
- Plan: docs/track/zlf/plan-v1.md (Slice 8, 9)
- Delivery record: docs/track/zlf/delivery-record-v1.md

## Discovery Phase

build

## Original Decision

Use napi-rs for TypeScript-Rust FFI bindings. TypeScript SDK loads native .node module directly.

## Problem Found

napi-rs requires building within Node.js context. The `cdylib` crate type causes linker errors when building standalone:
```
ld: symbol(s) not found for architecture arm64
clang: error: linker command failed with exit code 1
```

Root cause: napi-rs links against Node.js symbols that are only available when building via npm/Node.js build process.

## New Decision

Replace napi-rs FFI with JSON-over-STDIO approach:
1. **Rust CLI Binary** (`zlf-cli`): Accepts JSON commands via STDIN, returns JSON via STDOUT
2. **TypeScript SDK**: Calls Rust binary via `child_process.execFile`
3. **No native modules**: Just a Rust executable + TypeScript wrapper

## Impact

- User behavior: No change (same API surface)
- Modules/files:
  - Modified: `crates/zlf-api/Cargo.toml` (removed cdylib, napi deps)
  - Created: `crates/zlf-cli/` (new Rust binary crate)
  - Modified: `packages/zlf/src/zlf.ts` (use child_process instead of native binding)
- Data/contracts: JSON protocol over STDIN/STDOUT (request/response format)
- Tests/verification: Need to test Rust CLI binary + TypeScript SDK integration
- Cross-feature knowledge to update in `docs/knowledge`: FFI approach changed
- Risks: Slightly higher latency (process spawn vs direct FFI call)

## Approval / Rationale

Autonomous change - simplifies build process, removes native module compilation complexity, more portable across platforms.

## Verification Update

- Test Rust CLI binary: `echo '{"command":"init","path":"./test-db"}' | cargo run -p zlf-cli`
- Test TypeScript SDK: Unit tests with mocked child_process
- Integration test: TypeScript SDK → Rust CLI → actual database operations

## Scope Reduction

- Original scope items removed: napi-rs native module compilation
- Reason: Build complexity and cross-platform issues
- Impact on later phases: None (same functionality, different implementation)
- Deferred decisions: None
- Revisit trigger: If performance profiling shows process spawn overhead is significant
