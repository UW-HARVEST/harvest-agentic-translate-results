#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, verify symbol parity, and run the full
# differential suite across every feature combination x profile.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== rebuild C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so)"
echo "C .so: $C_SO"

# ---- enumerate feature combinations from Cargo.toml -------------------------
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {sub(/[[:space:]]*=.*/,""); if ($0!="default") print}
' Cargo.toml)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "=== no [features] table in Cargo.toml -> single build configuration ==="
  COMBOS+=("default:")                       # plain cargo test
  COMBOS+=("no-default:--no-default-features")
  COMBOS+=("all:--all-features")
else
  n=${#FEATURES[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=()
    for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}"); done
    joined=$(IFS=,; echo "${sel[*]:-}")
    COMBOS+=("nodefault[$joined]:--no-default-features --features $joined")
  done
  COMBOS+=("default:")
  COMBOS+=("all:--all-features")
fi

for profile in release debug; do
  PFLAG=""; [ "$profile" = release ] && PFLAG="--release"
  for entry in "${COMBOS[@]}"; do
    label="${entry%%:*}"; flags="${entry#*:}"
    echo
    echo "############ profile=$profile combo=$label flags='${flags:-<none>}' ############"

    if ! timeout 600 cargo build $PFLAG $flags >/tmp/build.$profile.log 2>&1; then
      echo "BUILD FAILED"; tail -20 /tmp/build.$profile.log; FAIL=1; continue
    fi
    R_SO="target/$profile/librgb_to_hsv_lib.so"
    [ -f "$R_SO" ] || { echo "missing $R_SO"; FAIL=1; continue; }

    # ---- symbol parity: every C dynamic symbol must exist in the Rust .so ----
    nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort -u > /tmp/c.syms
    nm -D --defined-only "$R_SO"  | awk '{print $NF}' | sort -u > /tmp/r.syms
    MISSING="$(comm -23 /tmp/c.syms /tmp/r.syms)"
    if [ -n "$MISSING" ]; then
      echo "SYMBOL PARITY FAILED — missing from Rust .so:"; echo "$MISSING"; FAIL=1
    else
      echo "symbol parity OK ($(wc -l </tmp/c.syms) C symbol(s), 0 missing)"
    fi
    # ---- no undefined non-libc symbols in the Rust .so ----
    UNDEF="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
      | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind_|^gettid$|^statx$' || true)"
    if [ -n "$UNDEF" ]; then
      echo "UNRESOLVED non-libc symbols in Rust .so:"; echo "$UNDEF"; FAIL=1
    else
      echo "no unresolved non-libc symbols"
    fi

    if ! timeout 600 cargo test $PFLAG $flags >/tmp/test.$profile.log 2>&1; then
      echo "TESTS FAILED"; grep -E "^test .*FAILED|DIVERGENCE|test result" /tmp/test.$profile.log | head -30; FAIL=1
    else
      grep -E "test result" /tmp/test.$profile.log
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then echo "=== ALL CONFIGURATIONS PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit $FAIL
