#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/env.sh"

BACKEND="$ROOT/target/debug/librustc_codegen_sci.so"
WORKER="$ROOT/target/debug/sci-codegen-worker"
OUT="$ROOT/artifacts/smoke"

test -x "$WORKER"
test -f "$BACKEND"
rm -rf "$OUT"
mkdir -p "$OUT"

export SCI_CODEGEN_WORKER="$WORKER"

compile_fixture() {
    local name="$1"

    "$RUSTC" \
        --sysroot "$SCI_RUST_SYSROOT" \
        -Zcodegen-backend="$BACKEND" \
        --crate-type=lib \
        --edition=2024 \
        -Cpanic=abort \
        -Coverflow-checks=on \
        -Ccodegen-units=1 \
        --emit=obj="$OUT/$name.o" \
        "$ROOT/tests/fixtures/$name.rs"

    cc -no-pie "$ROOT/tests/fixtures/${name}_harness.c" "$OUT/$name.o" -o "$OUT/$name-smoke"
    "$OUT/$name-smoke"
}

compile_fixture add
compile_fixture abi_direct
