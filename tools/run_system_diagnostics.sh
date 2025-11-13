#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

mkdir -p reports logs
REPORT="reports/SYSTEM_STATUS_REPORT.md"
echo "# System Verification & Status Report" > "$REPORT"
echo "_Generated: $(date)_" >> "$REPORT"
echo >> "$REPORT"

# Tunables (override via env)
: "${RUST_FEATURES:=std,crc16,crc16_2b,byte_stuff}"
: "${RUST_PROFILE:=debug}"   # change to 'release' if preferred
: "${FUZZ_SECONDS:=30}"
: "${FUZZ_TARGETS:=fuzz_parser fuzz_diff}"

section(){ echo -e "\n## $1\n" >> "$REPORT"; }
sub(){ echo -e "\n### $1\n" >> "$REPORT"; }
have(){ command -v "$1" >/dev/null 2>&1; }

# ---------------------------------------------------------------------------
section "1. Environment"
{
  echo "- OS: $(uname -a)"
  echo "- Rust: $(rustc --version 2>/dev/null || echo missing)"
  echo "- Cargo: $(cargo --version 2>/dev/null || echo missing)"
  echo "- cargo-fuzz: $(cargo fuzz --version 2>/dev/null || echo 'not installed')"
  echo "- clang: $(clang --version | head -n1)"
  echo "- CPU: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || lscpu | grep 'Model name')"
  echo "- Memory: $(sysctl -n hw.memsize 2>/dev/null || free -h | grep Mem)"
  echo "- Git Commit: $(git rev-parse --short HEAD 2>/dev/null || echo n/a) $(git diff --quiet || echo '+dirty')"
  echo "- RUST_FEATURES: $RUST_FEATURES"
  echo "- RUST_PROFILE: $RUST_PROFILE"
  echo
} >> "$REPORT"

# ---------------------------------------------------------------------------
section "2. Build & Integration"

sub "C build"
if make -C uart_fsm_c clean && make -C uart_fsm_c &>logs/c_build.txt; then
  echo "- ✅ C build succeeded" >> "$REPORT"
else
  echo "- ❌ C build failed" >> "$REPORT"
  tail -n 25 logs/c_build.txt >> "$REPORT"
fi

sub "Rust (FFI) build"
PROFILE_FLAG=
if [ "$RUST_PROFILE" = "release" ]; then PROFILE_FLAG=--release; fi
if cargo build --manifest-path rust_logic/Cargo.toml --no-default-features --features "$RUST_FEATURES" $PROFILE_FLAG &>logs/rust_build.txt; then
  echo "- ✅ Rust build succeeded (features: $RUST_FEATURES, profile: $RUST_PROFILE)" >> "$REPORT"
else
  echo "- ❌ Rust build failed" >> "$REPORT"
  tail -n 25 logs/rust_build.txt >> "$REPORT"
fi

sub "Hybrid link smoke"
if make -C c_firmware clean all RUST_PROFILE="$RUST_PROFILE" RUST_FEATURES="$RUST_FEATURES" &>logs/hybrid_build.txt \
   && ./c_firmware/main &>logs/hybrid_run.txt; then
  echo "- ✅ Hybrid Rust→C worker ran" >> "$REPORT"
else
  echo "- ⚠️ Hybrid run error" >> "$REPORT"
  tail -n 25 logs/hybrid_build.txt >> "$REPORT"
  tail -n 25 logs/hybrid_run.txt >> "$REPORT"
fi

# ---------------------------------------------------------------------------
section "3. Functional Verification"

DEMO_LOG="logs/c_demo1.txt"
./uart_fsm_c/uart_fsm_demo --test >"$DEMO_LOG" 2>&1 || true
LC_ALL=C sed -i '' 's/→/->/g' "$DEMO_LOG" 2>/dev/null || sed -i 's/→/->/g' "$DEMO_LOG"

echo "- Checking expected output patterns" >> "$REPORT"

declare -a PATTERN_DESC=("STATE: Idle -> Active" "PONG" "HEARTBEAT" "Active -> Idle" "Error -> Idle")
declare -a PATTERN_REGEX=("STATE.*Idle.*Active" "PONG" "HEARTBEAT" "Active.*Idle" "Error.*Idle")

for i in "${!PATTERN_DESC[@]}"; do
  desc="${PATTERN_DESC[$i]}"; regex="${PATTERN_REGEX[$i]}"
  if grep -Eq "$regex" "$DEMO_LOG"; then echo "- ✅ Found $desc" >> "$REPORT"; else echo "- ❌ Missing $desc" >> "$REPORT"; fi
done

echo -e "\n> Demo Output (first 10 lines):\n\`\`\`" >> "$REPORT"
head -10 "$DEMO_LOG" >> "$REPORT"
echo "\`\`\`" >> "$REPORT"

