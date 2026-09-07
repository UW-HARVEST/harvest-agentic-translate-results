#!/usr/bin/env bash
# Full verification run: build both libraries, then run every phase under every
# feature combination and both cargo profiles.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
fail=0

echo "=== Building the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "C  .so: $C_SO"

# Feature combinations. This crate declares no [features], so the set is just
# the default and the (identical) no-default-features build; the loop is written
# generically so it keeps working if features are ever added.
FEATURES="$(sed -n '/^\[features\]/,/^\[/p' Cargo.toml | grep -oE '^[a-zA-Z0-9_-]+' | grep -v default || true)"
COMBOS=("--offline")
COMBOS+=("--offline --no-default-features")
for f in $FEATURES; do
  COMBOS+=("--offline --no-default-features --features $f")
done
if [ -n "$FEATURES" ]; then
  COMBOS+=("--offline --all-features")
fi

for profile_flag in "--release" ""; do
  pname=${profile_flag:-debug}
  for combo in "${COMBOS[@]}"; do
    echo
    echo "=== cargo test $profile_flag $combo ==="
    # The cdylib under test must exist for the profile being exercised.
    cargo build $profile_flag $combo >/dev/null 2>&1
    if timeout 600 cargo test $profile_flag $combo -- --test-threads=4 2>&1 \
         | grep -E '^(test|running|error|test result)' ; then
      :
    fi
    # shellcheck disable=SC2181
    if ! timeout 600 cargo test $profile_flag $combo -- --test-threads=4 >/dev/null 2>&1; then
      echo "FAILED: $pname / $combo"
      fail=1
    else
      echo "PASSED: $pname / $combo"
    fi
  done
done

echo
echo "=== Symbol diff (must be empty) ==="
RUST_SO="$(ls target/release/libtritanopia_lib.so 2>/dev/null || ls target/debug/libtritanopia_lib.so)"
diff <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort) \
     <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort) \
  && echo "symbol diff EMPTY (ok)" || { echo "SYMBOL DIFF NON-EMPTY"; fail=1; }

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$fail"
