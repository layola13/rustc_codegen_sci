# rustc_codegen_sci Progress

Baseline: 2026-07-16.

## Implemented

- `ff65f7f`: initial rustc-private backend, worker, canonical plan, SCI object
  emission, and native smoke link/run.
- `c861e67` through `7c235d1`: checked arithmetic, extern C scalar calls,
  signed switch lowering, 64-bit multiplication, and pointer-sized integers.
- `c8f0fa1` through `0419a0e`: void calls, local scalar tuple/struct lowering,
  aggregate copy/move, and local empty-struct ZST values.
- `5535a3c`: protocol-level `ptr` type and direct raw-pointer local/extern ABI.
- `93f2f84`: null raw-pointer constants, pointer equality/inequality, thin
  `PtrToPtr` copies, and project-local task/progress/current-plan tracking.
- `f7d864e`: rustc-derived `FnAbiPlan` serialization on defined/extern
  functions and worker validation for currently implemented ABI modes.
- `6a169b4`: `PLAN_VERSION = 8` target descriptor serialization covering object
  format, rustc DataLayout, CPU/features, relocation model, and code model,
  with worker-side contract validation.
- `cb534fe`: `PLAN_VERSION = 9` `TypeLayoutRecipe` wire schema and backend
  lowering for size/alignment, scalar valid ranges, fields, variants, and
  niches.
- `9e31b38`: worker-level ABI/type-layout fixture matrix for the current
  validation boundary, plus ABI value size/alignment validation.
- `4635c76`: linked Direct scalar ABI smoke fixture matrix covering C-to-SCI
  exported functions and SCI-to-C extern calls.
- `544be0e`: auditable 33-case Direct scalar ABI suite with signed and unsigned
  narrow-integer boundary values.
- `302c770`: RPC v2 worker diagnostic codes and coarse diagnostic locations.
- `f8958ae`: `PLAN_VERSION = 10` scalar raw-pointer load/store memory
  operations.
- `36f7b3c`: scalar raw-pointer field-offset load/store for simple aggregate
  pointees.
- `fcdcd59`: scalar raw-pointer fixed array-index load/store using rustc array
  layout offsets.
- `f143446`: backend-originated lowering errors carry MIR block and
  statement/terminator context.
- `a093e1a`: backend-originated fatal diagnostics are classified with stable
  `SCI_BACKEND_*` codes.
- `7efc32b`: backend-originated MIR statement/terminator lowering errors emit
  rustc source spans.
- `b7f5308`: backend FnAbi preflight rejects unsupported Pair/Cast/Indirect pass
  modes before MIR lowering.
- `9f821b6`: single-field signed and unsigned 8/16/32/64-bit Cast ABI
  arguments and returns lower through scalar registers.
- `d76760b`: protocol-level `DiagnosticPayload` shared by worker RPC responses
  and backend fatal diagnostics.
- `cc094a9`: `PLAN_VERSION = 11` scalar stack allocations with size/alignment
  validation and address-taken local lowering through canonical stack slots.
- `066f3ed`: `PLAN_VERSION = 12` scalar `extern "C"` function pointer
  indirect calls lower through canonical `CallIndirect` terminators carrying
  explicit scalar argument/return signatures.
- Current increment: initial rust-trusted work-product manifests plus
  content-addressed object reuse keyed by canonical plan bytes, cache policy,
  and SCI identity.

## Current Increment

- Upgraded the canonical protocol to `PLAN_VERSION = 9`.
- Added `TypeLayoutRecipe` with size, alignment, uninhabited flag, field shape,
  variant/tag encoding, largest niche, and backend scalar valid ranges.
- Lowered monomorphized MIR local layouts and extern signature layouts from
  rustc `LayoutData` into every `SciModulePlan`.
- Added worker-side layout recipe validation for malformed field memory order,
  bad alignment, duplicate type recipes, malformed variant metadata, and empty
  scalar primitive names.
- Avoided rustc pretty type printing in layout keys to keep codegen out of
  `trimmed_def_paths` diagnostic state.
- Added table-driven worker fixtures for supported Direct/Ignore ABI shapes,
  rejected Pair/Cast/Indirect ABI modes, mismatched lowered argument/return
  boundaries, malformed ABI size/alignment, and representative primitive,
  struct, union, array, empty, and niche enum type layouts.
- ABI value validation now checks every serialized ABI size/alignment pair before
  mode-specific checks.
