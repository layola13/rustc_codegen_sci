# rustc_codegen_sci Current Plan

Updated: 2026-07-16.

## Active Direction

Keep the main path fixed:

```text
rustc MIR -> canonical SCI plan -> SA/direct SAB -> SCI Referee -> object
```

No LLVM backend fallback and no `bc2sa` fallback.

## Current Window

1. Finish direct scalar/raw-pointer ABI fixtures, including null and pointer
   equality behavior. Keep dereference and provenance-changing casts gated.
2. Introduce explicit ABI plan data instead of inferring ABI from scalar local
   lists: Ignore, Direct, Pair, Cast, and Indirect.
3. Implement the first Pair/Indirect C ABI fixtures and aggregate return/arg
   lowering against rustc `FnAbi` evidence.
4. Add target-qualified stack/load/store plans with size and alignment, then
   connect local aggregates to memory representation.
5. Add allocation images and relocations for statics, strings, and panic data.
6. Add direct SAB no-fallback emission from the same canonical plan.

## Exit Gates

- `./scripts/test.sh` passes.
- C/SCI ABI fixtures link and execute.
- Worker validation rejects malformed types, CFG, memory, and ABI plans.
- Unsupported MIR/ABI fails before SCI object publication.
- Each completed increment updates `tasks.md`, `progress.md`, and this file,
  then receives a focused git commit.
