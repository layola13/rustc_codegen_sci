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
`no_std`, scalar integer and void function signatures including
`isize`/`usize`, straight-line MIR assignments, scalar integer arithmetic,
signed/unsigned comparisons, integer casts, direct scalar function calls,
scalar `extern "C"` calls, unit/void returns and calls, MIR assert abort paths,
division/remainder, shifts, unary integer negation/bit-not, multi-block bool
branch CFG, and scalar integer `SwitchInt`/`match` lowered through
worker-generated compare chains. With overflow checks enabled, checked integer
add/sub/mul are lowered through synthetic `(value, overflow)` tuple fields
before MIR `Assert`; checked mul is supported for integer widths up to 64
bits. Local scalar tuples and structs can be constructed and read through
field projection, and local scalar aggregate copy/move is lowered field by
field. Function-internal empty struct ZST locals are ignored as no-op values;
aggregate and ZST struct argument/return ABI is still rejected. Unsupported
targets, ABIs, MIR operations, and features are hard errors. There is no
LLVM-backend or `bc2sa` fallback.

Build and run the focused smoke gate:

```bash
./scripts/test.sh
```

See `docs/status.md` for the exact implemented capability matrix and
`docs/architecture.md` for component boundaries. The staged implementation
roadmap is in `docs/implementation_plan_cn.md`.
