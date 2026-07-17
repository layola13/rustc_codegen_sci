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
   matrix and backend Pair/Cast/Indirect compile-fail fixtures cover the current
   validation boundary, and the linked smoke suite now has 33 counted Direct
   scalar C-to-SCI and SCI-to-C cases.
2. Extend the first Cast ABI return lowering into broader Pair/Cast/Indirect C
   ABI fixtures and aggregate return/arg lowering against rustc `FnAbi`
   evidence; current smoke coverage supports single-field signed and unsigned
   8/16/32/64-bit Cast C ABI returns and rejects Indirect C ABI plus Pair Rust
   ABI returns before MIR lowering.
3. Extend structured diagnostics from worker RPC codes/coarse locations and
   backend fatal codes plus MIR block/statement contexts/source spans to
   protocol-level backend diagnostic payloads.
4. Extend the new scalar raw-pointer load/store path to stack allocations,
   dynamic array/slice projections, and whole-aggregate memory representation.
5. Add allocation images and relocations for statics, strings, and panic data.
6. Add direct SAB no-fallback emission from the same canonical plan.

## Exit Gates

- `./scripts/test.sh` passes.
- C/SCI ABI fixtures link and execute.
- Worker validation rejects malformed types, CFG, memory, and ABI plans.
- Unsupported MIR/ABI fails before SCI object publication.
- Each completed increment updates `tasks.md`, `progress.md`, and this file,
  then receives a focused git commit.
