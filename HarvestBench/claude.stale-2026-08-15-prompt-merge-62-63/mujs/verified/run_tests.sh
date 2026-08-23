#!/bin/bash
# Differential test driver.
#
#   ./run_tests.sh [extra cargo test args...]
#
# 1. builds the C shared library (if missing)
# 2. builds the Rust cdylib for EVERY valid feature combination
#    (Cargo.toml declares no [features], so the only combination is the
#    default/empty one — the loop below is generated from Cargo.toml so it
#    stays correct if features are ever added)
# 3. runs the integration tests against both .so files
set -u
cd "$(dirname "$0")"

# ---- 1. C shared library -------------------------------------------------
if [ ! -f c_src/build/libmujs.so ]; then
  echo "=== building C shared library ==="
  ( cd c_src && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . -j8 >/dev/null ) || exit 1
fi

# ---- 2. enumerate feature combinations ----------------------------------
# Extract feature names from Cargo.toml's [features] section (if any) and
# build the power set. With no [features] the power set is {""}.
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip()
            if n and n != 'default':
                names.append(n)
combos = ['']
for i in range(1, 1 << len(names)):
    combos.append(','.join(n for j, n in enumerate(names) if i >> j & 1))
print('\n'.join(combos))
PY
)

FAIL=0
for combo in "${FEATURES[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "############################################################"
  echo "### feature combination: $label"
  echo "############################################################"
  if [ -z "$combo" ]; then
    FLAGS=(--no-default-features)
  else
    FLAGS=(--no-default-features --features "$combo")
  fi

  echo "--- cargo check ---"
  cargo check "${FLAGS[@]}" 2>&1 | grep -E '^(error|warning: unused variable)' | head -20
  cargo check "${FLAGS[@]}" -q 2>/dev/null || { echo "CHECK FAILED for $label"; FAIL=1; continue; }

  echo "--- cargo build (cdylib) ---"
  cargo build "${FLAGS[@]}" -q || { echo "BUILD FAILED for $label"; FAIL=1; continue; }

  echo "--- symbol parity ---"
  nm -D --defined-only c_src/build/libmujs.so | awk '{print $3}' | sort -u > /tmp/.csyms
  nm -D --defined-only target/debug/libmujs.so | awk '{print $3}' | sort -u > /tmp/.rsyms
  miss=$(comm -23 /tmp/.csyms /tmp/.rsyms)
  if [ -n "$miss" ]; then
    echo "MISSING SYMBOLS in Rust .so:"; echo "$miss"; FAIL=1
  else
    echo "OK: all $(wc -l < /tmp/.csyms) C symbols present in the Rust .so"
  fi

  # The differential tests must run SERIALLY:
  #  * tests/stdout.rs redirects the process-wide fds 1 and 2;
  #  * tests/errors_bigmem.rs allocates 0.25-1 GiB per case.
  echo "--- cargo test (debug) ---"
  cargo test "${FLAGS[@]}" --no-fail-fast "$@" -- --test-threads=1 || FAIL=1

  echo "--- cargo build --release + symbol parity ---"
  cargo build --release "${FLAGS[@]}" -q || { echo "RELEASE BUILD FAILED"; FAIL=1; continue; }
  nm -D --defined-only target/release/libmujs.so | awk '{print $3}' | sort -u > /tmp/.rsymsr
  missr=$(comm -23 /tmp/.csyms /tmp/.rsymsr)
  if [ -n "$missr" ]; then
    echo "MISSING SYMBOLS in the release Rust .so:"; echo "$missr"; FAIL=1
  else
    echo "OK: all $(wc -l < /tmp/.csyms) C symbols present in the release Rust .so"
  fi

  echo "--- cargo test (release) ---"
  cargo test --release "${FLAGS[@]}" --no-fail-fast "$@" -- --test-threads=1 || FAIL=1

  # CONFIGS.md row H12: the Date tests read the process TZ. Both libraries see
  # the same TZ, so any value is a valid comparison -- sweep several, including
  # a half-hour offset and a 30-minute-DST zone.
  echo "--- Date tests under several TZ values ---"
  for tz in UTC America/New_York Asia/Kolkata Australia/Lord_Howe Europe/Berlin; do
    printf '    TZ=%-22s ' "$tz"
    TZ=$tz cargo test "${FLAGS[@]}" --test scripts_lib h12 -- --test-threads=1 2>&1 \
      | grep -E '^test result' | head -1 || FAIL=1
  done
done

echo
if [ "$FAIL" = 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
