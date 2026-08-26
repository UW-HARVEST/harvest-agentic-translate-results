#!/usr/bin/env bash
# Enumerate EVERY valid feature combination from Cargo.toml and run
# `cargo check` + the full differential test suite for each.
#
# Usage:  ./check_all_features.sh [check|test]
#   check  (default) -- cargo check for every combination
#   test             -- cargo test  for every combination
set -uo pipefail

MODE="${1:-check}"
cd "$(dirname "$0")"

# ---------------------------------------------------------------------------
# 1. Extract the feature names from the [features] section of Cargo.toml.
#    Ignore the implicit "default" key itself (it is expressed via
#    --no-default-features / plain build) and ignore optional-dependency
#    features of the form "dep:...".
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inblk = 1; next }
    /^\[/           { inblk = 0 }
    inblk && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

HAS_DEFAULT=$(awk '/^\[features\]/{i=1;next} /^\[/{i=0} i && /^default[[:space:]]*=/{print "yes"}' Cargo.toml)

echo "=============================================================="
echo "Cargo.toml [features] section"
echo "=============================================================="
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "  (none declared)"
else
  printf '  %s\n' "${FEATURES[@]}"
fi
echo "  default feature set declared: ${HAS_DEFAULT:-no}"
echo

# ---------------------------------------------------------------------------
# 2. Build the list of combinations = the full power set of FEATURES.
#    With zero features declared there is exactly ONE valid configuration.
# ---------------------------------------------------------------------------
COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBOS=("")            # the single, empty combination
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=============================================================="
echo "Enumerated ${#COMBOS[@]} feature combination(s)"
echo "=============================================================="
for c in "${COMBOS[@]}"; do
  echo "  --no-default-features --features '${c}'"
done
echo

# ---------------------------------------------------------------------------
# 3. Run the requested cargo command for every combination.
# ---------------------------------------------------------------------------
FAIL=0
for c in "${COMBOS[@]}"; do
  label="${c:-<no features>}"
  echo "--------------------------------------------------------------"
  echo ">>> $MODE : $label"
  echo "--------------------------------------------------------------"
  if [ "$MODE" = "test" ]; then
    timeout 900 cargo test --release --offline --no-default-features --features "$c" 2>&1 \
      | grep -E 'test result|^error|^test [a-z_]+ \.\.\. FAILED|panicked at|Compiling|Finished' \
      || true
    rc=${PIPESTATUS[0]}
  else
    timeout 600 cargo check --release --offline --all-targets \
      --no-default-features --features "$c" 2>&1 | tail -5
    rc=${PIPESTATUS[0]}
  fi
  if [ "$rc" -ne 0 ]; then
    echo "!!! FAILED (exit $rc) for combination: $label"
    FAIL=1
  else
    echo "OK: $label"
  fi
  echo
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "ALL ${#COMBOS[@]} FEATURE COMBINATION(S) PASSED ($MODE)"
else
  echo "SOME FEATURE COMBINATIONS FAILED ($MODE)"
fi
echo "=============================================================="
exit $FAIL
