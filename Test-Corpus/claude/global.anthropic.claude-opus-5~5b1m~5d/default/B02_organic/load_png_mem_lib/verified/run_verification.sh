#!/usr/bin/env bash
# Full verification run: builds the C reference library and the Rust cdylib,
# diffs their exported symbols, and runs every differential test under every
# feature combination.
#
#   ./run_verification.sh
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
fail=0

echo "=== 1. build the C reference library ==="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so="$(ls "$root"/c_src/build/*.so)"
echo "    $c_so"

echo
echo "=== 2. enumerate feature combinations ==="
# Every combination of the features declared in Cargo.toml, plus
# --no-default-features. The crate declares no [features], so this is
# {default, no-default-features}.
features="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,"");print}' "$here/Cargo.toml")"
combos=("" "--no-default-features")
if [ -n "$features" ]; then
  # power set of the declared features
  feats=($features)
  n=${#feats[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    list=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then list="$list,${feats[$i]}"; fi
    done
    combos+=("--no-default-features --features ${list#,}")
  done
fi
printf '    %s\n' "${combos[@]/#/[default] }" | sed 's/\[default\] $/[default]/'

for combo in "${combos[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=== 3. cargo build --release $combo ==="
  # shellcheck disable=SC2086
  cargo build --offline --release $combo || { echo "build FAILED for $label"; fail=1; continue; }
  rust_so="$here/target/release/libload_png_mem_lib.so"

  echo "--- symbol diff (C vs Rust) for $label ---"
  nm -D --defined-only "$c_so"    | awk '{print $3}' | sort > "$here/target/c.syms"
  nm -D --defined-only "$rust_so" | awk '{print $3}' | sort > "$here/target/rust.syms"
  missing="$(comm -23 "$here/target/c.syms" "$here/target/rust.syms")"
  if [ -n "$missing" ]; then
    echo "MISSING FROM RUST:"; echo "$missing"; fail=1
  else
    echo "    OK: all $(wc -l < "$here/target/c.syms") C symbols are exported by Rust"
  fi
  echo "--- undefined non-libc symbols in the Rust .so ---"
  nm -D --undefined-only "$rust_so" | awk '{print $2}' \
    | grep -v -E '^(malloc|calloc|free|memcpy|memset|memcmp|memmove|abort|__.*|_ITM_.*|_Unwind_.*)$' \
    > "$here/target/rust.undef" || true
  if [ -s "$here/target/rust.undef" ]; then
    echo "UNEXPECTED:"; cat "$here/target/rust.undef"; fail=1
  else
    echo "    OK: none"
  fi

  # Each target is run separately with its own timeout: the error-path and fuzz
  # targets deliberately abort thousands of child processes, which is slow.
  for t in smoke phase_b_inflate phase_b_png phase_c_errors fuzz_inflate fuzz_inflate_tables fuzz_png; do
    echo "--- cargo test --release $combo --test $t ---"
    # shellcheck disable=SC2086
    timeout 600 cargo test --offline --release $combo --test "$t" -- --test-threads=1 \
      || { echo "tests FAILED: $t ($label)"; fail=1; }
  done
  echo "--- cargo test --release $combo --lib (layout unit tests) ---"
  # shellcheck disable=SC2086
  timeout 600 cargo test --offline --release $combo --lib \
    || { echo "unit tests FAILED ($label)"; fail=1; }
done

echo
if [ "$fail" = 0 ]; then
  echo "=== ALL CHECKS PASSED ==="
else
  echo "=== FAILURES PRESENT ==="
fi
exit "$fail"
