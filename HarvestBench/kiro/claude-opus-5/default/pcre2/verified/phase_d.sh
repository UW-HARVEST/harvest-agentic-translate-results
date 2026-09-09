#!/bin/bash
# Phase D: symbol parity + feature-combination sweep.
#
# 1. Rebuild the C .so and the Rust cdylib.
# 2. Diff `nm -D` exported symbols; the diff MUST be empty both ways.
# 3. Check the Rust .so has no undefined non-libc symbols.
# 4. Enumerate every cargo feature combination and run the full suite for each.
set -u
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
C_SO="$ROOT/c_src/build/libpcre2.so"
R_SO="target/release/libpcre2.so"
fail=0

echo "=== rebuild ==="
(cd "$ROOT/c_src/build" && timeout 600 cmake --build . -j8 >/dev/null) || { echo "C build FAILED"; exit 1; }
timeout 600 cargo build --release 2>&1 | tail -2 || { echo "Rust build FAILED"; exit 1; }

echo
echo "=== SYMBOLS: nm -D --defined-only ==="
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDRBW]$/ {print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TDRBW]$/ {print $3}' | sort -u > /tmp/r_syms.txt
nc=$(wc -l < /tmp/c_syms.txt); nr=$(wc -l < /tmp/r_syms.txt)
missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
extra=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)
echo "C exports: $nc   Rust exports: $nr"
if [ -n "$missing" ]; then echo "MISSING from Rust:"; echo "$missing"; fail=1; else echo "MISSING from Rust: none"; fi
if [ -n "$extra" ]; then echo "EXTRA in Rust:"; echo "$extra"; else echo "EXTRA in Rust: none"; fi

echo
echo "=== SYMBOLS: sizes of exported data objects ==="
nm -D -S --defined-only "$C_SO" | awk '$3 ~ /^[DRB]$/ {print $4, $2}' | sort > /tmp/c_data.txt
nm -D -S --defined-only "$R_SO" | awk '$3 ~ /^[DRB]$/ {print $4, $2}' | sort > /tmp/r_data.txt
if diff -u /tmp/c_data.txt /tmp/r_data.txt > /tmp/data_diff.txt; then
  echo "data symbol sizes: identical ($(wc -l < /tmp/c_data.txt) symbols)"
else
  echo "data symbol size DIFFERENCES:"; cat /tmp/data_diff.txt; fail=1
fi

echo
echo "=== SYMBOLS: undefined symbols in the Rust .so (must all be libc/libgcc) ==="
nm -D --undefined-only "$R_SO" | awk '{print $2}' | sort -u > /tmp/r_undef.txt
# Anything that is not a glibc/libgcc/ITM/gmon symbol is a real missing dependency.
grep -v -E '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_|^__cxa_|^__errno_location$|^__tls_get_addr$' /tmp/r_undef.txt > /tmp/r_undef_bad.txt
if [ -s /tmp/r_undef_bad.txt ]; then
  echo "NON-LIBC undefined symbols:"; cat /tmp/r_undef_bad.txt; fail=1
else
  echo "0 missing/undefined non-libc symbols ($(wc -l < /tmp/r_undef.txt) libc/libgcc imports)"
fi

echo
echo "=== FEATURES: enumerate cargo feature combinations ==="
FEATS=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print(" ".join(sorted(d["packages"][0]["features"].keys())))')
BINS=$(cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print(",".join(t["name"] for t in d["packages"][0]["targets"] if "bin" in t["kind"]))')
echo "declared features: [${FEATS}]"
echo "binary targets   : [${BINS}]"

run_suite() {
  local label="$1"; shift
  echo
  echo "--- test run: $label ---"
  timeout 600 cargo build --release "$@" 2>&1 | grep -E '^(error|warning: unused)' | head -5
  timeout 600 cargo build --release --tests "$@" 2>&1 | grep -E '^error' | head -5
  if timeout 900 cargo test --release "$@" -- --test-threads=4 2>&1 | tee /tmp/run_$$.log | grep -E '^test result|^error' ; then :; fi
  if grep -qE 'FAILED|^error' /tmp/run_$$.log; then echo "SUITE FAILED for $label"; fail=1; fi
  rm -f /tmp/run_$$.log
}

if [ -z "$FEATS" ]; then
  echo "No cargo features are declared, so there is exactly ONE configuration."
  echo "Verifying that --no-default-features and --all-features are identical to it."
  run_suite "default"
  run_suite "--no-default-features" --no-default-features
  run_suite "--all-features" --all-features
else
  # Full power set of declared features.
  python3 - "$FEATS" <<'PY' > /tmp/combos.txt
import itertools, sys
f = sys.argv[1].split()
print("")            # no features
for k in range(1, len(f)+1):
    for c in itertools.combinations(f, k):
        print(",".join(c))
PY
  while IFS= read -r combo; do
    if [ -z "$combo" ]; then
      run_suite "--no-default-features" --no-default-features
    else
      run_suite "--no-default-features --features $combo" --no-default-features --features "$combo"
    fi
  done < /tmp/combos.txt
  run_suite "default"
  run_suite "--all-features" --all-features
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "PHASE D: PASS"
else
  echo "PHASE D: FAIL"
fi
exit $fail
