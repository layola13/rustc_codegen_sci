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
- Current worktree: `PLAN_VERSION = 9` `TypeLayoutRecipe` wire schema and
  backend lowering for size/alignment, scalar valid ranges, fields, variants,
  and niches.

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

## Current Boundary

The backend supports direct pointer values, serializes rustc ABI evidence,
serializes the current x86_64 Linux target descriptor/DataLayout, and carries
monomorphized type layout recipes, but not dereference, load/store,
provenance-changing casts, nonzero pointer constants, allocations, relocations,
or non-Direct ABI lowering.
Aggregate ABI, sysroot, Cargo productization, WASM, direct SAB, and strict proof
remain incomplete.
