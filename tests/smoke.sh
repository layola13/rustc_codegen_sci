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
"$RUSTC" \
    --sysroot "$SCI_RUST_SYSROOT" \
    -Zcodegen-backend="$BACKEND" \
    --crate-type=lib \
    --edition=2024 \
    -Cpanic=abort \
    -Coverflow-checks=on \
    -Ccodegen-units=1 \
    --emit=obj="$OUT/add.o" \
    "$ROOT/tests/fixtures/add.rs"

cc -no-pie "$ROOT/tests/fixtures/add_harness.c" "$OUT/add.o" -o "$OUT/add-smoke"
"$OUT/add-smoke"
