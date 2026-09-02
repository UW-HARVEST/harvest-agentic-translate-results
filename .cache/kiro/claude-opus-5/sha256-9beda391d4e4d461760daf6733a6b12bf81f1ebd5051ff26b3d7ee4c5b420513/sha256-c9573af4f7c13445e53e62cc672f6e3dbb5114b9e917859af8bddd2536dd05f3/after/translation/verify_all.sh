#!/usr/bin/env bash
# Full verification driver: builds the C reference, enumerates EVERY cargo
# feature combination, and runs the whole differential suite under each one for
# both the debug and release cdylib.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=============================================================="
echo " 1. Build the C reference shared library"
echo "=============================================================="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C .so: $C_SO"

echo
echo "=============================================================="
echo " 2. Enumerate feature combinations"
echo "=============================================================="
# All declared features (excluding "default"); the power set is the combo list.
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', s, re.M | re.S)
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
if [ -z "$FEATURES" ]; then
  echo "No [features] declared -> single configuration."
  COMBOS=("default" "no-default")
else
  echo "Features: $FEATURES"
  COMBOS=("default" "no-default")
  # power set of declared features, each run with --no-default-features
  n=0; arr=($FEATURES); total=$((1 << ${#arr[@]}))
  while [ $n -lt $total ]; do
    sel=""
    for ((i=0; i<${#arr[@]}; i++)); do
      if (( (n >> i) & 1 )); then sel="$sel,${arr[i]}"; fi
    done
    [ -n "$sel" ] && COMBOS+=("feat:${sel#,}")
    n=$((n + 1))
  done
fi
printf ' combo: %s\n' "${COMBOS[@]}"

run_combo () {
  local combo="$1" flags=""
  case "$combo" in
    default)    flags="" ;;
    no-default) flags="--no-default-features" ;;
    feat:*)     flags="--no-default-features --features ${combo#feat:}" ;;
  esac
  echo
  echo "--------------------------------------------------------------"
  echo " combo: $combo   (cargo $flags)"
  echo "--------------------------------------------------------------"

  # cargo check first, then BOTH cdylib profiles so the tests can load both.
  timeout 600 cargo check  $flags               >/dev/null 2>&1 || { echo "  cargo check FAILED";  FAIL=1; return; }
  timeout 600 cargo build  $flags               >/dev/null 2>&1 || { echo "  debug build FAILED";  FAIL=1; return; }
  timeout 600 cargo build  $flags --release     >/dev/null 2>&1 || { echo "  release build FAILED"; FAIL=1; return; }

  # symbol parity gate
  local rso missing
  for rso in target/debug/libsiphash_lib.so target/release/libsiphash_lib.so; do
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$rso"  | awk '{print $3}' | sort -u))
    if [ -n "$missing" ]; then
      echo "  SYMBOL PARITY FAILED for $rso:"; echo "$missing" | sed 's/^/    missing: /'
      FAIL=1
    else
      echo "  symbol parity OK: $rso"
    fi
  done

  local t
  for t in phase_d_symbols phase_c_errors phase_b_valid siphash_stdout; do
    if timeout 600 cargo test $flags --test "$t" -- --test-threads=4 >/tmp/vt_$t.log 2>&1; then
      echo "  PASS $(printf '%-18s' "$t") $(grep -m1 '^test result' /tmp/vt_$t.log)"
    else
      echo "  FAIL $(printf '%-18s' "$t")"
      grep -E '^test .* FAILED|panicked at|test result' /tmp/vt_$t.log | head -12 | sed 's/^/       /'
      FAIL=1
    fi
  done
}

echo
echo "=============================================================="
echo " 3. Run the differential suite under every combination"
echo "=============================================================="
for c in "${COMBOS[@]}"; do run_combo "$c"; done

echo
echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo " ALL CONFIGURATIONS PASSED"
else
  echo " FAILURES PRESENT (see above)"
fi
echo "=============================================================="
exit "$FAIL"
