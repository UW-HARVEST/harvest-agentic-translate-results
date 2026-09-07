#!/usr/bin/env bash
# Phase D: symbol parity + every feature combination.
# Enumerates the [features] table from Cargo.toml (there is none, so the only
# combinations are the default build and --no-default-features) and runs
# cargo check + the full differential suite under each.
set -u
cd "$(dirname "$0")/.." || exit 1
ROOT=..
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
R_SO=target/release/libread_side_info_lib.so

fail=0

echo "=== Feature combinations discovered in Cargo.toml ==="
FEATURES=$(python3 - <<'PY'
import re
s=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M|re.S)
if not m:
    print("")  # no features
else:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            print(line.split('=')[0].strip())
PY
)
if [ -z "$FEATURES" ]; then
  echo "(none declared -> combinations are: <default>, --no-default-features)"
  COMBOS=("" "--no-default-features")
else
  echo "$FEATURES"
  # powerset of the declared features, always with --no-default-features
  COMBOS=("" "--no-default-features")
  mapfile -t FL <<< "$FEATURES"
  n=${#FL[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    set=""
    for ((i=0; i<n; i++)); do
      (( mask & (1<<i) )) && set="${set:+$set,}${FL[$i]}"
    done
    COMBOS+=("--no-default-features --features $set")
  done
fi

for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=================================================================="
  echo "FEATURE COMBO: $label"
  echo "=================================================================="

  if ! timeout 600 cargo check $combo -q 2>&1 | tail -5; then
    echo "  cargo check FAILED"; fail=1; continue
  fi
  echo "  cargo check: ok"

  if ! timeout 600 cargo build --release $combo -q 2>&1 | tail -5; then
    echo "  cargo build FAILED"; fail=1; continue
  fi

  # --- symbol parity for this combo ---
  c_syms=$(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$R_SO" | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  echo "  C exports  : $(echo "$c_syms" | grep -c .) symbol(s)"
  echo "  Rust exports: $(echo "$r_syms" | grep -c .) symbol(s)"
  if [ -n "$missing" ]; then
    echo "  *** MISSING FROM RUST .so: ***"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  symbol diff: EMPTY (all C symbols exported by Rust)"
  fi
  # undefined non-libc symbols in the Rust .so
  undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | \
          grep -vE '^(_ITM_|__gmon_start__|__cxa_|_Unwind_|__tls_|__stack_chk)' | \
          grep -vE '@GLIBC|@GCC|^$' )
  if [ -n "$undef" ]; then
    echo "  *** UNDEFINED NON-LIBC SYMBOLS IN RUST .so: ***"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  undefined non-libc symbols: 0"
  fi

  # --- full differential suite under this combo ---
  if timeout 600 cargo test --release $combo -q -- --test-threads=4 2>&1 | tail -8; then
    echo "  differential suite: ok"
  else
    echo "  *** differential suite FAILED ***"; fail=1
  fi
done

echo
echo "=================================================================="
if [ "$fail" -eq 0 ]; then echo "PHASE D: ALL COMBOS PASSED"; else echo "PHASE D: FAILURES PRESENT"; fi
exit "$fail"
