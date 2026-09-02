#!/usr/bin/env bash
# Full verification driver.
#
#   1. rebuild the C .so
#   2. enumerate every cargo feature combination from Cargo.toml and `cargo
#      check` each one
#   3. run the whole differential suite against each feature combination, and
#      against BOTH the dev-profile and release-profile Rust cdylib
#   4. print the nm -D symbol diff (must be empty)
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 1
fail=0

echo "== 1. build C shared library =="
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
echo "   $C_SO"

cd translation || exit 1

# Sum the "N passed" figures across all test binaries in one `cargo test` run.
count_passed() {
  grep -oE 'test result: ok\. [0-9]+ passed' "$1" \
    | grep -oE '[0-9]+' | awk '{t+=$1} END{print t+0}'
}

echo
echo "== 2. enumerate feature combinations from Cargo.toml =="
# Every feature name declared under [features] (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0,a,"="); gsub(/[[:space:]]/,"",a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=("default")
COMBOS+=("--no-default-features")
if [ -n "$FEATURES" ]; then
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  # all features together
  COMBOS+=("--all-features")
fi
printf '   declared features: %s\n' "${FEATURES:-<none>}"
printf '   combos: %s\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "     - $c"; done

echo
echo "== 3. cargo check per combination =="
for c in "${COMBOS[@]}"; do
  flags=""; [ "$c" != "default" ] && flags="$c"
  if timeout 600 cargo check -q $flags 2>/dev/null; then
    echo "   ok    cargo check $c"
  else
    echo "   FAIL  cargo check $c"; fail=1
  fi
done

echo
# Expected number of test cases; a run that silently executes fewer (or none)
# must FAIL rather than look green.
EXPECTED_CASES=$(grep -rh '^#\[test\]' tests/*.rs | awk 'END{print NR+0}')
echo "== 4. differential suite per combination x profile ($EXPECTED_CASES cases expected) =="
for c in "${COMBOS[@]}"; do
  flags=""; [ "$c" != "default" ] && flags="$c"

  # --- dev profile cdylib ---
  timeout 600 cargo build -q $flags 2>/dev/null || { echo "   FAIL  build dev $c"; fail=1; continue; }
  if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$ROOT/translation/target/debug/libdriver.so" \
     timeout 600 cargo test $flags -- --test-threads=1 >/tmp/vt.$$ 2>&1; then
    n=$(count_passed /tmp/vt.$$)
    if [ "$n" -ge "$EXPECTED_CASES" ]; then
      echo "   ok    tests [dev]     $c   ($n/$EXPECTED_CASES cases passed)"
    else
      echo "   FAIL  tests [dev]     $c   only $n/$EXPECTED_CASES cases ran"; fail=1
    fi
  else
    echo "   FAIL  tests [dev]     $c"; grep -E 'FAILED|panicked|DIVERGENCE' /tmp/vt.$$ | head -20; fail=1
  fi

  # --- release profile cdylib (different codegen: inlining, panic=abort) ---
  timeout 600 cargo build -q --release $flags 2>/dev/null || { echo "   FAIL  build release $c"; fail=1; continue; }
  if DRIVER_C_SO="$C_SO" DRIVER_RUST_SO="$ROOT/translation/target/release/libdriver.so" \
     timeout 600 cargo test $flags -- --test-threads=1 >/tmp/vt.$$ 2>&1; then
    n=$(count_passed /tmp/vt.$$)
    if [ "$n" -ge "$EXPECTED_CASES" ]; then
      echo "   ok    tests [release] $c   ($n/$EXPECTED_CASES cases passed)"
    else
      echo "   FAIL  tests [release] $c   only $n/$EXPECTED_CASES cases ran"; fail=1
    fi
  else
    echo "   FAIL  tests [release] $c"; grep -E 'FAILED|panicked|DIVERGENCE' /tmp/vt.$$ | head -20; fail=1
  fi
done
rm -f /tmp/vt.$$

echo
echo "== 5. nm -D symbol diff (C -> Rust); must be empty =="
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > /tmp/c.syms.$$
for prof in debug release; do
  nm -D --defined-only "target/$prof/libdriver.so" | awk '{print $NF}' | sort -u > /tmp/r.syms.$$
  miss=$(comm -23 /tmp/c.syms.$$ /tmp/r.syms.$$)
  if [ -z "$miss" ]; then
    echo "   ok    $prof: 0 C symbols missing from Rust ($(wc -l < /tmp/c.syms.$$) checked)"
  else
    echo "   FAIL  $prof missing: $miss"; fail=1
  fi
done
rm -f /tmp/c.syms.$$ /tmp/r.syms.$$

echo
if [ "$fail" -eq 0 ]; then echo "ALL VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit $fail