- Added `abi_direct` smoke fixtures that compile through `rustc_codegen_sci`,
  link against a C harness, and execute Direct ABI checks for signed/unsigned
  8/16/32/64-bit integers, pointer identity, `isize`/`usize`, void returns, and
  host extern calls in the reverse direction.
- Extended `tests/smoke.sh` so each fixture is compiled, linked, and executed
  through the same backend/worker path.
- Expanded `abi_direct` into a counted 33-case harness, adding signed i8/i16
  negative round trips and unsigned u8/u16 high-bit round trips in both
  C-to-SCI and SCI-to-C directions.
- Upgraded worker RPC responses to carry a structured diagnostic code and
  optional function/block/local diagnostic location alongside the existing
  message.
- Classified worker rejections into ABI, target, layout, CFG, IO, object
  emission, and generic rejection codes, and included those fields in backend
  rustc fatal messages.
- Added canonical scalar `Load`/`Store` memory operations with pointer value,
  byte offset, scalar type, and alignment.
- Lowered simple raw-pointer dereference reads and writes (`*p` and `*p = v`)
  for scalar pointee types, with worker validation and SA `load`/`store`
  emission.
- Extended the smoke harness with C-provided `i32` pointer load, store, and
  replace cases that compile through `rustc_codegen_sci`, link, and execute.
- Extended memory place lowering to accumulate rustc field layout offsets after
  a raw-pointer dereference, enabling `(*p).field` scalar loads/stores.
- Added linked C/Rust smoke coverage for reading and writing the second field of
  a `repr(C)` two-`i32` aggregate through a raw pointer.
- Extended memory place lowering to resolve fixed array element offsets after a
  raw-pointer dereference, including MIR `Index(_temp)` when `_temp` has a
  single constant `usize` assignment.
- Added linked C/Rust smoke coverage for loading, storing, and replacing `i32`
  elements in a C-provided `[i32; 4]` pointer, with generated SA offsets
  `+4`, `+8`, and `+12`.
- Wrapped backend statement and terminator lowering errors with precise MIR
  block plus statement index or terminator context, while preserving the
  existing function-name prefix.
- Added a compile-fail smoke fixture for unsupported reference rvalues that
  asserts the backend diagnostic includes `block 0 statement 0`.
- Classified backend-originated fatal diagnostics into stable
  `SCI_BACKEND_*` codes, while preserving worker rejection codes without a
  second wrapper.
- Extended the compile-fail smoke fixture to assert
  `SCI_BACKEND_MIR_UNSUPPORTED` for unsupported MIR lowering.
- Promoted backend diagnostics from plain strings to an internal diagnostic
  record that can carry a rustc `Span`.
- Emitted statement and terminator lowering failures through `span_fatal` using
  MIR `SourceInfo`, and extended the compile-fail smoke fixture to assert the
  source file and line are present.
- Added backend FnAbi preflight over the rustc-derived `FnAbiPlan` so unsupported
  Pair/Cast/Indirect pass modes are rejected with backend ABI diagnostic codes
  and definition spans before local/MIR lowering.
- Added compile-fail smoke fixtures covering a real Cast C ABI return, a real
  Indirect C ABI return, and a real Pair Rust ABI return from the current
  rustc/x86_64 ABI classifier.
- Added a narrow Cast ABI implementation for aggregate returns that are exactly
  one scalar field and whose rustc Cast recipe is a single integer register no
  larger than 8 bytes, restricted to 1/2/4/8-byte scalar widths.
- Reused the aggregate synthetic-field lowering for the return place so the
  canonical plan returns the scalar field local while retaining the rustc
  `FnAbiPlan` Cast evidence.
- Updated worker ABI validation to accept only that scalar Cast
  argument/return shape; Pair, Indirect, and wider Cast cases remain rejected.
- Converted the former Cast compile-fail fixture into a linked C smoke fixture
  that calls SCI functions returning single-field integer structs and verifies
  the returned values.
- Expanded the linked Cast smoke fixture to cover single-field signed and
  unsigned 8/16/32/64-bit aggregate returns, and extended worker validation
  tests to accept only the 1/2/4/8-byte scalar Cast argument/return widths.
- Added matching single-field signed and unsigned 8/16/32/64-bit Cast aggregate
  argument lowering and linked C smoke coverage; aggregate argument locals now
  map to their synthetic scalar field local.
