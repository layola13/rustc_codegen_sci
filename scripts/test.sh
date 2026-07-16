#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
source "$ROOT/scripts/env.sh"

"$CARGO" test -p sci_protocol
"$CARGO" test -p sci_codegen_worker
"$CARGO" build --workspace
"$ROOT/tests/smoke.sh"
