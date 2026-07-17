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
   evidence; current smoke coverage supports bidirectional single-field signed
   and unsigned 8/16/32/64-bit Cast C ABI arguments/returns and rejects Indirect
   C ABI plus Pair Rust ABI returns before MIR lowering.
3. Add allocation images and relocations for statics, strings, and panic data.
4. Add direct SAB no-fallback emission from the same canonical plan.

Recently completed: the worker now writes rust-trusted work-product manifests
recording plan/object hashes, target, cache policy, and SCI identity, and it
reuses content-addressed cached objects after manifest and object-hash
validation.

## Exit Gates

- `./scripts/test.sh` passes.
- C/SCI ABI fixtures link and execute.
- Worker validation rejects malformed types, CFG, memory, and ABI plans.
- Unsupported MIR/ABI fails before SCI object publication.
- Each completed increment updates `tasks.md`, `progress.md`, and this file,
  then receives a focused git commit.
