#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_ROOT="${RUST_ROOT:-/root/projects/rust}"

export RUSTC="${RUSTC:-$RUST_ROOT/build/x86_64-unknown-linux-gnu/stage2/bin/rustc}"
export CARGO="${CARGO:-$RUST_ROOT/build/x86_64-unknown-linux-gnu/stage2-tools-bin/cargo}"
export SCI_RUST_SYSROOT="${SCI_RUST_SYSROOT:-$RUST_ROOT/build/x86_64-unknown-linux-gnu/stage1}"
export SCI_BIN="${SCI_BIN:-/root/projects/sci/zig-out/bin/sa}"

HOST_LIB="$RUST_ROOT/build/x86_64-unknown-linux-gnu/stage2/lib"
TARGET_LIB="$SCI_RUST_SYSROOT/lib/rustlib/x86_64-unknown-linux-gnu/lib"
export LD_LIBRARY_PATH="$HOST_LIB:$TARGET_LIB${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export RUSTC_BOOTSTRAP=1
export RUSTFLAGS="--sysroot=$SCI_RUST_SYSROOT -Cprefer-dynamic ${RUSTFLAGS:-}"
export RUSTDOCFLAGS="--sysroot=$SCI_RUST_SYSROOT -Cprefer-dynamic ${RUSTDOCFLAGS:-}"
export SCI_WORKSPACE_ROOT="$ROOT"

