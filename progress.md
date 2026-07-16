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

## Current Increment

- Added raw-pointer null constants emitted as `value = 0 as ptr`.
- Added worker-validated pointer equality/inequality while retaining hard errors
  for ordered pointer comparisons and pointer arithmetic.
- Added thin pointer `PtrToPtr` MIR casts as type-preserving copies.
- Added C-linked smoke coverage for pointer equality, null tests, and null
  pointer returns.
- Verified with pinned rustfmt and `./scripts/test.sh`.

## Current Boundary

The backend supports direct pointer values but not dereference, load/store,
provenance-changing casts, nonzero pointer constants, allocations, or
relocations. Aggregate ABI, sysroot, Cargo productization, WASM, direct SAB,
and strict proof remain incomplete.
