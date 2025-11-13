#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! grep -q 'cargo-fuzz = true' fuzz/Cargo.toml 2>/dev/null; then
  printf '\n[package.metadata]\ncargo-fuzz = true\n' >> fuzz/Cargo.toml
fi

mkdir -p logs reports fuzz/dict
: "${FUZZ_SECONDS:=60}"
: "${FUZZ_TARGETS:=fuzz_parser fuzz_diff}"
# Note: FUZZ_FEATURES is informational only; actual features for rust_logic are set in fuzz/Cargo.toml
: "${FUZZ_FEATURES:=std,crc16,crc16_2b,byte_stuff}"

FUZZ_REPORT="reports/FUZZ_REPORT.md"
echo "# Fuzz Report" > "$FUZZ_REPORT"
echo "_Generated: $(date)_" >> "$FUZZ_REPORT"
echo >> "$FUZZ_REPORT"

have() { command -v "$1" >/dev/null 2>&1; }

echo "==> Checking cargo-fuzz..."
if ! have cargo-fuzz; then
  echo "Installing cargo-fuzz..." | tee -a "$FUZZ_REPORT"
  cargo install cargo-fuzz >/dev/null
fi
echo "- cargo-fuzz $(cargo fuzz --version)" >> "$FUZZ_REPORT"

# Optional dictionary accelerates discovery
if [ ! -f fuzz/dict/uart.dict ]; then
  cat > fuzz/dict/uart.dict <<'DICT'
"\xAA"
"\x7D"
"\x01"
"\x02"
"\x03"
"\xFF"
"hi"
DICT
fi

echo >> "$FUZZ_REPORT"
echo "## Targets & settings" >> "$FUZZ_REPORT"
echo "- FUZZ_SECONDS: $FUZZ_SECONDS" >> "$FUZZ_REPORT"
echo "- FUZZ_FEATURES: $FUZZ_FEATURES (via fuzz/Cargo.toml for rust_logic)" >> "$FUZZ_REPORT"
echo "- FUZZ_TARGETS: $FUZZ_TARGETS" >> "$FUZZ_REPORT"

for t in $FUZZ_TARGETS; do
  CORPUS="fuzz/corpus/$t"
  ARTS="fuzz/artifacts/$t"
  LOG="logs/fuzz_${t}.log"
  mkdir -p "$CORPUS" "$ARTS"

  BEFORE=$(find "$CORPUS" -type f 2>/dev/null | wc -l | tr -d ' ')
  echo
  echo "==> Fuzzing $t for ${FUZZ_SECONDS}s"
  set +e
  cargo fuzz run "$t" \
    --no-default-features \
    -- -dict=fuzz/dict/uart.dict -max_total_time="$FUZZ_SECONDS" -print_final_stats=1 \
    >"$LOG" 2>&1
  RC=$?
  set -e

  AFTER=$(find "$CORPUS" -type f 2>/dev/null | wc -l | tr -d ' ')
  CRASHES=$(find "$ARTS" -type f 2>/dev/null | wc -l | tr -d ' ')

  echo >> "$FUZZ_REPORT"
  echo "### $t" >> "$FUZZ_REPORT"
  if [ "$RC" -eq 0 ]; then
    echo "- ✅ Completed without crashes" >> "$FUZZ_REPORT"
  else
    echo "- ❌ Non-zero exit (possible crash/timeout/OOM)" >> "$FUZZ_REPORT"
  fi
  echo "- Corpus: $BEFORE → $AFTER files" >> "$FUZZ_REPORT"
  echo "- Artifacts: $CRASHES files in \`$ARTS\`" >> "$FUZZ_REPORT"

  echo -e "\n<details><summary>Final libFuzzer stats (tail)</summary>\n\n\`\`\`" >> "$FUZZ_REPORT"
  tail -n 40 "$LOG" >> "$FUZZ_REPORT"
  echo -e "\`\`\`\n</details>" >> "$FUZZ_REPORT"
done

echo
echo "Fuzzing complete. See $FUZZ_REPORT"