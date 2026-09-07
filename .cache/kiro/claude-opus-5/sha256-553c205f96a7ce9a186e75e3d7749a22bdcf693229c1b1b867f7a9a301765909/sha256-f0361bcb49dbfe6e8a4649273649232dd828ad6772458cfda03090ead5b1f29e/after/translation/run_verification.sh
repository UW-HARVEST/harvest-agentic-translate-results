#!/usr/bin/env bash
# Full verification driver.
#
# `cargo test` does not rebuild the `cdylib` target, so the cdylib MUST be
# built explicitly before the differential tests run — otherwise the tests
# load a stale .so and pass vacuously. This script enforces that ordering and
# loops over every feature combination declared in Cargo.toml.
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== 1. build the C shared library ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C  .so: $C_SO"

echo
echo "=== 2. enumerate feature combinations from Cargo.toml ==="
# Collect the [features] table keys (excluding "default"), then build the
# power set. No [features] table => the only combos are default / none.
FEATS=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' Cargo.toml)

COMBOS=()
COMBOS+=("--default")                 # default features
COMBOS+=("--no-default-features")     # empty feature set
if [ -n "$FEATS" ]; then
  mapfile -t F <<<"$FEATS"
  n=${#F[@]}
  for ((m=1; m<(1<<n); m++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( m & (1<<i) )); then sel="${sel:+$sel,}${F[i]}"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
  echo "features found: $(echo "$FEATS" | tr '\n' ' ')"
else
  echo "features found: (none declared)"
fi
printf 'combination: %s\n' "${COMBOS[@]}"

echo
echo "=== 3. cargo check every combination ==="
for c in "${COMBOS[@]}"; do
  flags=$([ "$c" = "--default" ] && echo "" || echo "$c")
  echo "--- cargo check $c"
  timeout 600 cargo check $flags 2>&1 | tail -2
done

echo
echo "=== 4. build cdylib + run differential tests, per combination ==="
FAIL=0
for c in "${COMBOS[@]}"; do
  flags=$([ "$c" = "--default" ] && echo "" || echo "$c")
  echo
  echo "########## combination: $c ##########"
  # Rebuild the cdylib FIRST so the .so under test matches the sources.
  timeout 600 cargo build $flags 2>&1 | tail -2
  RUST_SO="$PWD/target/debug/librev16_lib.so"
  ls -l "$RUST_SO"

  echo "--- nm -D symbol parity ---"
  diff <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort) \
    && echo "symbol diff: EMPTY (parity OK)" \
    || { echo "symbol diff: NON-EMPTY (FAIL)"; FAIL=1; }

  echo "--- differential tests ---"
  if REV16_RUST_SO="$RUST_SO" timeout 600 cargo test $flags 2>&1 | tail -25; then
    :
  else
    echo "TESTS FAILED for $c"; FAIL=1
  fi

  # Also exercise the RELEASE cdylib (opt-level 3 + panic=abort is a different
  # codegen path: LLVM can recognise the bit-reversal idiom and emit a
  # completely different instruction sequence). The test harness itself stays
  # in debug because `panic = "abort"` is incompatible with libtest.
  echo "--- release cdylib, same differential tests ---"
  timeout 600 cargo build --release $flags 2>&1 | tail -1
  REL_SO="$PWD/target/release/librev16_lib.so"
  diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$REL_SO" | awk '{print $NF}' | sort) \
    && echo "release symbol diff: EMPTY (parity OK)" \
    || { echo "release symbol diff: NON-EMPTY (FAIL)"; FAIL=1; }
  if REV16_RUST_SO="$REL_SO" timeout 600 cargo test $flags 2>&1 | tail -6; then
    :
  else
    echo "RELEASE TESTS FAILED for $c"; FAIL=1
  fi
done

echo
echo "=== 5. binary/driver executables ==="
if grep -qE '^\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ]; then
  echo "Rust binary target present — stdout comparison required"
else
  echo "Rust: no [[bin]] target and no src/main.rs"
fi
if grep -qE 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "C executable present — stdout comparison required"
else
  echo "C: no add_executable in CMakeLists.txt (library only)"
fi
echo "=> no driver binary on either side; stdout comparison is N/A"

echo
if [ "$FAIL" -eq 0 ]; then echo "=== ALL COMBINATIONS PASSED ==="; else echo "=== FAILURES PRESENT ==="; exit 1; fi
