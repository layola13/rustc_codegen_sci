# rustc_codegen_sci Tasks

Baseline: 2026-07-16.

## Completed Bring-up

- [x] Independent rustc-private codegen backend workspace and pinned toolchain.
- [x] Canonical versioned SCI plan and bounded worker RPC.
- [x] MIR CGU traversal, SA emission, SCI `build-obj`, and rustc object return.
- [x] Scalar integer/bool arithmetic, comparisons, casts, CFG, calls, asserts,
  checked add/sub/mul, `isize`/`usize`, and void functions.
- [x] Local scalar tuple/struct construction, projection, and copy/move.
- [x] Direct raw-pointer ABI values for local/extern calls and returns.
- [x] Null raw-pointer constants, pointer `Eq`/`Ne`, and thin `PtrToPtr` copy.
- [x] Complete target descriptor and rustc DataLayout serialization with
  worker-side contract validation.
- [x] `TypeLayoutRecipe` wire schema and backend lowering for size/alignment,
  scalar valid ranges, fields, variants, and niches.
- [x] `FnAbiPlan` wire schema carrying rustc layout, calling convention,
  variadic/unwind flags, and Ignore/Direct/Pair/Cast/Indirect pass modes.
- [x] Worker ABI boundary tests rejecting Pair/Cast/Indirect before object
  emission.
- [x] Standard test gate runs worker unit tests.

## M0 Protocol And ABI

- [x] Serialize complete target descriptor and rustc DataLayout.
- [x] Add `TypeLayoutRecipe` with size, alignment, fields, variants, and niches.
- [x] Add `FnAbiPlan` with Ignore, Direct, Pair, Cast, and Indirect pass modes.
- [x] Add worker-level negative tests for unsupported non-Direct ABI modes.
- [ ] Implement Pair/Cast/Indirect ABI lowering and object emission.
- [ ] Build 20-30 bidirectional C/LLVM ABI fixtures.
- [ ] Add structured diagnostic codes and rustc span/MIR locations.

## M1 Trusted Backend MVP

- [ ] Add pointer load/store and target-qualified memory operation plans.
- [ ] Add stack allocations with size/alignment validation.
- [ ] Add indirect calls with explicit function signatures.
- [ ] Add aggregate argument/return ABI, including sret/byval.
- [ ] Add direct SAB no-fallback emission from the canonical plan.
- [ ] Add proof/work-product manifest and content-addressed reuse.

## M2 no_std Rust

- [ ] Add static allocation images, relocations, strings, and panic metadata.
- [ ] Add enums, discriminants, niches, arrays, and aggregate memory layout.
- [ ] Add references, raw-pointer operations, slices, fat pointers, and vtables.
- [ ] Add drop glue, partial moves, and initialization-state tracking.
- [ ] Add intrinsic registry with hard errors for unsupported intrinsics.

## M3-M6 Productization

- [ ] Cargo host/target split and workspace driver.
- [ ] SCI-built `core`, then `alloc`, then restricted `std`.
- [ ] `wasm32-unknown-unknown`, then `wasm32-wasip1` object emission.
- [ ] Unwind, TLS, atomics, SIMD, asm, debug info, coverage, sanitizer, and LTO.
- [ ] Strict proof sidecar/refinement certificate and linked-image validation.

All unchecked capabilities must remain hard errors before object publication.
