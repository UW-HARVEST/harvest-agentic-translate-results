#!/usr/bin/env bash
# Full differential verification: every feature combination x both profiles.
#
# `cargo test` alone is NOT sufficient: cargo does not "uplift" a cdylib that no
# test links, so the cdylib must be built explicitly for each profile before the
# tests dlopen it. The harness also refuses to run against a `.so` older than
# src/lib.rs, so a missing build fails loudly instead of silently passing.
set -u
cd "$(dirname "$0")"

# The C reference must exist first.
if [ ! -f ../c_src/build/libdriver.so ]; then
  ( cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
fi

CARGO_FLAGS="${CARGO_OFFLINE:---offline}"
fail=0
for combo in "" "--no-default-features" "--all-features"; do
  for prof in "" "--release"; do
    label="${combo:-<default-features>} ${prof:-<debug>}"
    if ! cargo build $CARGO_FLAGS --lib $combo $prof >/dev/null 2>&1; then
      echo "BUILD-FAIL  $label"; fail=1; continue
    fi
    out=$(timeout 580 cargo test $CARGO_FLAGS $combo $prof 2>&1)
    n=$(printf '%s\n' "$out" | grep -cE '^test .* \.\.\. ok$')
    if printf '%s\n' "$out" | grep -qE 'result: FAILED|^error'; then
      echo "FAIL        $label"
      printf '%s\n' "$out" | grep -E 'result:|FAILED|differ|terminated' | head -8
      fail=1
    else
      echo "PASS ($n tests)  $label"
    fi
  done
done
echo "---"
if [ $fail -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASS"; else echo "SOME COMBINATIONS FAILED"; exit 1; fi
