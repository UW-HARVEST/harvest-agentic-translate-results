#!/usr/bin/env bash
# Phase D — run cargo check / build / test for EVERY feature combination.
#
# Features are extracted from Cargo.toml's [features] table; with none declared
# the matrix degenerates to the default build plus --no-default-features.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"

# ---------------------------------------------------------------------------
# make sure the C .so exists
# ---------------------------------------------------------------------------
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library"
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || exit 1
fi

# ---------------------------------------------------------------------------
# enumerate features
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ -z "$FEATURES" ]; then
  COMBOS+=("__default__" "__none__")
else
  # powerset of the declared features, plus the default build
  FLIST=($FEATURES)
  n=${#FLIST[@]}
  COMBOS+=("__default__" "__none__")
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="$combo,${FLIST[$i]}"; fi
    done
    COMBOS+=("${combo#,}")
  done
fi

echo "== feature combinations to verify: ${#COMBOS[@]}"

FAIL=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) ARGS=() ; label="(default)" ;;
    __none__)    ARGS=(--no-default-features) ; label="--no-default-features" ;;
    *)           ARGS=(--no-default-features --features "$combo") ; label="--features $combo" ;;
  esac

  echo "--------------------------------------------------------------"
  echo "== $label"

  if ! timeout 600 cargo check "${ARGS[@]}" >/tmp/fm_check.log 2>&1; then
    echo "   cargo check FAILED"; tail -30 /tmp/fm_check.log; FAIL=1; continue
  fi
  if ! timeout 600 cargo build --release "${ARGS[@]}" >/tmp/fm_build.log 2>&1; then
    echo "   cargo build FAILED"; tail -30 /tmp/fm_build.log; FAIL=1; continue
  fi
  if ! timeout 600 cargo test --release "${ARGS[@]}" >/tmp/fm_test.log 2>&1; then
    echo "   cargo test FAILED"; grep -E "FAILED|DIVERGENCE|panicked" /tmp/fm_test.log | head -20; FAIL=1; continue
  fi
  grep -E "^test result" /tmp/fm_test.log | sed 's/^/   /'

  # symbol parity for this combination
  CSO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
  RSO=target/release/libstr_put_lib.so
  nm -D --defined-only "$CSO" | awk '{print $NF}' | sort >/tmp/fm_c.txt
  nm -D --defined-only "$RSO" | awk '{print $NF}' | sort >/tmp/fm_r.txt
  if MISSING=$(comm -23 /tmp/fm_c.txt /tmp/fm_r.txt) && [ -z "$MISSING" ]; then
    echo "   symbol diff: empty ($(wc -l </tmp/fm_c.txt) C symbols, $(wc -l </tmp/fm_r.txt) Rust symbols)"
  else
    echo "   symbol diff NOT empty; missing from Rust:"; echo "$MISSING"; FAIL=1
  fi
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit $FAIL
