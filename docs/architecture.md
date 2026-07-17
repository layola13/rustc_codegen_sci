# Architecture

```text
rustc frontend / borrowck / monomorphization
  -> rustc_codegen_sci dylib
  -> canonical SciModulePlan
  -> framed SCI worker RPC v2
  -> SA text generated from the canonical plan
  -> SCI Referee and native emitter
  -> relocatable object
  -> rustc archive/link pipeline
```

## Boundaries

- `crates/rustc_codegen_sci` is the rustc-private adapter. It is rebuilt for
  every supported rustc commit and does not link SCI or LLVM into rustc.
- `crates/sci_protocol` owns the stable, deterministic plan and RPC encoding.
  The rustc adapter and worker consume the same definitions.
- `crates/sci_codegen_worker` is the process boundary around SCI. It validates
  protocol, target, feature, and plan invariants before invoking SCI.
- The worker emits SA text only from `SciModulePlan`. Direct SAB v5 will consume
  the same plan and must not acquire separate Rust lowering logic.

## Trust Modes

- `rust-trusted` trusts rustc's language and borrow checking decisions while
  still requiring SCI Referee acceptance.
- `sa-verified-subset` is reserved and rejected until borrowck sidecars and MIR
  refinement certificates are complete.

## No Fallback

The selected emitter either returns a verified object or a structured error.
It never calls rustc's LLVM backend and never routes through LLVM bitcode or
`bc2sa`.
