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
- [x] Backend FnAbi preflight rejects unsupported Pair/Cast/Indirect pass modes
  with source spans before MIR lowering/object emission.
- [x] First bidirectional Cast ABI lowering: single-field signed and unsigned
  8/16/32/64-bit aggregate arguments/returns emitted as cast scalar registers
  with linked C smoke coverage.
- [x] Worker-level ABI/type-layout fixture matrix for the current serialized
  validation boundary.
- [x] Linked Direct scalar ABI smoke fixture matrix covering C-to-SCI exports and
  SCI-to-C extern calls.
- [x] Auditable 33-case linked Direct scalar ABI suite with signed/unsigned
  narrow-integer boundary values.
- [x] Wire-level worker diagnostic codes and coarse function/block/local
  diagnostic locations.
- [x] Backend-originated lowering errors annotated with MIR block and
  statement/terminator context, covered by compile-fail smoke.
- [x] Backend-originated fatal diagnostics classified with stable
  `SCI_BACKEND_*` codes.
- [x] Backend-originated MIR statement/terminator lowering errors emitted with
  rustc source spans.
- [x] Protocol-level `DiagnosticPayload` shared by worker RPC responses and
  backend fatal diagnostics, including stable code and optional location.
- [x] `PLAN_VERSION = 11` scalar raw-pointer `Load`/`Store` memory operations
  plus stack allocations with size/alignment validation.
- [x] Scalar raw-pointer field-offset load/store for simple `repr(C)` aggregate
  pointees.
- [x] Scalar raw-pointer fixed array-index load/store using rustc array layout
  offsets.
- [x] Standard test gate runs worker unit tests.

## M0 Protocol And ABI

- [x] Serialize complete target descriptor and rustc DataLayout.
- [x] Add `TypeLayoutRecipe` with size, alignment, fields, variants, and niches.
- [x] Add `FnAbiPlan` with Ignore, Direct, Pair, Cast, and Indirect pass modes.
- [x] Add worker-level negative tests for unsupported non-Direct ABI modes.
- [x] Add backend compile-fail fixtures for real rustc Pair/Cast/Indirect
  pass-mode rejection.
- [x] Add linked bidirectional Cast ABI fixtures for single-field signed and
  unsigned 8/16/32/64-bit aggregate arguments/returns.
- [x] Add initial linked bidirectional Direct scalar ABI fixtures.
- [x] Count and execute 20+ linked Direct ABI fixture cases in the smoke gate.
- [x] Add structured worker diagnostic codes to RPC responses.
- [x] Add backend MIR block/statement context to lowering diagnostics.
- [x] Add stable backend diagnostic codes to rustc fatal messages.
- [x] Add rustc source spans for backend MIR statement/terminator lowering
  diagnostics.
- [x] Add protocol-level structured backend diagnostic payloads.
- [x] Add stack allocations with size/alignment validation.
- [ ] Implement Pair/Cast/Indirect ABI lowering and object emission.
- [ ] Build 20-30 bidirectional C/LLVM ABI fixtures.

## M1 Trusted Backend MVP

- [x] Add scalar raw-pointer load/store and target-qualified memory operation
  plans.
- [x] Add raw-pointer scalar field load/store using rustc field layout offsets.
- [x] Add raw-pointer scalar fixed array-index load/store using rustc array
  layout offsets.
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
