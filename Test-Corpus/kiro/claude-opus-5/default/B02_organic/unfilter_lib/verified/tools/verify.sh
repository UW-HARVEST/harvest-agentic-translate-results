#!/usr/bin/env bash
# End-to-end verification driver.
#
#   1. build the C `.so` exactly as documented (asserts LIVE, no NDEBUG)
#   2. build a second C `.so` with -DNDEBUG (same code, asserts elided) --
#      bit-identical to (1) on every input where the asserts hold, and the only
#      way to differentially test the assert-guarded error paths
#   3. build the instrumented UB oracle (tools/build_lens_probe.sh)
#   4. build the Rust cdylib and diff `nm -D` against the C `.so`
#   5. run every test binary under EVERY cargo feature combination
#
# c_src is never modified; only build directories are created inside it, exactly
# as the documented build command does.
set -uo pipefail

crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
root="$(cd "$crate/.." && pwd)"
csrc="$root/c_src"
fail=0
step() { printf '\n=== %s ===\n' "$*"; }
note() { printf '  %s\n' "$*"; }
bad() { printf '  FAIL: %s\n' "$*"; fail=1; }

step "1/5  build C .so as documented"
( mkdir -p "$csrc/build" && cd "$csrc/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || bad "C build failed"
c_so="$(find "$csrc/build" -maxdepth 1 -name 'lib*.so' | head -1)"
note "C .so: $c_so"

step "2/5  build C .so with -DNDEBUG"
( mkdir -p "$csrc/build_ndebug" && cd "$csrc/build_ndebug" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_C_FLAGS=-DNDEBUG >/dev/null \
  && cmake --build . >/dev/null ) || bad "C NDEBUG build failed"

step "3/5  build the undefined-behaviour oracle"
"$crate/tools/build_lens_probe.sh" >/dev/null || bad "probe build failed"
note "ok"

step "4/5  build Rust cdylib and diff exported symbols"
( cd "$crate" && cargo build --release ) >/dev/null 2>&1 || bad "cargo build --release failed"
r_so="$crate/target/release/libunfilter_lib.so"
c_syms="$(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u)"
r_syms="$(nm -D --defined-only "$r_so" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u)"
missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
note "C exports $(echo "$c_syms" | grep -c .) symbols"
if [ -n "$missing" ]; then
  bad "missing from the Rust .so:"; echo "$missing" | sed 's/^/      /'
else
  note "symbol diff is EMPTY (0 missing)"
fi
undef="$(nm -D --undefined-only "$r_so" | awk '{print $NF}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__cxa_|^__gmon_start__$|^__tls_get_addr$|^statx$|^gettid$' || true)"
if [ -n "$undef" ]; then
  bad "non-libc undefined symbols in the Rust .so:"; echo "$undef" | sed 's/^/      /'
else
  note "0 undefined non-libc symbols"
fi

step "5/5  run the test suite under every feature combination"
# Enumerate the [features] table; the cross-product of optional features is the
# set of configurations to verify. This crate declares none, so the only
# combination is the default (empty) one -- computed, not assumed.
feats="$(cd "$crate" && python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'(?ms)^\[features\]\s*$(.*?)(?=^\[|\Z)', txt)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            names.append(k)
print(' '.join(names))
PY
)"
combos=()
if [ -z "$feats" ]; then
  note "no [features] declared -> 1 configuration (default)"
  combos+=("__default__")
else
  read -r -a arr <<<"$feats"
  n=${#arr[@]}
  for ((mask=0; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then sel="${sel:+$sel,}${arr[i]}"; fi
    done
    combos+=("$sel")
  done
  note "features: $feats -> ${#combos[@]} combinations"
fi

for combo in "${combos[@]}"; do
  if [ "$combo" = "__default__" ]; then
    label="default"; flags=()
  elif [ -z "$combo" ]; then
    label="--no-default-features"; flags=(--no-default-features)
  else
    label="--no-default-features --features $combo"
    flags=(--no-default-features --features "$combo")
  fi
  printf '\n--- %s ---\n' "$label"
  ( cd "$crate" && cargo build --release "${flags[@]}" ) >/dev/null 2>&1 \
    || { bad "build failed for $label"; continue; }
  ( cd "$crate" && timeout 600 cargo test --release "${flags[@]}" \
      --test smoke --test phase_b_inflate --test phase_b_unfilter \
      -- --test-threads=1 2>&1 | grep -E 'test result|^error' )
  [ "${PIPESTATUS[0]}" = 0 ] || bad "Phase B failed for $label"
  ( cd "$crate" && timeout 600 cargo test --release "${flags[@]}" \
      --test phase_c_errors -- --test-threads=1 2>&1 \
      | grep -vE 'Assertion' | grep -E 'test result|^error' )
  [ "${PIPESTATUS[0]}" = 0 ] || bad "Phase C failed for $label"
  ( cd "$crate" && FUZZ_ITERS="${FUZZ_ITERS:-400}" timeout 600 cargo test --release "${flags[@]}" \
      --test fuzz_differential -- --test-threads=1 --nocapture 2>&1 \
      | grep -E 'identical|test result|^error' )
  [ "${PIPESTATUS[0]}" = 0 ] || bad "fuzz failed for $label"
done

printf '\n=== summary ===\n'
if [ "$fail" = 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
