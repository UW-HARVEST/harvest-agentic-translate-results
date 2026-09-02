#!/usr/bin/env bash
# Phase D: run the whole verification under EVERY feature combination.
# Feature names are extracted from Cargo.toml (never hard-coded), then the
# power-set is enumerated and each element is built + tested + symbol-checked.
set -uo pipefail
cd "$(dirname "$0")/.."

# Extract feature names from the [features] section, ignoring "default".
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

FEAT_ARR=()
while IFS= read -r f; do [ -n "$f" ] && FEAT_ARR+=("$f"); done <<< "$FEATURES"
N=${#FEAT_ARR[@]}
echo "features declared in Cargo.toml: $N ${FEAT_ARR[*]:-(none)}"

# Build the list of configurations to test.
CONFIGS=()
CONFIGS+=("default:")                              # default features
CONFIGS+=("no-default:--no-default-features")      # nothing enabled
if [ "$N" -gt 0 ]; then
  for ((mask=1; mask<(1<<N); mask++)); do
    combo=""
    for ((i=0; i<N; i++)); do
      (( mask & (1<<i) )) && combo="${combo:+$combo,}${FEAT_ARR[i]}"
    done
    CONFIGS+=("$combo:--no-default-features --features $combo")
    CONFIGS+=("default+$combo:--features $combo")
  done
fi

echo "configurations to verify: ${#CONFIGS[@]}"
echo

fail=0
for entry in "${CONFIGS[@]}"; do
  label=${entry%%:*}
  flags=${entry#*:}
  printf '=== [%s] cargo %s ===\n' "$label" "${flags:-<default>}"

  if ! timeout 300 cargo check $flags >/dev/null 2>&1; then
    echo "  FAIL cargo check"; fail=$((fail+1)); continue
  fi
  if ! timeout 300 cargo build --release $flags >/dev/null 2>&1; then
    echo "  FAIL cargo build --release"; fail=$((fail+1)); continue
  fi
  if ! ./scripts/symbol_parity.sh >/dev/null 2>&1; then
    echo "  FAIL symbol parity"; ./scripts/symbol_parity.sh | tail -20; fail=$((fail+1)); continue
  fi
  out=$(timeout 300 cargo test --release $flags --test differential -- --test-threads=1 2>&1)
  if [ $? -ne 0 ]; then
    echo "  FAIL differential tests"; echo "$out" | tail -30; fail=$((fail+1)); continue
  fi
  echo "  OK  $(echo "$out" | grep -E '^test result' | tail -1)"
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL ${#CONFIGS[@]} FEATURE CONFIGURATIONS PASS"
else
  echo "$fail configuration(s) FAILED"
fi
exit "$fail"
