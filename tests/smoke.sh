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

compile_fail_fixture() {
    local name="$1"
    shift
    local stderr="$OUT/$name.stderr"

    if "$RUSTC" \
        --sysroot "$SCI_RUST_SYSROOT" \
        -Zcodegen-backend="$BACKEND" \
        --crate-type=lib \
        --edition=2024 \
        -Cpanic=abort \
        -Coverflow-checks=on \
        -Ccodegen-units=1 \
        --emit=obj="$OUT/$name.o" \
        "$ROOT/tests/fixtures/$name.rs" \
        2>"$stderr"
    then
        echo "expected fixture $name to fail, but it compiled" >&2
        return 1
    fi

    local expected
    for expected in "$@"; do
        if ! grep -Fq "$expected" "$stderr"; then
            echo "expected fixture $name stderr to contain: $expected" >&2
            cat "$stderr" >&2
            return 1
        fi
    done
}

compile_fixture add
compile_fixture abi_direct
compile_fixture abi_cast
compile_fail_fixture abi_pair \
    "rustc_codegen_sci backend rejected module [SCI_BACKEND_ABI_UNSUPPORTED] at function \`sci_abi_pair_return\`: sci_abi_pair_return: ABI return uses unsupported Pair pass mode" \
    "tests/fixtures/abi_pair.rs:4:"
compile_fail_fixture abi_indirect \
    "rustc_codegen_sci backend rejected module [SCI_BACKEND_ABI_UNSUPPORTED] at function \`sci_abi_indirect_return\`: sci_abi_indirect_return: ABI return uses unsupported Indirect pass mode" \
    "tests/fixtures/abi_indirect.rs:11:"
compile_fail_fixture unsupported_ref \
    "rustc_codegen_sci backend rejected module [SCI_BACKEND_MIR_UNSUPPORTED] at function \`sci_unsupported_ref_i32\`, block 0: sci_unsupported_ref_i32: block 0 statement 0:" \
    "tests/fixtures/unsupported_ref.rs:5:"
