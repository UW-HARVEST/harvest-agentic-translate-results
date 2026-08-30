#!/usr/bin/env bash
# Runs the full differential verification suite across every cargo feature
# combination and both profiles, and prints the nm -D symbol diff for each.
#
# Usage:  ./run_verification.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
OFFLINE="${CARGO_OFFLINE_FLAG:---offline}"
TMP="${TMPDIR:-$PWD/target}"
mkdir -p "$TMP"

# ---------------------------------------------------------------------------
# 1. Build the C ground-truth shared library exactly as the task prescribes.
# ---------------------------------------------------------------------------
echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -n1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate the feature combinations declared in Cargo.toml.
#    (features section -> powerset)
# ---------------------------------------------------------------------------
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/ /,"",a[1]); if (a[1] != "default") print a[1]}' Cargo.toml)
echo "=== declared features: ${FEATS:-<none>} ==="

COMBOS=("")            # the empty combination = --no-default-features
for f in $FEATS; do
    NEW=()
    for c in "${COMBOS[@]}"; do
        if [ -z "$c" ]; then NEW+=("$f"); else NEW+=("$c,$f"); fi
    done
    COMBOS+=("${NEW[@]}")
done

FAIL=0
SUMMARY=()

for PROFILE in debug release; do
  PROF_FLAG=""
  [ "$PROFILE" = "release" ] && PROF_FLAG="--release"

  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features=[${COMBO:-none}]"
    echo
    echo "############################################################"
    echo "### $LABEL"
    echo "############################################################"

    FEAT_FLAG=(--no-default-features)
    [ -n "$COMBO" ] && FEAT_FLAG+=(--features "$COMBO")

    # cargo check
    if ! timeout 600 cargo check $OFFLINE $PROF_FLAG "${FEAT_FLAG[@]}" \
            --all-targets >/dev/null 2>&1; then
        echo "!!! cargo check FAILED for $LABEL"
        timeout 600 cargo check $OFFLINE $PROF_FLAG "${FEAT_FLAG[@]}" --all-targets 2>&1 | tail -30
        SUMMARY+=("CHECK-FAIL  $LABEL"); FAIL=1; continue
    fi

    # build the cdylib so the symbol diff below inspects this exact config
    timeout 600 cargo build $OFFLINE $PROF_FLAG "${FEAT_FLAG[@]}" --lib >/dev/null 2>&1
    RUST_SO="target/$PROFILE/libget_predict_func_lib.so"

    # nm -D symbol diff (C defined API symbols must all be in the Rust .so)
    if [ -f "$RUST_SO" ]; then
        nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sed 's/@.*//' | sort -u > $TMP/.c_syms.$$
        nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > $TMP/.r_syms.$$
        MISSING=$(comm -23 $TMP/.c_syms.$$ $TMP/.r_syms.$$ \
                  | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$')
        if [ -n "$MISSING" ]; then
            echo "!!! MISSING SYMBOLS in Rust .so:"; echo "$MISSING"
            SUMMARY+=("SYMBOL-FAIL $LABEL"); FAIL=1
        else
            echo "symbol diff: EMPTY (all C API symbols exported by Rust)"
        fi
        rm -f $TMP/.c_syms.$$ $TMP/.r_syms.$$
    fi

    # run the tests
    if timeout 600 cargo test $OFFLINE $PROF_FLAG "${FEAT_FLAG[@]}" 2>&1 \
            | tee $TMP/.t.$$ | grep -E '^(test result|running|error)' ; then :; fi
    if grep -qE 'FAILED|error:' $TMP/.t.$$; then
        echo "!!! TESTS FAILED for $LABEL"
        grep -nE 'panicked|FAILED|^error' $TMP/.t.$$ | head -40
        SUMMARY+=("TEST-FAIL   $LABEL"); FAIL=1
    else
        NTESTS=$(grep -oE '[0-9]+ passed' $TMP/.t.$$ | awk '{s+=$1} END{print s+0}')
        echo "tests: $NTESTS passed"
        SUMMARY+=("OK ($NTESTS passed)  $LABEL")
    fi
    rm -f $TMP/.t.$$
  done
done

echo
echo "============================ SUMMARY ============================"
for s in "${SUMMARY[@]}"; do echo "  $s"; done
echo "================================================================="
if [ "$FAIL" -ne 0 ]; then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: ALL CONFIGURATIONS PASSED"
