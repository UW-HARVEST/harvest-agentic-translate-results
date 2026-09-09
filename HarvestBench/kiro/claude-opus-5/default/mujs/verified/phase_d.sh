#!/usr/bin/env bash
# Phase D driver: rebuild both libraries, diff the exported symbol tables, and
# run the whole differential test suite under EVERY cargo feature combination.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
FAIL=0

echo "=== build C shared library ==="
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }

echo "=== enumerate feature combinations (from Cargo.toml, not hard-coded) ==="
cat > /tmp/pd_features.py <<'PYEOF'
import re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n and n != 'default':
                names.append(n)
print('\n'.join(names))
PYEOF
mapfile -t RAW < <(python3 /tmp/pd_features.py)
FEATURES=()
for f in ${RAW[@]+"${RAW[@]}"}; do
  [ -n "$f" ] && FEATURES+=("$f")
done
NF=${#FEATURES[@]}
echo "features found: ${NF} :: ${FEATURES[*]-<none>}"

COMBOS=("" "--no-default-features")
if [ "$NF" -gt 0 ]; then
  total=$((1 << NF))
  for ((mask = 1; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < NF; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
  done
  COMBOS+=("--all-features")
fi

for combo in "${COMBOS[@]}"; do
  label=${combo:-<default>}
  echo
  echo "############ combination: $label ############"

  echo "--- cargo check ---"
  # shellcheck disable=SC2086
  timeout 600 cargo check $combo >/tmp/pd_check.log 2>&1
  if [ $? -ne 0 ]; then
    tail -20 /tmp/pd_check.log
    echo "CHECK FAILED for $label"
    FAIL=1
    continue
  fi

  echo "--- cargo build --release ---"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $combo >/tmp/pd_build.log 2>&1
  if [ $? -ne 0 ]; then
    tail -20 /tmp/pd_build.log
    echo "BUILD FAILED for $label"
    FAIL=1
    continue
  fi

  echo "--- nm -D symbol diff ---"
  nm -D --defined-only "$ROOT/c_src/build/libmujs.so" | awk '{print $3}' | sort >/tmp/pd_c.txt
  nm -D --defined-only "$ROOT/translation/target/release/libmujs.so" | awk '{print $3}' | sort >/tmp/pd_r.txt
  miss=$(comm -23 /tmp/pd_c.txt /tmp/pd_r.txt)
  extra=$(comm -13 /tmp/pd_c.txt /tmp/pd_r.txt)
  undef=$(nm -D --undefined-only "$ROOT/translation/target/release/libmujs.so" |
    awk '{print $2}' |
    grep -v '@GLIBC\|@GCC\|_ITM_\|__gmon_start__\|_Unwind_\|__cxa_\|__tls_get_addr' || true)
  echo "C symbols:    $(wc -l </tmp/pd_c.txt)"
  echo "Rust symbols: $(wc -l </tmp/pd_r.txt)"
  echo "missing from Rust: $(printf '%s' "$miss" | grep -c . || true)"
  echo "extra in Rust:     $(printf '%s' "$extra" | grep -c . || true)"
  [ -n "$miss" ] && { echo "MISSING:"; echo "$miss"; FAIL=1; }
  [ -n "$extra" ] && { echo "EXTRA:"; echo "$extra"; FAIL=1; }
  [ -n "$undef" ] && { echo "UNDEFINED NON-LIBC:"; echo "$undef"; FAIL=1; }

  echo "--- cargo test --release (Phases B + C) ---"
  log=/tmp/pd_test_$(echo "$label" | tr -c 'A-Za-z0-9' '_').log
  # shellcheck disable=SC2086
  timeout 900 cargo test --release $combo >"$log" 2>&1
  rc=$?
  grep -E 'test result|FAILED|DIVERGENCE|panicked|SIGABRT|SIGSEGV' "$log" | tail -20
  if [ "$rc" -ne 0 ]; then
    echo "TESTS FAILED for $label (cargo exit $rc, log $log)"
    FAIL=1
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "PHASE D: ALL COMBINATIONS PASS (symbol diff empty, all tests green)"
else
  echo "PHASE D: FAILURES PRESENT"
fi
exit "$FAIL"