- Extended extern call lowering so SCI-to-C calls can pass and receive the same
  narrow Cast aggregate shapes through scalar call operands/destinations, with
  linked host C smoke coverage.
- Upgraded the worker RPC response wire shape to `RPC_VERSION = 3`, replacing
  split diagnostic strings with `DiagnosticPayload { code, message, location }`.
- Reused the same protocol-level diagnostic payload inside backend-originated
  fatal diagnostics, including explicit function locations for ABI preflight
  rejections and function/block locations for MIR lowering failures.
- Updated compile-fail smoke expectations so backend ABI and MIR diagnostics
  assert the structured location rendered from the payload.
- Upgraded the canonical protocol to `PLAN_VERSION = 11` and introduced
  `StackAlloc` operations for canonical stack slots, with backend lowering and
  worker validation for size/alignment.
- Added linked smoke coverage for stack-backed scalar locals that lower through
  canonical stack slots and still round-trip through load/store on the same
  local value.
- Upgraded the canonical protocol to `PLAN_VERSION = 12` and introduced
  `CallSignaturePlan` plus `TerminatorPlan::CallIndirect` for indirect calls
  with explicit scalar argument/return signatures.
- Added worker validation for indirect calls: callee must be a defined `ptr`,
  argument values must match the explicit signature, and the destination must
  match the signature return type.
- Added worker SA emission for `call_indirect` from the canonical terminator,
  plus unit coverage for accepted and rejected indirect-call signatures.
- Lowered scalar `extern "C"` function pointer calls from MIR to canonical
  `CallIndirect`, while rejecting variadic, unwinding, non-C, aggregate, and
  unsupported pass-mode function pointer signatures before object publication.
- Added linked C smoke coverage passing a host callback into SCI-compiled Rust
  and verifying the indirect call result.
- Added worker-side rust-trusted work-product manifests recording schema,
  cache policy, plan version, rustc commit, target, CGU name, plan hash,
  work-product hash, object hash, SCI binary path, and SCI identity.
- Added content-addressed worker object reuse under
  `SCI_CODEGEN_CACHE_DIR` or `target/sci-cache`, keyed by canonical module wire
  bytes plus SCI identity and cache policy.
- Cache hits validate the cached manifest, plan hash, work-product hash, and
  object hash before publishing the cached object to rustc's requested output
  path; the output manifest records `cache_hit: true`.
- Added worker unit coverage for cached object/manifest publication, and smoke
  evidence that a second run with the same cache emits cache-hit manifests.

## Current Boundary

The backend supports direct pointer values, serializes rustc ABI evidence,
serializes the current x86_64 Linux target descriptor/DataLayout, and carries
monomorphized type layout recipes. It now supports simple scalar raw-pointer
load/store dereferences, scalar field projections after raw-pointer dereference,
and fixed scalar array-index projections after raw-pointer dereference. It does
not yet support dynamic array indices, slices, whole-aggregate memory copies,
provenance-changing casts, nonzero pointer constants, allocations, relocations,
or general non-Direct ABI lowering. The implemented non-Direct ABI cases are
single-field signed and unsigned 8/16/32/64-bit Cast aggregate arguments and
returns; unsupported Pair/Indirect and unsupported Cast cases are rejected in
backend FnAbi preflight before MIR lowering/object emission.
Worker tests now cover the current serialized ABI and layout validation
boundary, and the smoke suite now has 33 linked Direct scalar ABI cases plus
bidirectional narrow Cast aggregate argument/return coverage. The broader
C/LLVM ABI suite still needs Pair/Indirect, sret/byval, and wider aggregate
coverage.
Worker failures and backend-originated fatal diagnostics now share a
protocol-level `DiagnosticPayload` carrying stable code plus optional
function/block/local location. Backend-originated lowering failures also include
MIR block and statement/terminator context plus rustc source spans for
statement/terminator lowering failures. Stack-backed scalar locals now lower
through canonical `stack_alloc` slots instead of needing a separate memory model.
Scalar `extern "C"` function pointer calls now lower through canonical
`CallIndirect` terminators with explicit scalar signatures; aggregate or
non-C/variadic/unwinding function pointer calls remain hard errors. Aggregate
ABI, sysroot, Cargo productization, WASM, direct SAB, and strict proof remain
incomplete. The work-product cache is a rust-trusted reuse gate and does not yet
include a strict ownership proof sidecar or linked-image validator.
