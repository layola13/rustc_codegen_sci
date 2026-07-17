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
- Current worktree: auditable 33-case Direct scalar ABI suite with signed and
  unsigned narrow-integer boundary values.

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

## Current Boundary

The backend supports direct pointer values, serializes rustc ABI evidence,
serializes the current x86_64 Linux target descriptor/DataLayout, and carries
monomorphized type layout recipes, but not dereference, load/store,
provenance-changing casts, nonzero pointer constants, allocations, relocations,
or non-Direct ABI lowering.
Worker tests now cover the current serialized ABI and layout validation
boundary, and the smoke suite now has 33 linked Direct scalar ABI cases. The
broader bidirectional C/LLVM ABI suite still needs non-Direct and aggregate
coverage.
Aggregate ABI, sysroot, Cargo productization, WASM, direct SAB, and strict proof
remain incomplete.
