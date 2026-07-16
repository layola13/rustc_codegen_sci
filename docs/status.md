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
  direct scalar function calls, scalar extern C calls, if/else CFG, MIR assert
  abort paths, scalar integer `SwitchInt`/`match`, division/remainder, shifts,
  unary integer negation/bit-not, checked add/sub/mul overflow tuple lowering,
  SCI object emission, native link, and execution.

## Bring-up Capability

| Area | Current support |
| --- | --- |
| Target | `x86_64-unknown-linux-gnu` |
| Panic | `abort` |
| Crates | `rlib`, object emission |
| Function ABI | scalar integer C/Rust ABI with direct pass modes |
| MIR CFG | multiple blocks with `return`, `goto`, bool `SwitchInt`/`br`, scalar integer `SwitchInt` compare-chain emission, and `Assert` abort paths |
| MIR calls | direct module-local scalar function calls and direct scalar `extern "C"` calls with unreachable unwind |
| MIR rvalues | `Use`, integer arithmetic/bitwise/div/rem/shift `BinaryOp`, checked add/sub/mul `(value, overflow)` tuple lowering, integer comparisons, integer `UnaryOp` negation/bit-not, integer `IntToInt` casts |
| Values | integer/bool locals and integer/bool constants |
| SCI format | SA text generated from canonical plan |
| Proof mode | `rust-trusted` |

Every missing capability is rejected before object publication.

## Next Required Work

The full staged roadmap is tracked in `docs/implementation_plan_cn.md`.

1. Indirect/ABI-rich calls, checked multiplication for 64-bit integers, signed
   negative `SwitchInt` edge cases, and wider scalar operation coverage.
2. Complete `TyAndLayout` and `FnAbi` serialization, aggregates, allocations,
   relocations, statics, and drop glue.
3. SAB v5 in SCI and direct SAB emission.
4. Direct relocatable Wasm emitters for `wasm32-unknown-unknown` and
   `wasm32-wasip1`.
5. Allocator shim, `core/alloc/std`, unwind, TLS, SIMD, asm, debug info, LTO,
   sanitizer, and coverage parity.
6. Proof-aware incremental reuse and the strict proof track.
