# rustc_codegen_sci 完整实施计划

基线日期：2026-07-16。

本文以当前仓库实现为起点，不再讨论 `rustc -> LLVM bitcode -> bc2sa`。主线固定为：

```text
Rust 源码
  -> rustc 官方前中端
  -> mono Instance + codegen MIR
  -> canonical SCI plan
  -> SA text / direct SAB
  -> SCI Referee
  -> relocatable object
  -> rustc archive/link pipeline
```

当前仓库已经完成最小可运行 PoC：rustc-private backend dylib、CGU/mono item 遍历、标量 MIR lowering、worker RPC、SA 文本 emission、SCI `build-obj`、对象回传 rustc、C harness 链接运行。下一步计划从“能跑标量 add/branch/call”推进到“可维护的 Rust codegen backend”。

## 结论

- 可以实现 `rustc -> SA/SCI` 的受限 codegen backend，且应继续用独立仓库 `/root/projects/rustc_codegen_sci`，不要 fork 整个 rustc，也不要走 `bc2sa`。
- 第一可交付目标是 `rust-trusted`：信任 rustc 的类型检查、borrowck、单态化和布局，SCI 负责验证结构、CFG、类型、ABI、资源/capability 计划和对象生成门禁。
- `sa-verified-subset` 不是普通后端功能，必须先解决 borrowck sidecar 到 optimized/codegen MIR 的 refinement certificate；未通过前不能宣称 SCI 重新证明了完整 Rust 所有权。
- 完整 Rust/std/unwind/SIMD/多 target 是长期工程，不是当前 PoC 的自然外推。
- 编译速度首期不要承诺快于 rustc LLVM。短期因为多了 plan、IPC、Referee，clean build 预计更慢；只有 direct SAB、缓存、并行 worker、非 LLVM emitter 成熟后，debug 子集才有机会接近或超过 LLVM。

## 近端冲刺

目标：把当前标量 PoC 扩成可覆盖常见 `no_std` 控制流与算术的稳定 bring-up slice。

交付项：

1. MIR `Assert` lowering，`panic=abort` 下生成确定 panic/abort 路径，不允许 unwind edge。
2. 整数 `Div`/`Rem`、shift、unary op、更多 `SwitchInt`，包括 signed/unsigned 显式区分。
3. `CheckedBinaryOp` 的 tuple/projection 支持，保留 overflow assert 语义。
4. worker 校验 synthetic panic block、CFG target、callee signature、local type/dataflow。
5. smoke fixture 扩到 div/rem/shift/assert/checked op，并保留生成 SA golden。
6. 更新 `docs/status.md` 能力矩阵，所有未支持项继续 hard error。

退出条件：

- `./scripts/test.sh` 通过。
- 生成对象可链接运行。
- 未支持 MIR/ABI 不进入 SCI object publication。
- 无 LLVM backend fallback，无 `bc2sa` fallback。

## M0：协议与门禁冻结

目标：把现有 `SciModulePlan` 从标量 PoC 升级成后续 ABI、layout、WASM、SAB、proof 都能复用的稳定契约。

交付项：

- target descriptor：triple、object format、endianness、pointer width、DataLayout、CPU/features、relocation/code model。
- `TypeLayoutRecipe`：size/align、scalar valid range、field offset/order、variant/niche/tag encoding。
- `FnAbiPlan`：Direct、Pair、Cast、Indirect、sret/byval/on-stack、extension、calling convention、variadic、can_unwind。
- `AllocationImage` 和 relocation graph：bytes、uninit ranges、static/function/vtable relocation、section、TLS、COMDAT、used/compiler_used。
- `MemoryOpPlan`：load/store/copy size、align、volatile、atomic ordering、padding/provenance policy。
- structured diagnostics：stable code、rust span、mono instance、MIR location、SCI cause chain。
- backend capability hooks：target、crate type、panic、LTO、coverage、sanitizer、float reliability、intrinsic fallback 都显式声明或拒绝。

退出条件：

- worker 不再从 host default 推断 target。
- ABI fixture 能做 LLVM caller <-> SCI callee 双向验证。
- unsupported feature 都是前端或 plan 阶段结构化错误。
- 协议版本变化会失效旧 artifact。

## M1：可审计 backend bring-up

目标：完成 `no_std`、`panic=abort`、x86_64 Linux、标量/受限指针/基本 CFG 的可信 MVP。

交付项：

