#!/usr/bin/env bash
# Phase D: enumerate every feature combination declared in Cargo.toml, then for
# each one build the cdylib, diff its exported symbols against the C .so, and
# run the full differential suite.
#
# Usage: ./scripts/verify_all.sh
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$(cd .. && pwd)"

C_SO=$(ls "$ROOT"/c_src/build/*.so 2>/dev/null | head -1)
if [ -z "$C_SO" ]; then
  echo "C .so not found; build it first:"
  echo "  cd $ROOT/c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi
echo "C  .so: $C_SO"

# --- enumerate features -----------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, sys
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
if not m:
    print('', end='')
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(' '.join(n for n in names if n != 'default'), end='')
PY
)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the only configurations are"
  echo "the default one and --no-default-features."
  COMBOS=("default" "--no-default-features")
else
  # full power set of the declared features
  read -ra FARR <<< "$FEATURES"
  n=${#FARR[@]}
  COMBOS=("default")
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then sel="$sel,${FARR[$b]}"; fi
    done
    COMBOS+=("--no-default-features --features ${sel#,}")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "==================================================================="
  echo "CONFIGURATION: $combo"
  echo "==================================================================="
  if [ "$combo" = "default" ]; then
    FLAGS=()
  else
    read -ra FLAGS <<< "$combo"
  fi

  if ! cargo build --release -q "${FLAGS[@]}" 2>&1 | tail -5; then
    echo "  BUILD FAILED"
    fail=1
    continue
  fi
  R_SO=target/release/libgjk_cache_lib.so
  if [ ! -f "$R_SO" ]; then
    echo "  Rust .so missing"
    fail=1
    continue
  fi

  # --- symbol parity ---
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/vc_syms.txt
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/vr_syms.txt
  missing=$(comm -23 /tmp/vc_syms.txt /tmp/vr_syms.txt)
  nc=$(wc -l < /tmp/vc_syms.txt)
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY: FAIL — missing from Rust .so:"
    echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  SYMBOL PARITY: OK ($nc/$nc C symbols exported by the Rust .so)"
  fi
  undef=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
    | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_' || true)
  if [ -n "$undef" ]; then
    echo "  UNDEFINED NON-LIBC SYMBOLS: FAIL"
    echo "$undef" | sed 's/^/    /'
    fail=1
  else
    echo "  UNDEFINED NON-LIBC SYMBOLS: none"
  fi

  # --- differential suite ---
  out=$(timeout 900 cargo test --release "${FLAGS[@]}" 2>&1)
  echo "$out" | grep -E '^test result' | sed 's/^/  /'
  nfail=$(echo "$out" | grep -oE '[0-9]+ failed' | awk '{s+=$1} END{print s+0}')
  if [ "${nfail:-0}" -ne 0 ]; then
    echo "  TESTS: FAIL ($nfail)"
    echo "$out" | grep -E '^---- |panicked at' | head -20 | sed 's/^/    /'
    fail=1
  else
    echo "  TESTS: OK"
  fi
done

echo
echo "==================================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
echo "==================================================================="
exit "$fail"
