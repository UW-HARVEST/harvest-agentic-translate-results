#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run the differential test suite
# under every feature combination.
#
# Usage: translation/run_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
# Cargo cannot reach crates.io in this environment; the deps are vendored in
# the local cargo cache.
CARGO_FLAGS="--offline"
FAIL=0

echo "=== 1/5 build C shared library ==============================="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C BUILD FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

echo
echo "=== 2/5 build Rust cdylib (debug + release) =================="
( cd "$HERE" && cargo build $CARGO_FLAGS ) || { echo "RUST DEBUG BUILD FAILED"; exit 1; }
( cd "$HERE" && cargo build $CARGO_FLAGS --release ) || { echo "RUST RELEASE BUILD FAILED"; exit 1; }
ls -l "$HERE/target/debug/libdriver.so" "$HERE/target/release/libdriver.so"

echo
echo "=== 3/5 symbol parity (nm -D) ================================"
C_SYMS=$(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u)
R_SYMS=$(nm -D --defined-only "$HERE/target/release/libdriver.so" | awk '{print $NF}' | sort -u)
echo "C exports:"; echo "$C_SYMS" | sed 's/^/  /'
echo "Rust exports:"; echo "$R_SYMS" | sed 's/^/  /'
MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$R_SYMS"))
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST .so:"; echo "$MISSING" | sed 's/^/  /'
  FAIL=1
else
  echo "symbol diff is EMPTY -- parity OK"
fi
echo "Rust non-libc unresolved deps:"
ldd "$HERE/target/release/libdriver.so" | sed 's/^/  /'

echo
echo "=== 4/5 feature combinations ================================="
# Enumerate declared features from Cargo.toml (excluding "default").
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]);if(a[1]!="default")print a[1]}' "$HERE/Cargo.toml")
if [ -z "$FEATS" ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("default" "no-default-features" "all-features")
else
  echo "declared features: $FEATS"
  COMBOS=("default" "no-default-features" "all-features")
  for f in $FEATS; do COMBOS+=("feat:$f"); done
fi

echo
echo "=== 5/5 differential tests per combination ==================="
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default)             ARGS="" ;;
    no-default-features) ARGS="--no-default-features" ;;
    all-features)        ARGS="--all-features" ;;
    feat:*)              ARGS="--no-default-features --features ${combo#feat:}" ;;
  esac
  # Run the suite against BOTH built profiles: optimisation level changes
  # codegen in ABI-observable ways (e.g. whether the internal
  # driver -> printHexCharLine call survives as an interposable symbol
  # reference), so a green debug run does not imply a green release run.
  for prof in debug release; do
    echo
    echo "--- combo: $combo | rust .so profile: $prof ---"
    # fd 1 is redirected inside the harness, so tests must not run concurrently.
    ( cd "$HERE" \
      && RUST_DRIVER_SO="$HERE/target/$prof/libdriver.so" \
         timeout 600 cargo test $CARGO_FLAGS $ARGS -- --test-threads=1 ) \
      || { echo "TESTS FAILED for combo: $combo (profile $prof)"; FAIL=1; }
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "############ ALL PHASES PASSED ############"
else
  echo "############ FAILURES PRESENT ############"
fi
exit $FAIL
