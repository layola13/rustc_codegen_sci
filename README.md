# rustc_codegen_sci

`rustc_codegen_sci` is an out-of-tree Rust codegen backend that lowers
monomorphized rustc MIR into a canonical SCI plan, sends that plan to a
versioned worker, runs the SCI Referee, and returns native object files to
rustc's normal archive and link pipeline.

The backend is pinned to:

- rustc commit `fcbe7917ba18120d9eda136f1c7c5a60c78e554e`
- `rustc 1.99.0-nightly`
- LLVM `22.1.8`
- SCI `0.0.4`

The current bring-up slice supports `x86_64-unknown-linux-gnu`, `panic=abort`,
`no_std`, scalar integer, raw pointer, and void function signatures including
`isize`/`usize`, straight-line MIR assignments, scalar integer arithmetic,
signed/unsigned comparisons, integer casts, direct scalar/raw-pointer function
calls, scalar/raw-pointer `extern "C"` calls, unit/void returns and calls, MIR
assert abort paths, division/remainder, shifts, unary integer negation/bit-not,
multi-block bool branch CFG, and scalar integer `SwitchInt`/`match` lowered through
worker-generated compare chains. With overflow checks enabled, checked integer
add/sub/mul are lowered through synthetic `(value, overflow)` tuple fields
before MIR `Assert`; checked mul is supported for integer widths up to 64
bits. Local scalar tuples and structs can be constructed and read through
field projection, and local scalar aggregate copy/move is lowered field by
field. Function-internal empty struct ZST locals are ignored as no-op values;
raw pointer null constants, equality/inequality, and thin pointer-to-pointer
copies are supported. The canonical plan now carries a complete target
descriptor for the current x86_64 Linux slice, including rustc DataLayout,
object format, CPU/features, relocation model, and code model, plus
monomorphized `TypeLayoutRecipe` records and rustc-derived `FnAbiPlan` metadata
for function definitions and extern calls. The currently implemented ABI modes
are Ignore/Direct plus narrow single-field signed and unsigned 8/16/32/64-bit
Cast aggregate returns that map to one integer register; unsupported
Pair/Indirect and unsupported Cast cases are rejected before object publication.
The backend also preflights rustc `FnAbiPlan` pass modes and rejects unsupported
definitions before MIR lowering. Simple scalar raw-pointer load/store
dereference, scalar field projection through a raw pointer, and fixed scalar
array-index projection through a raw pointer are supported; dynamic indices,
slices, and whole-aggregate memory operations are not supported yet.
General aggregate and ZST struct argument/return ABI is still rejected.
Unsupported targets, ABIs, MIR operations, and features are hard errors. There
is no LLVM-backend or `bc2sa` fallback. Worker rejections carry structured
diagnostic codes and coarse locations. Backend-originated fatal diagnostics
carry stable `SCI_BACKEND_*` codes and lowering errors include MIR
block/statement or block/terminator context plus rustc source spans for
statement/terminator lowering failures.

Build and run the focused smoke gate:

```bash
./scripts/test.sh
```

See `docs/status.md` for the exact implemented capability matrix and
`docs/architecture.md` for component boundaries. The staged implementation
roadmap is in `docs/implementation_plan_cn.md`.

Live execution tracking is maintained in `tasks.md`, `progress.md`, and
`current_plan.md`.
