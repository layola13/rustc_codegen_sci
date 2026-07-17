# rustc_codegen_sci Current Plan

Updated: 2026-07-17.

## Active Direction

Keep the main path fixed:

```text
rustc MIR -> canonical SCI plan -> SA/direct SAB -> SCI Referee -> object
```

No LLVM backend fallback and no `bc2sa` fallback.

## Current Window

1. Build 20-30 bidirectional C/LLVM ABI fixtures over the serialized rustc
   `FnAbiPlan` and `TypeLayoutRecipe`; the worker-level ABI/layout fixture
   matrix covers the current validation boundary, and the linked smoke suite now
   has initial Direct scalar C-to-SCI and SCI-to-C coverage.
2. Implement the first Pair/Cast/Indirect C ABI fixtures and aggregate return/arg
   lowering against rustc `FnAbi` evidence.
3. Add target-qualified stack/load/store plans with size and alignment, then
   connect local aggregates to memory representation.
4. Add allocation images and relocations for statics, strings, and panic data.
5. Add direct SAB no-fallback emission from the same canonical plan.

## Exit Gates

- `./scripts/test.sh` passes.
- C/SCI ABI fixtures link and execute.
- Worker validation rejects malformed types, CFG, memory, and ABI plans.
- Unsupported MIR/ABI fails before SCI object publication.
- Each completed increment updates `tasks.md`, `progress.md`, and this file,
  then receives a focused git commit.
