# Implementation Status

Baseline date: 2026-07-16.

## Implemented

- Independent workspace and pinned toolchain manifest.
- Rustc-private backend dylib entry point.
- Early target, panic, LTO, coverage, crate type, and float capability gates.
- Canonical SCI lowering-plan data model.
- Versioned framed worker RPC with bounded frame sizes.
- Worker-side target and plan validation.
- SA text emitter from the canonical plan.
- SCI `build-obj` process boundary.
- Rustc CGU traversal, MIR lowering, worker invocation, and object artifact
  return to rustc's normal output pipeline.
- Smoke coverage for scalar addition, signed comparison, bool-to-int cast,
  `isize`/`usize`, direct scalar function calls, scalar extern C calls,
  unit/void returns and calls, if/else CFG, MIR assert abort paths,
  signed/unsigned scalar integer `SwitchInt`/`match`, division/remainder,
  shifts, unary integer negation/bit-not, checked add/sub/mul overflow tuple
  lowering through 64-bit integers, local scalar tuple/struct construction and
  field projection, SCI object emission, native link, and execution.

## Bring-up Capability

| Area | Current support |
| --- | --- |
| Target | `x86_64-unknown-linux-gnu` |
| Panic | `abort` |
| Crates | `rlib`, object emission |
| Function ABI | scalar integer and void C/Rust ABI with direct pass modes, including `isize`/`usize` on 64-bit targets |
| MIR CFG | multiple blocks with `return`, `goto`, bool `SwitchInt`/`br`, signed/unsigned scalar integer `SwitchInt` compare-chain emission, and `Assert` abort paths |
| MIR calls | direct module-local scalar/void function calls and direct scalar/void `extern "C"` calls with unreachable unwind |
| MIR rvalues | `Use`, scalar tuple/struct `Aggregate`, integer arithmetic/bitwise/div/rem/shift `BinaryOp`, checked add/sub/mul `(value, overflow)` tuple lowering through 64-bit integers, integer comparisons, integer `UnaryOp` negation/bit-not, integer `IntToInt` casts |
| Values | integer/bool locals and integer/bool constants, including `isize`/`usize` lowered through the active target pointer width |
| SCI format | SA text generated from canonical plan |
| Proof mode | `rust-trusted` |

Every missing capability is rejected before object publication.

## Next Required Work

The full staged roadmap is tracked in `docs/implementation_plan_cn.md`.

1. Indirect/ABI-rich calls, tuple argument/return ABI, and wider scalar
   operation coverage.
2. Complete `TyAndLayout` and `FnAbi` serialization, aggregates, allocations,
   relocations, statics, and drop glue.
3. SAB v5 in SCI and direct SAB emission.
4. Direct relocatable Wasm emitters for `wasm32-unknown-unknown` and
   `wasm32-wasip1`.
5. Allocator shim, `core/alloc/std`, unwind, TLS, SIMD, asm, debug info, LTO,
   sanitizer, and coverage parity.
6. Proof-aware incremental reuse and the strict proof track.
