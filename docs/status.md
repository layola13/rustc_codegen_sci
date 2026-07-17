# Implementation Status

Baseline date: 2026-07-16.

## Implemented

- Independent workspace and pinned toolchain manifest.
- Rustc-private backend dylib entry point.
- Early target, panic, LTO, coverage, crate type, and float capability gates.
- Canonical SCI lowering-plan data model.
- Complete x86_64 Linux target descriptor serialization, including object
  format, rustc DataLayout, CPU/features, relocation model, and code model.
- `TypeLayoutRecipe` serialization for monomorphized rustc layouts, including
  size/alignment, field shape, variant/tag encoding, largest niche, and scalar
  valid ranges.
- rustc-derived `FnAbiPlan` metadata serialized for defined and extern
  functions, including layout, calling convention, variadic/unwind flags, and
  Ignore/Direct/Pair/Cast/Indirect pass mode tags.
- Worker unit coverage for the ABI boundary: Direct/Ignore is accepted and
  Pair/Indirect plus unsupported Cast cases are rejected before object
  publication.
- Backend compile-fail smoke coverage for real rustc Pair/Cast/Indirect
  pass-mode rejection before MIR lowering/object emission.
- Narrow Cast ABI return lowering for single-field signed and unsigned
  8/16/32/64-bit aggregates returned in one integer register, with linked C
  smoke coverage.
- Versioned framed worker RPC with bounded frame sizes.
- Backend-originated lowering diagnostics annotated with MIR block and
  statement/terminator context.
- Backend-originated fatal diagnostics classified with stable `SCI_BACKEND_*`
  codes.
- Backend-originated MIR statement/terminator lowering diagnostics emitted with
  rustc source spans.
- Worker-side target descriptor, type layout recipe, and plan validation.
- SA text emitter from the canonical plan.
- SCI `build-obj` process boundary.
- Rustc CGU traversal, MIR lowering, worker invocation, and object artifact
  return to rustc's normal output pipeline.
- Smoke coverage for scalar addition, signed comparison, bool-to-int cast,
  `isize`/`usize`, raw pointer direct ABI, direct scalar/raw-pointer function
  calls, scalar/raw-pointer extern C calls, unit/void returns and calls,
  if/else CFG, MIR assert abort paths,
  signed/unsigned scalar integer `SwitchInt`/`match`, division/remainder,
  shifts, unary integer negation/bit-not, checked add/sub/mul overflow tuple
  lowering through 64-bit integers, local scalar tuple/struct construction and
  field projection, local scalar aggregate copy/move, function-internal empty
  struct ZST locals, raw pointer null/equality/inequality and thin `PtrToPtr`
  copies, scalar raw-pointer field and fixed array-index load/store, SCI object
  emission, native link, and execution.

## Bring-up Capability

| Area | Current support |
| --- | --- |
| Target | `x86_64-unknown-linux-gnu` with explicit ELF object format, rustc DataLayout, `x86-64` CPU, empty target features, PIC relocation model, and default code model |
| Panic | `abort` |
| Crates | `rlib`, object emission |
| Function ABI | scalar integer, raw pointer, and void C/Rust ABI with rustc-derived `FnAbiPlan`; Ignore/Direct pass modes are accepted; single-field signed and unsigned 8/16/32/64-bit Cast aggregate returns are lowered through the scalar return register; Pair/Indirect and unsupported Cast cases are serialized but rejected until implemented; backend preflight rejects unsupported non-Direct definitions before MIR lowering; simple scalar raw-pointer deref/load/store, scalar field projection, and fixed scalar array-index projection are supported |
| Type Layout | monomorphized rustc `LayoutData` recipes for local and extern signature types, including size/alignment, fields, variants, niches, and scalar valid ranges |
| MIR CFG | multiple blocks with `return`, `goto`, bool `SwitchInt`/`br`, signed/unsigned scalar integer `SwitchInt` compare-chain emission, and `Assert` abort paths |
| MIR calls | direct module-local scalar/raw-pointer/void function calls and direct scalar/raw-pointer/void `extern "C"` calls with unreachable unwind |
| MIR rvalues | `Use`, scalar raw-pointer load/store including simple field offsets and fixed array element offsets, scalar tuple/struct `Aggregate`, local scalar aggregate copy/move, no-op empty struct local construction, integer arithmetic/bitwise/div/rem/shift `BinaryOp`, checked add/sub/mul `(value, overflow)` tuple lowering through 64-bit integers, integer and pointer `Eq`/`Ne`, integer `UnaryOp` negation/bit-not, integer `IntToInt` casts, thin `PtrToPtr` copies |
| Values | integer/bool/raw-pointer locals, integer/bool constants, and null pointer constants, including `isize`/`usize` lowered through the active target pointer width |
| SCI format | SA text generated from canonical plan |
| Proof mode | `rust-trusted` |

Every missing capability is rejected before object publication.

## Next Required Work

The full staged roadmap is tracked in `docs/implementation_plan_cn.md`; live
execution state is tracked in `tasks.md`, `progress.md`, and `current_plan.md`.

1. Indirect/ABI-rich calls, tuple argument/return ABI, and wider scalar
   operation coverage.
2. Complete `TyAndLayout` recipes, Pair/Cast/Indirect ABI lowering,
   aggregates, allocations, relocations, statics, and drop glue.
3. SAB v5 in SCI and direct SAB emission.
4. Direct relocatable Wasm emitters for `wasm32-unknown-unknown` and
   `wasm32-wasip1`.
5. Allocator shim, `core/alloc/std`, unwind, TLS, SIMD, asm, debug info, LTO,
   sanitizer, and coverage parity.
6. Proof-aware incremental reuse and the strict proof track.
