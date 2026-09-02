#!/usr/bin/env bash
# Phase D driver: symbol parity + Phases B/C under every feature combination
# and every build profile.  Run from the repository root.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 0. build the C reference library
# ---------------------------------------------------------------------------
say "building the C reference library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -name '*.so' | head -1)"
echo "C_SO=$C_SO"

# ---------------------------------------------------------------------------
# 1. enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(python3 - "$CRATE/Cargo.toml" <<'PY'
import sys, re
txt = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)
echo "declared features: '${FEATURES}'"

COMBOS=()
if [ -z "${FEATURES// /}" ]; then
  # no [features] table -> the default build is the only configuration
  COMBOS+=("<default>")
else
  COMBOS+=("<default>" "<none>")
  for f in $FEATURES; do COMBOS+=("$f"); done
  # all features together
  COMBOS+=("$(echo "$FEATURES" | tr ' ' ',')")
  # every pair
  arr=($FEATURES)
  for ((i=0; i<${#arr[@]}; i++)); do
    for ((j=i+1; j<${#arr[@]}; j++)); do COMBOS+=("${arr[$i]},${arr[$j]}"); done
  done
fi

flags_for() {
  case "$1" in
    "<default>") echo "" ;;
    "<none>")    echo "--no-default-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# ---------------------------------------------------------------------------
# 2. for each combination x profile: build, diff symbols, run all tests
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  FLAGS="$(flags_for "$combo")"
  for profile in release debug; do
    say "combo=$combo profile=$profile"
    if [ "$profile" = release ]; then PF="--release"; else PF=""; fi

    ( cd "$CRATE" && timeout 600 cargo build $PF $FLAGS ) >/dev/null 2>&1 \
      || { echo "BUILD FAILED (combo=$combo profile=$profile)"; FAIL=1; continue; }

    R_SO="$CRATE/target/$profile/libsh_geti_lib.so"
    [ -f "$R_SO" ] || { echo "MISSING $R_SO"; FAIL=1; continue; }

    # --- symbol parity gate -------------------------------------------------
    nm -D --defined-only "$C_SO" | awk '{print $3}' | grep -v '^$' | sort -u > /tmp/pd_c.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | grep -v '^$' | sort -u > /tmp/pd_r.txt
    MISSING="$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)"
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED (combo=$combo profile=$profile); missing from Rust:"
      echo "$MISSING"
      FAIL=1
    else
      echo "symbols: $(wc -l < /tmp/pd_c.txt)/$(wc -l < /tmp/pd_c.txt) present, 0 missing"
    fi
    # undefined non-libc symbols in the Rust .so
    UNDEF="$(nm -D -u "$R_SO" | awk '{print $2}' \
             | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_' || true)"
    if [ -n "$UNDEF" ]; then
      echo "UNDEFINED NON-LIBC SYMBOLS (combo=$combo profile=$profile): $UNDEF"
      FAIL=1
    fi

    # --- Phases B + C -------------------------------------------------------
    ( cd "$CRATE" && C_SO="$C_SO" RUST_SO="$R_SO" \
      timeout 600 cargo test $PF $FLAGS 2>&1 ) > /tmp/pd_test.log
    if grep -qE '^test result: FAILED|error\[|error:' /tmp/pd_test.log; then
      echo "TESTS FAILED (combo=$combo profile=$profile)"
      grep -E '^test result|FAILED|panicked|^error' /tmp/pd_test.log | head -30
      FAIL=1
    else
      grep -E '^test result' /tmp/pd_test.log
    fi
  done
done

say "SUMMARY"
if [ "$FAIL" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
