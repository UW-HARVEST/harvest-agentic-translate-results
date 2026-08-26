#!/usr/bin/env bash
# Enumerate EVERY valid Cargo feature combination and run `cargo check` plus the
# full differential test suite (Phases B, C, D) for each one.
#
# Feature names are extracted from Cargo.toml's [features] table; the powerset is
# enumerated. This crate currently has no [features] table, so the powerset is
# the single empty combination — the loop is written generically so it stays
# correct if features are added later.
#
# Usage: ./check_all_features.sh [--release]
set -uo pipefail
cd "$(dirname "$0")"

EXTRA=()
[ "${1:-}" = "--release" ] && EXTRA+=(--release)

# ---------------------------------------------------------------------------
# 1. Extract feature names from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
if not m:
    raise SystemExit
for line in m.group(1).splitlines():
    line = line.split('#', 1)[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name and name != 'default':
        print(name)
PY
)

NF=${#FEATURES[@]}
echo "=== features found: $NF ${FEATURES[*]:-(none)} ==="

# ---------------------------------------------------------------------------
# 2. Build the powerset of feature combinations
# ---------------------------------------------------------------------------
COMBOS=()
for ((mask = 0; mask < (1 << NF); mask++)); do
  combo=""
  for ((i = 0; i < NF; i++)); do
    if (((mask >> i) & 1)); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done

echo "=== ${#COMBOS[@]} combination(s) to verify ==="

# ---------------------------------------------------------------------------
# 3. Make sure the C reference .so exists
# ---------------------------------------------------------------------------
if ! ls c_src/build/*.so >/dev/null 2>&1; then
  echo "--- building the C reference shared object ---"
  (mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
    cmake --build . >/dev/null) || {
    echo "FATAL: C build failed"
    exit 1
  }
fi
C_SO=$(ls c_src/build/*.so | head -1)
echo "C reference: $C_SO"

# ---------------------------------------------------------------------------
# 4. check + test every combination
# ---------------------------------------------------------------------------
FAIL=0
run() { # label, extra cargo args...
  local label="$1"
  shift
  echo
  echo "########## $label ##########"

  echo "--- cargo check ---"
  if ! timeout 600 cargo check --all-targets "$@" "${EXTRA[@]}" 2>&1 | tail -5; then
    echo "CHECK FAILED: $label"
    FAIL=1
    return
  fi

  echo "--- cargo test (Phases B, C, D) ---"
  local out
  out=$(timeout 600 cargo test "$@" "${EXTRA[@]}" 2>&1)
  echo "$out" | grep -E "^test result:|^test .* FAILED|error(\[|:)" | head -30
  if echo "$out" | grep -qE "^test .* FAILED|error: could not compile"; then
    echo "TEST FAILED: $label"
    FAIL=1
  else
    echo "OK: $label"
  fi
}

# The harness rebuilds the cdylib itself; tell it which features to use.
export DIFF_NO_DEFAULT_FEATURES=1

for combo in "${COMBOS[@]}"; do
  export DIFF_FEATURES="$combo"
  if [ -z "$combo" ]; then
    run "--no-default-features (empty feature set)" --no-default-features
  else
    run "--no-default-features --features $combo" --no-default-features --features "$combo"
  fi
done

# Also the implicit default build (identical here: there is no `default` feature,
# but verify rather than assume).
unset DIFF_NO_DEFAULT_FEATURES
export DIFF_FEATURES=""
run "default features"

echo
if [ "$FAIL" -eq 0 ]; then
  echo "=============================================="
  echo "ALL ${#COMBOS[@]} feature combination(s) + default: PASS"
  echo "=============================================="
else
  echo "=============================================="
  echo "FAILURES DETECTED"
  echo "=============================================="
  exit 1
fi
