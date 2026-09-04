#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY cargo feature
# combination declared in Cargo.toml.
#
# Usage:  ./run_all_features.sh
#
# The feature list is extracted from Cargo.toml, never hard-coded, so this
# script keeps working if features are added later.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=../c_src/build/libzstd.so
if [ ! -f "$C_SO" ]; then
  echo "building the C reference library..."
  (cd ../c_src && mkdir -p build && cd build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . -j "$(nproc)" >/dev/null) || exit 1
fi

# ---- extract the [features] table (names only, excluding "default") ---------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/      { inf=1; next }
    /^\[/                { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "== features declared in Cargo.toml: ${#FEATURES[@]} =="
for f in "${FEATURES[@]:-}"; do [ -n "$f" ] && echo "   - $f"; done

# ---- build the list of combinations to test ---------------------------------
COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No features are declared: the crate has exactly ONE configuration.
  COMBOS+=("default")
else
  COMBOS+=("default")
  COMBOS+=("--no-default-features")
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="$sel,${FEATURES[$i]}"; fi
    done
    sel="${sel#,}"
    COMBOS+=("--no-default-features --features $sel")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "default" ]; then
    flags=()
    label="default"
  else
    # shellcheck disable=SC2206
    flags=($combo)
    label="$combo"
  fi
  echo
  echo "=============================================================="
  echo "== cargo check  [$label]"
  echo "=============================================================="
  if ! timeout 600 cargo check --offline --release "${flags[@]}" 2>&1 | tail -5; then
    echo "*** cargo check FAILED for [$label]"
    fail=1
    continue
  fi
  echo "== cargo build --release  [$label]"
  if ! timeout 600 cargo build --offline --release "${flags[@]}" 2>&1 | tail -3; then
    echo "*** cargo build FAILED for [$label]"
    fail=1
    continue
  fi
  echo "== symbol parity  [$label]"
  missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"                 | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only target/release/libzstd.so | awk '{print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "*** MISSING SYMBOLS for [$label]:"
    echo "$missing"
    fail=1
  else
    echo "   0 missing symbols"
  fi
  echo "== cargo test  [$label]"
  timeout 3000 cargo test --offline --release --no-fail-fast "${flags[@]}" \
      > "test_${label// /_}.log" 2>&1
  rc=$?
  grep -E '^test result|Running tests/|FAILED|SIGSEGV|SIGABRT|SIGFPE' "test_${label// /_}.log"
  if [ "$rc" -ne 0 ]; then
    echo "*** cargo test FAILED for [$label] (exit $rc); see test_${label// /_}.log"
    fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combination(s))"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$fail"
