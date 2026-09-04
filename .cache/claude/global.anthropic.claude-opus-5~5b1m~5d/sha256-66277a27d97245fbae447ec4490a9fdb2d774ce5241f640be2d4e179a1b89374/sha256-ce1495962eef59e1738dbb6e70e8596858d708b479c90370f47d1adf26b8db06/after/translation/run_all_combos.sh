#!/usr/bin/env bash
# Runs the full differential suite under every feature combination and both
# profiles, plus the nm -D symbol diff.
#
# The tests capture fd 1, so the libtest harness MUST be single-threaded.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=../c_src/build/libdriver.so
if [ ! -f "$C_SO" ]; then
  ( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || exit 1
fi

# Feature combinations: Cargo.toml declares no [features], so this is the
# complete set. (Extracted mechanically below so it stays correct if any are
# added later.)
FEATS=$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -oE '^[a-zA-Z0-9_-]+' | grep -v '^\[' | tr '\n' ' ')
COMBOS=("" "--no-default-features" "--all-features")
if [ -n "${FEATS// /}" ]; then
  for f in $FEATS; do COMBOS+=("--no-default-features --features $f"); done
fi

rc=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    echo "=============================================================="
    echo "=== cargo test ${profile:-<dev>} ${combo:-<default features>}"
    echo "=============================================================="
    cargo build --offline $profile $combo >/dev/null 2>&1
    RUST_TEST_THREADS=1 timeout 600 cargo test --offline $profile $combo -- --test-threads=1
    st=$?
    [ $st -ne 0 ] && { echo "FAILED (exit $st): $profile $combo"; rc=1; }
  done
done

echo "=============================================================="
echo "=== nm -D symbol diff (C .so vs Rust .so)"
for p in debug release; do
  [ -f "target/$p/libdriver.so" ] || continue
  d=$(diff <(nm -D --defined-only "$C_SO"            | awk '$2=="T" && $3!="_init" && $3!="_fini" {print $3}' | sort) \
           <(nm -D --defined-only "target/$p/libdriver.so" | awk '$2=="T" {print $3}' | sort))
  if [ -n "$d" ]; then echo "SYMBOL DIFF ($p):"; echo "$d"; rc=1; else echo "target/$p: symbol diff empty  OK"; fi
done

echo
[ $rc -eq 0 ] && echo "ALL COMBINATIONS PASSED" || echo "SOME COMBINATIONS FAILED"
exit $rc
