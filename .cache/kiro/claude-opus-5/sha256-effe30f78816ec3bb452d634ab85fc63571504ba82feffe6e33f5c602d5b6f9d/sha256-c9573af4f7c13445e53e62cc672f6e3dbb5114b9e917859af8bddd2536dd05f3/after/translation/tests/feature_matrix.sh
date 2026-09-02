#!/usr/bin/env bash
# Phase D -- run the whole differential suite under EVERY feature combination.
#
# Feature combinations are extracted from the crate's own metadata rather than
# hard-coded, so this stays correct if `[features]` is ever added to Cargo.toml.
#
# Usage:  ./tests/feature_matrix.sh
# Run from the `translation/` crate root (or anywhere -- it cd's itself).

set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(cd "$CRATE_DIR/.." && pwd)"
cd "$CRATE_DIR"

TIMEOUT=${TIMEOUT:-600}

echo "=============================================================="
echo "0. Building the C shared library (ground truth)"
echo "=============================================================="
(
  cd "$WORK_DIR/c_src"
  mkdir -p build
  cd build
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
C_SO="$WORK_DIR/c_src/build/libdriver.so"
test -f "$C_SO" || { echo "FAIL: $C_SO was not produced"; exit 1; }
echo "ok: $C_SO"

# -------------------------------------------------------------------------
# Discover the feature list from cargo metadata.
# -------------------------------------------------------------------------
mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 2>/dev/null \
    | tr ',' '\n' \
    | sed -n 's/.*"features":{\(.*\)/\1/p' >/dev/null 2>&1 || true
  # Robust extraction: list the declared feature names, one per line.
  cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; [print(f) for p in json.load(sys.stdin)["packages"] for f in p["features"] if f != "default"]'
)

echo
echo "=============================================================="
echo "1. Declared cargo features"
echo "=============================================================="
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "none -- the crate declares no [features], so the default build is the"
  echo "only build configuration. The matrix below still runs the default,"
  echo "--no-default-features and --all-features variants to prove it."
else
  printf '  %s\n' "${FEATURES[@]}"
fi

# -------------------------------------------------------------------------
# Build the list of combinations: default, no-default, all-features, and the
# full power set of declared features (if any).
# -------------------------------------------------------------------------
COMBOS=()
COMBOS+=("")                                   # default
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")

n=${#FEATURES[@]}
if [ "$n" -gt 0 ] && [ "$n" -le 12 ]; then
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then sel+=("${FEATURES[b]}"); fi
    done
    if [ "${#sel[@]}" -gt 0 ]; then
      joined=$(
        IFS=,
        echo "${sel[*]}"
      )
      COMBOS+=("--no-default-features --features $joined")
      COMBOS+=("--features $joined")
    fi
  done
fi

echo
echo "=============================================================="
echo "2. cargo check for every combination"
echo "=============================================================="
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  printf '  check %-56s ' "$label"
  # shellcheck disable=SC2086
  if timeout "$TIMEOUT" cargo check --release --all-targets $combo >/tmp/fm_check.log 2>&1; then
    echo "ok"
  else
    echo "FAIL"
    tail -30 /tmp/fm_check.log
    exit 1
  fi
done

echo
echo "=============================================================="
echo "3. Symbol parity + full differential suite per combination"
echo "=============================================================="
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "---- $label ----"

  # shellcheck disable=SC2086
  timeout "$TIMEOUT" cargo build --release $combo >/tmp/fm_build.log 2>&1 || {
    echo "FAIL: build"
    tail -30 /tmp/fm_build.log
    exit 1
  }
  RUST_SO="$CRATE_DIR/target/release/libdriver.so"
  test -f "$RUST_SO" || { echo "FAIL: $RUST_SO not produced"; exit 1; }

  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort >/tmp/fm_c.txt
  nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort >/tmp/fm_r.txt
  missing=$(comm -23 /tmp/fm_c.txt /tmp/fm_r.txt)
  if [ -n "$missing" ]; then
    echo "FAIL: symbols exported by C but missing from Rust:"
    echo "$missing"
    exit 1
  fi
  echo "symbol parity: ok ($(wc -l </tmp/fm_c.txt) C exports, 0 missing)"

  # shellcheck disable=SC2086
  if timeout "$TIMEOUT" cargo test --release $combo >/tmp/fm_test.log 2>&1; then
    grep -E '^test result:' /tmp/fm_test.log | sed 's/^/  /'
  else
    echo "FAIL: tests"
    tail -60 /tmp/fm_test.log
    exit 1
  fi
done

echo
echo "=============================================================="
echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combination(s))"
echo "=============================================================="
