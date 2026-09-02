#!/usr/bin/env bash
# Build + test the crate under EVERY feature combination.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# adding a feature automatically widens the matrix.  Both the C `.so` and both
# Rust profiles (release + debug) are rebuilt first, because the differential
# tests compare against whichever `libdriver.so` artifacts exist on disk.
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

# ---------------------------------------------------------------------------
# 1. Reference C shared library
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build"
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
test -f "$ROOT/c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] section: the only two configurations are default and
  # --no-default-features (identical here, but both are checked).
  COMBOS+=("--no-default-features")
  COMBOS+=("")
else
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel+=("${FEATURES[$i]}"); fi
    done
    if [ "${#sel[@]}" -eq 0 ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $(
        IFS=,
        echo "${sel[*]}"
      )")
    fi
  done
  COMBOS+=("") # plus the default feature set
fi

echo "feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. For each combination: check, build both profiles, run the full suite
# ---------------------------------------------------------------------------
fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"
  echo "=============================================================="
  echo "### $label"
  echo "=============================================================="

  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --all-targets $combo; then
    echo "FAIL(check): $label"
    fail=1
    continue
  fi

  # Both cdylib profiles must exist so the differential tests exercise both.
  # shellcheck disable=SC2086
  timeout 600 cargo build $combo
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo
  # shellcheck disable=SC2086
  timeout 600 cargo build --example driver_dump $combo
  # shellcheck disable=SC2086
  timeout 600 cargo build --release --example driver_dump $combo

  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --release $combo; then
    echo "FAIL(test --release): $label"
    fail=1
    continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $combo; then
    echo "FAIL(test): $label"
    fail=1
    continue
  fi

  # Symbol parity, re-verified independently of the test harness.
  for prof in release debug; do
    so="target/$prof/libdriver.so"
    [ -f "$so" ] || continue
    if ! diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
              <(nm -D --defined-only "$so" | awk '{print $NF}' | sort) \
         | grep -q '^<'; then
      echo "symbol parity OK ($prof): every C symbol is exported by the Rust .so"
    else
      echo "FAIL(symbols): $label / $prof"
      fail=1
    fi
  done

  echo "PASS: $label"
done

echo "=============================================================="
if [ "$fail" -ne 0 ]; then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all feature combinations verified"
