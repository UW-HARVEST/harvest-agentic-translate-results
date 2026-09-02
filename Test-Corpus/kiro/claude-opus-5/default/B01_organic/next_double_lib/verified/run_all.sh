#!/usr/bin/env bash
# Full verification driver: builds both libraries, checks symbol parity, and
# runs the differential suite under every Cargo feature combination and both
# profiles. Run from the `translation/` directory.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "1. build the C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)
echo "C .so:    $C_SO"

# --------------------------------------------------------------------------
step "2. enumerate Cargo feature combinations"
# Every feature declared in [features] (none here => default only).
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,"");print}' Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "no [features] declared -> combinations: {default} and {--no-default-features}"
  COMBOS=("default" "none")
else
  echo "declared features: $FEATS"
  COMBOS=("default" "none")
  for f in $FEATS; do COMBOS+=("$f"); done
  COMBOS+=("$(echo $FEATS | tr ' ' ',')")
fi

flags_for() {
  case "$1" in
    default) echo "" ;;
    none)    echo "--no-default-features" ;;
    *)       echo "--no-default-features --features $1" ;;
  esac
}

# --------------------------------------------------------------------------
step "3. cargo check + build every combination x profile"
for combo in "${COMBOS[@]}"; do
  FL=$(flags_for "$combo")
  for prof in dev release; do
    PF=""; [ "$prof" = release ] && PF="--release"
    if timeout 600 cargo check $FL $PF >/tmp/chk.log 2>&1 \
       && timeout 600 cargo build $FL $PF >/tmp/bld.log 2>&1; then
      echo "  OK    check+build  combo=$combo profile=$prof"
    else
      echo "  FAIL  check+build  combo=$combo profile=$prof"; tail -20 /tmp/chk.log /tmp/bld.log; fail=1
    fi
  done
done

# --------------------------------------------------------------------------
step "4. symbol parity (nm -D), every combination x profile"
for combo in "${COMBOS[@]}"; do
  FL=$(flags_for "$combo")
  for prof in debug release; do
    R_SO="target/$prof/libnext_double_lib.so"
    [ -f "$R_SO" ] || { echo "  MISSING $R_SO"; fail=1; continue; }
    diff <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort) >/tmp/symdiff.txt
    if [ -s /tmp/symdiff.txt ]; then
      echo "  FAIL  symbol diff NON-EMPTY  combo=$combo profile=$prof"; cat /tmp/symdiff.txt; fail=1
    else
      echo "  OK    symbol diff empty      combo=$combo profile=$prof"
    fi
    # no unresolved symbols from the translated library itself
    U=$(nm -D --undefined-only "$R_SO" | grep -vE '@GLIBC|@GCC|_ITM_|__gmon_start__|gettid|statx' | wc -l)
    [ "$U" -eq 0 ] || { echo "  FAIL  $U unresolved non-libc symbols in $R_SO"; fail=1; }
  done
done

# --------------------------------------------------------------------------
step "5. differential suite: every combination x profile x cdylib profile"
for combo in "${COMBOS[@]}"; do
  FL=$(flags_for "$combo")
  for prof in dev release; do
    PF=""; TDIR=debug; [ "$prof" = release ] && { PF="--release"; TDIR=release; }
    for so_prof in debug release; do
      export DIFF_C_SO="$C_SO"
      export DIFF_RUST_SO="$PWD/target/$so_prof/libnext_double_lib.so"
      if timeout 600 cargo test $FL $PF -- --test-threads=4 >/tmp/test.log 2>&1; then
        n=$(grep -oP '\d+(?= passed)' /tmp/test.log | tail -1)
        echo "  OK    tests ($n passed)  combo=$combo profile=$prof cdylib=$so_prof"
      else
        echo "  FAIL  tests  combo=$combo profile=$prof cdylib=$so_prof"
        grep -E '^(test |error|thread|---- )' /tmp/test.log | tail -30; fail=1
      fi
      unset DIFF_C_SO DIFF_RUST_SO
    done
  done
done

# --------------------------------------------------------------------------
step "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
