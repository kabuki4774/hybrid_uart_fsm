#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

: "${RUST_FEATURES:=std,crc16,crc16_2b,byte_stuff}"

mkdir -p fuzz/corpus/fuzz_parser fuzz/corpus/fuzz_diff

mk() {
  local out="$1"; shift
  cargo run -p uart_fsm_rs --example make_frame --quiet \
    --no-default-features --features "$RUST_FEATURES" -- "$@" > "$out"
  echo "Seed: $out"
}

mk fuzz/corpus/fuzz_parser/start.bin --type START
mk fuzz/corpus/fuzz_parser/stop.bin  --type STOP
mk fuzz/corpus/fuzz_parser/ping.bin  --type PING --ascii hi
mk fuzz/corpus/fuzz_parser/reset.bin --type RESET

# Also useful for diff target
cp -f fuzz/corpus/fuzz_parser/* fuzz/corpus/fuzz_diff/ 2>/dev/null || true