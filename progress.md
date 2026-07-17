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
- Current worktree: `PLAN_VERSION = 8` target descriptor serialization covering
  object format, rustc DataLayout, CPU/features, relocation model, and code
  model, with worker-side contract validation.

## Current Increment

- Upgraded the canonical protocol to `PLAN_VERSION = 8`.
- Extended `TargetPlan` beyond triple/pointer/endian to carry object format,
  rustc DataLayout, CPU, target features, relocation model, and code model.
- Lowered the rustc session target descriptor into every `SciModulePlan`.
- Added backend gates for custom target CPU/features that are not implemented
  by the current x86_64 bring-up slice.
- Added worker validation for the complete supported target contract and unit
  coverage for accepted descriptors and DataLayout mismatch rejection.

## Current Boundary

The backend supports direct pointer values, serializes rustc ABI evidence, and
serializes the current x86_64 Linux target descriptor/DataLayout, but not
dereference, load/store, provenance-changing casts, nonzero pointer constants,
allocations, relocations, TypeLayoutRecipe, or non-Direct ABI lowering.
Aggregate ABI, sysroot, Cargo productization, WASM, direct SAB, and strict proof
remain incomplete.
