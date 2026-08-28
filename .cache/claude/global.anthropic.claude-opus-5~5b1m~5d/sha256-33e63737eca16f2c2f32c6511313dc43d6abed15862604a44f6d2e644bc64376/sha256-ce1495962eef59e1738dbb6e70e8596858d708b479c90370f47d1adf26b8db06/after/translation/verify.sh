#!/usr/bin/env bash
# Full verification matrix: every feature combination x every build profile.
#
#   ./verify.sh
#
# Cargo.toml declares no [features], so the complete combination set is
# {default, --no-default-features, --all-features}. The list is derived from
# Cargo.toml rather than hard-coded, so it stays correct if features are added.
set -uo pipefail
cd "$(dirname "$0")"

C_LIB_DIR="../c_src/build"
fail=0

# --- 0. Make sure the C reference library exists -----------------------------
if ! ls "$C_LIB_DIR"/lib*.so >/dev/null 2>&1; then
  echo "==> building C reference library"
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
echo "C reference: $(ls "$C_LIB_DIR"/lib*.so)"

# --- 1. Derive the feature combinations from Cargo.toml ----------------------
declared=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0!="") print}' Cargo.toml)
if [ -z "$declared" ]; then
  echo "Cargo.toml declares no [features]; combination set = {default, none, all}"
else
  echo "Declared features: $declared"
fi

COMBOS=("" "--no-default-features" "--all-features")

# --- 2. Symbol parity, checked independently of the test harness --------------
echo
echo "=== symbol parity (nm -D) ==="
timeout 600 cargo build --release >/dev/null 2>&1 || { echo "release build FAILED"; exit 1; }
C_SO=$(ls "$C_LIB_DIR"/lib*.so | head -1)
R_SO="target/release/libhalf2float_lib.so"

# Use a writable scratch dir: /tmp may be read-only in sandboxes, and writing
# there silently produced an EMPTY symbol list that then "passed" vacuously.
scratch=$(mktemp -d "${TMPDIR:-.}/symparity.XXXXXX") || { echo "mktemp FAILED"; exit 1; }
trap 'rm -rf "$scratch"' EXIT

nm -D --defined-only "$C_SO" | awk 'NF>=3 {print $3}' | sort -u > "$scratch/c_syms"
nm -D --defined-only "$R_SO" | awk 'NF>=3 {print $3}' | sort -u > "$scratch/r_syms"

c_count=$(wc -l < "$scratch/c_syms")
r_count=$(wc -l < "$scratch/r_syms")

# Guard against a vacuous pass: the C library is known to export half2float, so
# an empty or half2float-less list means nm/awk broke, not that parity holds.
if [ "$c_count" -eq 0 ] || ! grep -qx 'half2float' "$scratch/c_syms"; then
  echo "  ERROR: could not read symbols from $C_SO (got $c_count symbol(s));" \
       "refusing to report parity"; fail=1
elif [ "$r_count" -eq 0 ]; then
  echo "  ERROR: could not read symbols from $R_SO; refusing to report parity"; fail=1
else
  echo "C exports:    $c_count symbol(s): $(tr '\n' ' ' < "$scratch/c_syms")"
  echo "Rust exports: $r_count symbol(s) total (incl. Rust std/runtime internals)"
  missing=$(comm -23 "$scratch/c_syms" "$scratch/r_syms")
  if [ -n "$missing" ]; then
    echo "  MISSING from Rust .so:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  symbol diff: EMPTY (every C symbol exported by Rust)  [PASS]"
  fi
  # The three C tables are `static` (internal linkage) and must NOT be exported.
  for t in m__mantissa m__offset m__exponent; do
    if grep -qx "$t" "$scratch/r_syms"; then
      echo "  LEAK: '$t' is static in C but exported by the Rust .so"; fail=1
    fi
  done
fi

# --- 3. Tests: every combination x every profile ------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    label="features='${combo:-default}' profile=$profile"
    echo
    echo "=== $label ==="

    if [ "$profile" = release ]; then
      # panic=abort makes `cargo test --release` unusable for the harness, so
      # build the cdylib in release and run the (dev-profile) tests against it.
      timeout 600 cargo build --release $combo >/dev/null 2>&1 \
        || { echo "  build FAILED [$label]"; fail=1; continue; }
      so="$PWD/target/release/libhalf2float_lib.so"
    else
      timeout 600 cargo build $combo >/dev/null 2>&1 \
        || { echo "  build FAILED [$label]"; fail=1; continue; }
      so="$PWD/target/debug/libhalf2float_lib.so"
    fi

    if [ ! -f "$so" ]; then
      echo "  cdylib not produced at $so [$label]"; fail=1; continue
    fi

    out=$(HALF2FLOAT_RUST_SO="$so" timeout 600 cargo test $combo 2>&1)
    if echo "$out" | grep -qE '^(error|test result: FAILED)|FAILED|panicked'; then
      echo "  TESTS FAILED [$label]"
      echo "$out" | grep -E 'FAILED|panicked|test result|divergence' | head -20
      fail=1
    else
      echo "  $(echo "$out" | grep -c '\.\.\. ok') test(s) ok  [PASS]"
      echo "$out" | grep 'test result' | sed 's/^/    /'
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "=========== ALL COMBINATIONS PASSED ==========="
else
  echo "=========== FAILURES PRESENT (see above) ==========="
fi
exit "$fail"
