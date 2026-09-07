#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff exported symbols, then run the
# full differential suite under EVERY cargo feature combination and under both
# the dev and release cdylib profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CRATE="$ROOT/translation"
LOGS="$CRATE/testlogs"
mkdir -p "$LOGS"
fail=0

echo "=============== 1. build the C shared library ==============="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOGS/cmake.log" 2>&1 \
  && cmake --build . >>"$LOGS/cmake.log" 2>&1 ) || { echo "C BUILD FAILED"; tail -20 "$LOGS/cmake.log"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so)"
echo "C  .so: $C_SO"

echo "=============== 2. enumerate cargo feature combinations ==============="
# Every feature name declared in Cargo.toml (there are none in this crate, so the
# only combination is the default/empty one -- computed, not assumed).
FEATURES=$(cd "$CRATE" && cargo read-manifest --offline 2>/dev/null \
  | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin).get("features",{}).keys()))')
echo "declared features: [${FEATURES}]"

# combinations: default, then --no-default-features, then each subset
COMBOS=()
COMBOS+=("DEFAULT|")
COMBOS+=("NO-DEFAULT|--no-default-features")
if [ -n "$FEATURES" ]; then
  # shellcheck disable=SC2206
  arr=($FEATURES)
  n=${#arr[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && sel+=("${arr[$i]}")
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("$joined|--no-default-features --features $joined")
  done
fi

echo "=============== 3. run every combination x profile ==============="
for combo in "${COMBOS[@]}"; do
  name="${combo%%|*}"
  flags="${combo#*|}"
  for profile in release dev; do
    if [ "$profile" = release ]; then
      pflag="--release"; sodir="release"
    else
      pflag=""; sodir="debug"
    fi
    tag="${name}-${profile}"
    echo "--- combination '$name' profile '$profile' ---"

    # shellcheck disable=SC2086
    ( cd "$CRATE" && cargo build --offline $pflag $flags ) >"$LOGS/build-$tag.log" 2>&1 || {
      echo "  BUILD FAILED (see $LOGS/build-$tag.log)"; fail=1; continue; }

    R_SO="$CRATE/target/$sodir/libsh_puts_lib.so"

    # ---- symbol parity ----
    diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) >"$LOGS/symdiff-$tag.txt"
    if [ -s "$LOGS/symdiff-$tag.txt" ]; then
      echo "  SYMBOL DIFF NON-EMPTY:"; cat "$LOGS/symdiff-$tag.txt"; fail=1
    else
      echo "  symbol diff: EMPTY ($(nm -D --defined-only "$C_SO" | wc -l) symbols)"
    fi
    # no undefined non-libc symbols
    und=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | grep -v '^_ITM_\|^__cxa\|^__gmon\|@GLIBC\|^_Unwind\|^__tls\|^$' || true)
    if [ -n "$und" ]; then
      echo "  UNRESOLVED NON-LIBC SYMBOLS:"; echo "$und"; fail=1
    fi

    # ---- differential suite ----
    # shellcheck disable=SC2086
    ( cd "$CRATE" && C_SO="$C_SO" RUST_SO="$R_SO" \
        timeout 550 cargo test --offline $pflag $flags -- --test-threads=1 ) \
      >"$LOGS/test-$tag.log" 2>&1
    rc=$?
    passed=$(grep -c '\.\.\. ok' "$LOGS/test-$tag.log")
    if [ $rc -ne 0 ]; then
      echo "  TESTS FAILED (rc=$rc, $passed passed) -- see $LOGS/test-$tag.log"
      grep -E 'FAILED|panicked at' "$LOGS/test-$tag.log" | head -20
      fail=1
    else
      echo "  tests: $passed passed"
    fi
  done
done

echo "=============== SUMMARY ==============="
if [ $fail -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