- 更完整的 basic block、multi-way switch、assert、direct call、return。
- 标量 Direct ABI 闭合，有限 Pair/Indirect spike。
- SA text emitter 和 direct SAB emitter 使用同一 canonical plan。
- SCI Referee 必须在 artifact 写入前通过。
- rlib/object 输出路径与 rustc archive/link pipeline 保持兼容。
- proof/work-product manifest 初版：plan hash、object hash、target、policy、engine/schema hash。

退出条件：

- clean build、dirty build、no-op build 有稳定结果。
- direct SAB `--no-fallback` 与 SA text 路径行为一致。
- object、plan、proof report 不一致时拒绝复用。
- smoke + golden + compile-fail 覆盖所有声明能力。

## M2：no_std + ownership-rich lowering

目标：从“标量函数能跑”升级到常见 `no_std` Rust 子集。

交付项：

- aggregate、tuple、struct、enum、niche、discriminant、field projection。
- stack/global allocation、static relocation、字符串和 panic metadata。
- drop glue 调用、partial move、init-state dataflow、CFG capability join。
- reference/raw pointer 边界、fat pointer、slice、受限 trait object/vtable。
- intrinsic registry：逐项支持或 hard error，不按符号名静默当 extern。
- proof coverage report：tracked/untracked/native/ffi/raw/unknown 计数。

退出条件：

- LLVM/Miri 差分 corpus 通过。
- ABI mismatch 为零。
- drop/borrow/partial-move negative corpus 稳定拒绝。
- `rust-trusted` 报告清楚列出 trusted boundary，不宣称 strict proof。

## M2S：sa-verified-subset 研究门禁

目标：决定严格 Rust 所有权证明是否能进入工程排期。

交付项：

- borrowck sidecar capture：BorrowSet、region facts、loan 起止、MoveData/init dataflow。
- pre-opt MIR -> codegen MIR、generic -> mono、inline source -> caller、local crate -> metadata 的映射方案。
- MIR pass refinement certificate spike，覆盖 drop elaboration、inlining、SROA/GVN 等实际启用 pass，或明确禁用未证明 pass。
- `ProofFnContract`：参数/返回 provenance、borrow/consume/escape/drop/alloc effect、contract hash。

退出条件：

- sidecar 或 certificate 缺失时 hard fail，不自动降级为“已验证”。
- strict corpus 的 direct/indirect call contract 闭合。
- proof coverage 达到冻结阈值。
- 若没有 object/linked-image validator，只能声明 SCI IR proof + TCB，不能声明机器码端到端 proof。

## M3：Cargo 生态接入

目标：让真实 Cargo workspace 可以使用 SCI backend 编译 target crate，同时 host unit 继续使用 LLVM。

交付项：

- `sa rust init/check/build/emit-sa` 生成和驱动 nightly Cargo `codegen-backend` 配置。
- build script、proc macro、host dependency 强制走 LLVM backend。
- 同一 rustc commit 的 LLVM-built sysroot 互操作。
- rlib metadata、native library propagation、final linker diagnostics 映射。
- `sa rust test` 只在 `-Z panic-abort-tests`、test-per-process、rustdoc/doctest 门禁完成后开放。

退出条件：

- host/target artifact 分离可复现。
- proc macro/build.rs 不计入 target proof 覆盖。
- 常见 `no_std` workspace clean/dirty/no-op 构建通过。

## M4：SCI sysroot 与产品化缓存

目标：减少 LLVM-built sysroot trusted boundary，形成可发布工具链。

交付项：

- SCI-built `core`，随后 `alloc`，最后受限 `std panic=abort`。
- allocator shim、compiler-builtins、panic runtime/handler。
- proof-aware incremental reuse：pre-reuse manifest/hash validation、原子写入、损坏隔离、LRU/预算、并发锁。
- 发布包绑定 rustc commit、adapter hash、SCI engine ABI、SA ISA、SAB schema、sysroot identity。

退出条件：

- cache policy/engine/schema/adapter/sysroot 变化可靠失效旧 CGU。
- proof report 能随 rlib/最终链接聚合。
- ABI/metadata 升级策略明确。

## M5：WASM 平台

WASM 不应在 M0 前做，因为当前 SCI native emitter 不能再依赖 host target。WASM 路线应分两层：

1. `wasm32-unknown-unknown`：无 OS、无 WASI、`panic=abort`、无 unwind、无 threads/TLS，先支持 relocatable wasm object 或直接 wasm module emission。
2. `wasm32-wasip1`：在前者稳定后加入 WASI import、link model、start/export 策略、有限 libc/sysroot 边界。

必须补齐：