# ---------------------------------------------------------------------------
section "4. Protocol Features & Parity"
grep -q "fsm_next_deadline" uart_fsm_c/src/fsm.c && echo "- ✅ Tickless function compiled in" >> "$REPORT" || echo "- ⚠️ Tickless disabled" >> "$REPORT"
grep -q "crc16_ccitt"       uart_fsm_c/src/parser.c && echo "- ✅ CRC16 present"                 >> "$REPORT" || echo "- ⚠️ CRC16 disabled"      >> "$REPORT"
grep -q "USE_BYTESTUFF"     uart_fsm_c/src/parser.c && echo "- ✅ Byte-Stuffing feature available" >> "$REPORT" || echo "- ⚠️ Byte-Stuffing disabled" >> "$REPORT"

sub "Resolved Rust features (cargo tree)"
{
  echo "\`\`\`"
  cargo tree -e features -p rust_logic || true
  echo
  cargo tree -e features -p uart_fsm_rs || true
  echo "\`\`\`"
} >> "$REPORT"

# ---------------------------------------------------------------------------
section "5. Rust Worker Thread"
if ./c_firmware/main >logs/worker.txt 2>&1; then
  if grep -q "HEARTBEAT" logs/worker.txt; then
    echo "- ✅ Rust worker heartbeat observed" >> "$REPORT"
  else
    echo "- ⚠️ Rust worker ran without heartbeat" >> "$REPORT"
  fi
else
  echo "- ❌ Rust worker execution failed" >> "$REPORT"
fi

# ---------------------------------------------------------------------------
section "6. Performance Metrics"

BIN="uart_fsm_c/uart_fsm_demo"
if [ -f "$BIN" ]; then
  SIZE=$(stat -f%z "$BIN" 2>/dev/null || stat -c%s "$BIN")
  echo "- Binary size: ${SIZE} bytes" >> "$REPORT"
  echo "- SHA256: $(shasum -a 256 "$BIN" | cut -d' ' -f1)" >> "$REPORT"
fi

echo -e "\n### Timing (100 frames)\n" >> "$REPORT"
{ /usr/bin/time -lp ./uart_fsm_c/uart_fsm_demo --test >/dev/null; } 2>>"$REPORT"

{
  echo -e "\n### CPU Snapshot"
  ps -Ao %cpu,%mem,command | grep -E "uart_fsm|cargo" | head -5
} >> "$REPORT" || true

# ---------------------------------------------------------------------------
section "7. Documentation Completeness"
MISSING=$(find docs -type f -size 0 -print)
if [ -z "$MISSING" ]; then
  echo "- ✅ All documentation files populated" >> "$REPORT"
else
  echo "- ⚠️ Empty docs files:" >> "$REPORT"
  echo "$MISSING" >> "$REPORT"
fi

# ---------------------------------------------------------------------------
section "8. Fuzzing (libFuzzer via cargo-fuzz)"
if have cargo-fuzz; then
  echo "- Running fuzzers for ${FUZZ_SECONDS}s each..." >> "$REPORT"
  FUZZ_LOG="logs/fuzz_driver.log"
  set +e
  FUZZ_SECONDS="$FUZZ_SECONDS" FUZZ_TARGETS="$FUZZ_TARGETS" ./tools/run_fuzz.sh &>"$FUZZ_LOG"
  RC=$?
  set -e
  if [ "$RC" -eq 0 ]; then echo "- ✅ Fuzz driver completed (see reports/FUZZ_REPORT.md)" >> "$REPORT"
  else echo "- ⚠️ Fuzz driver returned non-zero (possible crashes found)" >> "$REPORT"; fi
  echo -e "\n<details><summary>Fuzz summary (excerpt)</summary>\n" >> "$REPORT"
  sed -n '1,200p' reports/FUZZ_REPORT.md >> "$REPORT" || true
  echo -e "\n</details>" >> "$REPORT"
else
  echo "- ⚠️ cargo-fuzz not installed; skipping fuzzing" >> "$REPORT"
fi

# ---------------------------------------------------------------------------
section "9. Differential Scan (Rust vs C parser over corpora)"
set +e
cargo run --manifest-path rust_logic/Cargo.toml --quiet \
  --no-default-features --features "$RUST_FEATURES" $PROFILE_FLAG \
  --bin diff_scan -- fuzz/corpus fuzz/artifacts > logs/diff_scan.txt 2>&1
RC_DIFF=$?
set -e

if [ "$RC_DIFF" -eq 0 ]; then
  echo "- ✅ Differential scan completed" >> "$REPORT"
else
  echo "- ⚠️ Differential scan reported mismatches" >> "$REPORT"
fi

echo -e "\n<details><summary>Diff scan summary</summary>\n\n\`\`\`" >> "$REPORT"
tail -n 200 logs/diff_scan.txt >> "$REPORT"
echo -e "\`\`\`\n</details>" >> "$REPORT"

# ---------------------------------------------------------------------------
section "10. Summary Table"
echo "| Check | Status |" >> "$REPORT"
echo "|-------|---------|" >> "$REPORT"
grep -E "✅|⚠️|❌" "$REPORT" | sed -E 's/^- //' | while read -r line; do
  icon=$(echo "$line" | grep -o "✅\|⚠️\|❌")
  desc=$(echo "$line" | sed -E 's/[✅⚠️❌]//g')
  echo "| $desc | $icon |" >> "$REPORT"
done

echo -e "\nReport written to $REPORT"