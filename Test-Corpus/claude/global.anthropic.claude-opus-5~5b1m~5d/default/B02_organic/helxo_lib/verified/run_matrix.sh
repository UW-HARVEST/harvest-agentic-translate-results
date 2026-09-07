#!/usr/bin/env bash
# Phase D driver: runs the whole differential suite for EVERY feature
# combination declared in Cargo.toml, against BOTH the debug and the release
# Rust cdylib.
#
# Usage:  ./run_matrix.sh
set -uo pipefail
cd "$(dirname "$0")"

CARGO="cargo --offline"
FAIL=0

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations mechanically from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, itertools, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        k = line.split('=')[0].strip()
        if k != 'default':
            names.append(k)
combos = []
for r in range(len(names) + 1):
    for c in itertools.combinations(names, r):
        combos.append(','.join(c))
if not combos:
    combos = ['']
print('\n'.join(combos))
PY
)

echo "=== feature combinations discovered ==="
if [ -z "$(echo "$FEATURES" | tr -d '[:space:]')" ]; then
  echo "(none — Cargo.toml declares no [features] table; the default/empty set is the only configuration)"
else
  echo "$FEATURES" | sed 's/^$/<default\/empty>/'
fi
echo

# ---------------------------------------------------------------------------
# 2. build the C reference .so
# ---------------------------------------------------------------------------
echo "=== building the C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls ../c_src/build/lib*.so | head -1)
echo "C .so: $C_SO"
echo

# ---------------------------------------------------------------------------
# 3. for each feature combo x each profile: build the cdylib, run the suite
# ---------------------------------------------------------------------------
while IFS= read -r combo; do
  if [ -z "$combo" ]; then
    FLAGS="--no-default-features"
    LABEL="<no-default-features>"
  else
    FLAGS="--no-default-features --features $combo"
    LABEL="$combo"
  fi

  for PROFILE in debug release; do
    if [ "$PROFILE" = release ]; then
      PROF_FLAG="--release"
      SO="target/release/libhelxo_lib.so"
    else
      PROF_FLAG=""
      SO="target/debug/libhelxo_lib.so"
    fi

    echo "############################################################"
    echo "# features: $LABEL   profile: $PROFILE"
    echo "############################################################"

    $CARGO build $PROF_FLAG $FLAGS --lib >/dev/null 2>&1 \
      || { echo "  cdylib build FAILED"; FAIL=1; continue; }
    [ -f "$SO" ] || { echo "  missing $SO"; FAIL=1; continue; }

    # symbol parity, computed here as well as inside the test suite
    MISSING=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$SO"  | awk '{print $NF}' | sort -u))
    if [ -n "$MISSING" ]; then
      echo "  SYMBOL PARITY FAILED — missing from Rust .so:"; echo "$MISSING" | sed 's/^/    /'
      FAIL=1
    else
      echo "  symbol parity: OK (0 missing)"
    fi

    # Pass 1: plain.  Pass 2: glibc heap hardening — MALLOC_CHECK_=3 aborts on
    # any double free / overflow and MALLOC_PERTURB_ poisons freed memory, so a
    # divergence in the two libraries' allocate/free pattern (both share the
    # process heap) is caught rather than silently tolerated.
    for HARDEN in 0 1; do
      if [ "$HARDEN" = 1 ]; then
        export MALLOC_CHECK_=3 MALLOC_PERTURB_=42
        echo "  -- pass: MALLOC_CHECK_=3 MALLOC_PERTURB_=42"
      else
        unset MALLOC_CHECK_ MALLOC_PERTURB_
        echo "  -- pass: plain"
      fi
      HELXO_RUST_SO="$PWD/$SO" timeout 900 $CARGO test $FLAGS --tests -- --test-threads=1 2>&1 \
        | grep -E "^(test result|error|failures:)" | sed 's/^/     /'
      # shellcheck disable=SC2181
      if [ "${PIPESTATUS[0]}" != "0" ]; then
        echo "     TESTS FAILED for features='$LABEL' profile=$PROFILE harden=$HARDEN"
        FAIL=1
      fi
    done
    unset MALLOC_CHECK_ MALLOC_PERTURB_
    echo
  done
done <<< "$FEATURES"

echo "############################################################"
if [ "$FAIL" = 0 ]; then
  echo "# ALL CONFIGURATIONS PASSED"
else
  echo "# SOME CONFIGURATIONS FAILED"
fi
echo "############################################################"
exit "$FAIL"
