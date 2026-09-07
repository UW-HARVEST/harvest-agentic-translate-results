#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff their exported symbol tables,
# and run the differential suite under EVERY feature combination in Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
fail=0

echo "=== building C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml (powerset of [features]).
# ---------------------------------------------------------------------------
FEATS=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
if [ -z "$FEATS" ]; then
    echo "=== no [features] in Cargo.toml -> 2 configurations ==="
    COMBOS+=("default:")
    COMBOS+=("no-default:--no-default-features")
else
    COMBOS+=("default:")
    mapfile -t FARR <<<"$FEATS"
    n=${#FARR[@]}
    for ((mask = 0; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FARR[i]}"; fi
        done
        COMBOS+=("no-default+[${combo}]:--no-default-features --features=${combo}")
    done
fi

# ---------------------------------------------------------------------------
for entry in "${COMBOS[@]}"; do
    label="${entry%%:*}"
    flags="${entry#*:}"
    for profile in debug release; do
        relflag=""; [ "$profile" = release ] && relflag="--release"
        echo
        echo "############ combo=$label profile=$profile ############"

        if ! timeout 600 cargo build $relflag $flags >/dev/null 2>&1; then
            echo "  BUILD FAILED"; fail=1; continue
        fi

        R_SO="target/$profile/libdriver.so"
        if [ ! -f "$R_SO" ]; then echo "  MISSING $R_SO"; fail=1; continue; fi

        echo "--- nm -D defined-symbol diff (C vs Rust) ---"
        diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
             <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) \
          && echo "  symbol tables IDENTICAL (0 missing)" \
          || { echo "  SYMBOL DIFF NON-EMPTY"; fail=1; }

        echo "--- undefined non-libc symbols in Rust .so ---"
        leftover=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
            | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^gettid$\|^statx$\|^__cxa_thread_atexit_impl$' || true)
        if [ -n "$leftover" ]; then echo "  UNRESOLVED: $leftover"; fail=1
        else echo "  none (all imports are libc/unwinder)"; fi

        echo "--- differential test suite ---"
        if timeout 600 cargo test $relflag $flags 2>&1 | tail -3; then :; else fail=1; fi
        # tail hides the exit status of cargo; re-check explicitly
        timeout 600 cargo test $relflag $flags >/dev/null 2>&1 \
          || { echo "  TESTS FAILED"; fail=1; }
    done
done

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