- target descriptor 的 wasm object format、pointer width、DataLayout、feature set。
- WASM relocation、function/table/global/memory import-export、section layout。
- `extern "C"`/Rust ABI 在 wasm32 上的 pass mode fixture。
- `panic=abort` 和 `core`/`alloc` 在 wasm32 的 runtime 策略。
- worker 明确 target，不允许 host x86_64 object emitter 混入。

退出条件：

- LLVM wasm caller <-> SCI wasm callee 双向 ABI fixture 通过。
- wasm runtime 差分执行通过。
- `wasm32-unknown-unknown` 先于 WASI/threads/unwind。

## M6：困难特性逐项开放

以下特性必须单独立项，不合并成“完整 Rust”一次性交付：

- panic unwind、EH personality、cleanup edge。
- TLS、threads、atomics、WASI threads。
- i128、SIMD、target_feature、inline/global/naked asm。
- debug info、coverage、sanitizer、LTO。
- dynamic library、cdylib、proc macro target、自定义 linker script。
- unsafe/raw/UnsafeCell 的严格证明。
- translation certificate、object validator、linked-image validator。

每项都需要 capability flag、早期诊断、ABI/runtime test、negative test、性能门槛和文档声明。

## 性能计划

短期预期：

- clean build：SCI backend 预计慢于 rustc LLVM，主要来自 MIR lowering、IPC、Referee 和当前仍经 SCI native emitter 的对象生成。合理预估是 LLVM 的 1.2x-3x wall time，必须用本机基准确认。
- dirty/no-op build：如果 proof-aware cache 做好，可能接近 rustc 增量表现；如果每次重跑 Referee/worker，则仍会慢。
- runtime：取决于 SCI emitter 生成机器码质量，不能默认等于 LLVM `-O`。

中期优化：

- persistent worker，批量 CGU request，减少进程启动和 schema 解析。
- direct SAB，避免 SA 文本 parse 成本。
- content-addressed plan/object cache。
- 并行 worker 调度，分离 Referee 与 object emission profiling。

长期如果目标是 debug 编译速度快于 LLVM，需要 SCI 直接 object emitter 或 Cranelift-like 快速后端；“LLVM 前面加一层验证”通常不会自然更快。

## 立即执行顺序

1. 已完成 `Assert`、div/rem/shift/unary、multi-way switch、checked add/sub/mul 覆盖到 64 位整数和 smoke 扩展。
2. 已升级到 `PLAN_VERSION = 6`，包含 target-qualified header、模块级 extern function plan、void return/call plan 和 raw pointer direct ABI 类型。
3. 当前增量已完成直接标量/raw pointer/void `extern "C"` 调用、null pointer、pointer `Eq`/`Ne` 和 thin `PtrToPtr` copy；下一步做 20-30 个 ABI fixture，优先 Direct/Pair/Cast/Indirect/sret/byval。
4. 已完成函数内部 local scalar tuple/struct 的构造、field projection、本地 copy/move，以及空 struct ZST local no-op；aggregate/ZST struct 参数/返回 ABI 仍保持 hard error。
5. 做 static allocation/relocation 最小闭环，支持字符串与 panic metadata。
6. 引入 direct SAB no-fallback 路径，与 SA text parity。
7. 做 proof-aware manifest/cache，不让 stock rustc work-product 路径独自决定复用。
8. 再进入 aggregate/drop/borrow/fat pointer。
9. 最后推进 Cargo、sysroot、WASM 和 strict proof。

## SLA/SAB 参考实现借鉴

`/root/projects/sa_plugins/sa_plugin_sla/sap.json` 只是插件清单：它声明 `sa sla ...` 命令、动态库路径、权限和 help 文本，不是编译器主逻辑。真正可借鉴的是 `sa_plugin_sla` 的 SAB 管线约束：

- 用户面同时保留 `.sa` 文本和 `.sab` 二进制，但内部要收敛到共享 lowering plan，再分叉到两个 emitter。
- direct SAB 主线不能实现成 `source -> .sa text -> SAB`；文本只能用于调试、审计和兼容路径。
- 托管 SAB artifact 使用稳定 `.sla-cache/sab/...` 路径，用户可见 `--out` 只是额外副本；这对应 `rustc_codegen_sci` 后续 CGU work-product/cache 绑定。
- `SLA_SAB_NO_FALLBACK=1` 这类 no-fallback gate 很重要；`rustc_codegen_sci` 的 direct SAB 模式也必须失败即报错，不能回退到 SA 文本或 LLVM。
- SAB v4 不保存完整 raw `.sa` 文本，只保存结构化指令、操作数、函数寄存器、package/upstream metadata；Rust 侧 direct SAB 也应消费 canonical plan，而不是复用文本 emitter 输出。
