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
- Current increment: backend-originated lowering errors carry MIR block and
  statement/terminator context.

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

## Current Boundary

The backend supports direct pointer values, serializes rustc ABI evidence,
serializes the current x86_64 Linux target descriptor/DataLayout, and carries
monomorphized type layout recipes. It now supports simple scalar raw-pointer
load/store dereferences, scalar field projections after raw-pointer dereference,
and fixed scalar array-index projections after raw-pointer dereference. It does
not yet support dynamic array indices, slices, whole-aggregate memory copies,
provenance-changing casts, nonzero pointer constants, allocations, relocations,
or non-Direct ABI lowering.
Worker tests now cover the current serialized ABI and layout validation
boundary, and the smoke suite now has 33 linked Direct scalar ABI cases. The
broader bidirectional C/LLVM ABI suite still needs non-Direct and aggregate
coverage.
Worker failures now have structured RPC diagnostic codes and coarse parsed
locations. Backend-originated lowering failures now include MIR block and
statement/terminator context, while rustc spans and fully structured backend
diagnostic codes are still pending.
Aggregate ABI, sysroot, Cargo productization, WASM, direct SAB, and strict proof
remain incomplete.
