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

## Current Increment

- Added worker unit coverage for the ABI validation boundary.
- Confirmed Direct arguments with Ignore returns validate.
- Confirmed Pair, Cast, and Indirect argument pass modes hard-error before SA
  or object emission.
- Extended `./scripts/test.sh` so worker unit tests run in the standard gate.
- Verified with pinned rustfmt and `./scripts/test.sh`.

## Current Boundary

The backend supports direct pointer values and serializes rustc ABI evidence,
but not dereference, load/store, provenance-changing casts, nonzero pointer
constants, allocations, relocations, or non-Direct ABI lowering. Aggregate ABI,
sysroot, Cargo productization, WASM, direct SAB, and strict proof remain
incomplete.
