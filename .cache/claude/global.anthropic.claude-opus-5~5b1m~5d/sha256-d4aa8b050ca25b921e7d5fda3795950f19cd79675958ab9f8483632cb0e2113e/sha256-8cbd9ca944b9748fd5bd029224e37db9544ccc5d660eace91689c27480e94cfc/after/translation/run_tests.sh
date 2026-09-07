#!/usr/bin/env bash
# Full verification sweep: build C + Rust, diff dynamic symbols, run the
# differential test suite under every feature combination and both profiles.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
OFFLINE="--offline"

echo "=== 1. build C shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C   .so: $C_SO"

echo "=== 2. feature combinations declared in Cargo.toml ==="
FEATURES=$(sed -n '/^\[features\]/,/^\[/p' "$CRATE/Cargo.toml" | grep -E '^[a-zA-Z0-9_-]+ *=' | cut -d= -f1 | tr -d ' ' || true)
if [ -z "$FEATURES" ]; then
  echo "(none — only the default configuration exists)"
  COMBOS=("" "--no-default-features")
else
  echo "$FEATURES"
  COMBOS=("" "--no-default-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

for PROFILE in release debug; do
  for COMBO in "${COMBOS[@]}"; do
    PROF_FLAG=""
    [ "$PROFILE" = release ] && PROF_FLAG="--release"
    echo
    echo "=== build+test  profile=$PROFILE  features='${COMBO:-default}' ==="
    (cd "$CRATE" && cargo build $PROF_FLAG $OFFLINE $COMBO >/dev/null)
    R_SO="$CRATE/target/$PROFILE/libcrc16_lib.so"
    echo "--- nm -D symbol diff (C vs Rust) ---"
    nm -D --defined-only "$C_SO" | grep -v ' [wV] ' | awk '{print $NF}' | sort -u > ${TMPDIR:-/tmp}/.c_syms.$$
    nm -D --defined-only "$R_SO" | grep -v ' [wV] ' | awk '{print $NF}' | sort -u > ${TMPDIR:-/tmp}/.r_syms.$$
    MISSING="$(comm -23 ${TMPDIR:-/tmp}/.c_syms.$$ ${TMPDIR:-/tmp}/.r_syms.$$)"
    rm -f ${TMPDIR:-/tmp}/.c_syms.$$ ${TMPDIR:-/tmp}/.r_syms.$$
    if [ -n "$MISSING" ]; then
      echo "MISSING FROM RUST .so:"; echo "$MISSING"; exit 1
    fi
    echo "symbol diff empty (0 missing)"
    (cd "$CRATE" && C_SO="$C_SO" RUST_SO="$R_SO" cargo test $PROF_FLAG $OFFLINE $COMBO -- --test-threads=4)
  done
done

echo
echo "=== ALL CONFIGURATIONS PASSED ==="
