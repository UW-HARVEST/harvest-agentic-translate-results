#!/usr/bin/env bash
# Full differential verification: C .so vs Rust .so, both loaded via libloading.
#
# Two non-obvious things this script exists to guarantee:
#
#  1. `cargo test --test X` does NOT rebuild a cdylib that the tests only
#     `dlopen` by path. Skipping the build therefore silently tests a STALE
#     .so — an injected bug in src/lib.rs still "passes". (Verified: injecting
#     `rem_euclid` for `%` passed 42/42 against a stale .so, and failed 15
#     tests once rebuilt.) tests/common/mod.rs has a staleness guard that
#     aborts if this happens; this script makes sure it never does.
#
#  2. `compare_allocations()` compares two malloc() ADDRESSES, so its result
#     depends on glibc's per-thread tcache LIFO state. Parallel test threads
#     refill that tcache from the shared arena, which intermittently produces
#     a bogus off-by-one "divergence". Tests are therefore run with
#     --test-threads=1 (the harness also takes a process-wide lock and retries).
set -euo pipefail
cd "$(dirname "$0")"

echo "=== building C shared library ==="
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
C_SO=$(ls ../c_src/build/lib*.so)
echo "    $C_SO"

# Enumerate feature combinations mechanically from Cargo.toml.
# (This crate declares no [features], so the set is just the default build;
#  --no-default-features is still exercised to prove it.)
mapfile -t FEATS < <(python3 - <<'PY'
import re,sys
s=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M|re.S)
names=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            n=line.split('=')[0].strip().strip('"')
            if n!='default': names.append(n)
print('')                        # default build
print('--no-default-features')   # nothing enabled
for n in names:
    print(f'--no-default-features --features {n}')
if names:
    print('--no-default-features --features ' + ','.join(names))
    print('--all-features')
PY
)

FAIL=0
for f in "${FEATS[@]}"; do
  label="${f:-<default>}"
  echo
  echo "############################################################"
  echo "### feature combo: $label"
  echo "############################################################"
  # shellcheck disable=SC2086
  cargo build --offline --release $f
  # shellcheck disable=SC2086
  cargo build --offline --release --examples $f

  echo "--- symbol parity ---"
  diff <(nm -D --defined-only "$C_SO"        | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libarity_lib.so | awk '{print $3}' | sort) \
    && echo "    symbol sets IDENTICAL" \
    || { echo "    *** SYMBOL MISMATCH ***"; FAIL=1; }

  echo "--- tests (single-threaded) ---"
  # shellcheck disable=SC2086
  cargo test --offline --release $f -- --test-threads=1 || FAIL=1
done

echo
if [ "$FAIL" -eq 0 ]; then echo "ALL FEATURE COMBOS PASSED"; else echo "FAILURES PRESENT"; exit 1; fi
