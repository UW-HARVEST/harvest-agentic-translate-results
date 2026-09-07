#!/usr/bin/env bash
# Full verification run: builds both libraries and runs the differential suite
# under every Cargo feature combination and both Rust optimisation profiles.
set -euo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== 1. build the C shared library (exactly as CMakeLists specifies) ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "    C   .so: $C_SO"

echo
echo "=== 2. feature combinations ==="
# Cargo.toml declares no [features] table, so the only combinations are the
# default one and --no-default-features (identical, but both are exercised).
COMBOS=("" "--no-default-features")
if grep -q '^\[features\]' Cargo.toml; then
  echo "!! a [features] table appeared in Cargo.toml - extend COMBOS" >&2
  exit 1
fi

echo
echo "=== 3. run the suite: {release, debug} x feature combos ==="
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    flag=""
    [ "$profile" = release ] && flag="--release"
    label="profile=$profile features='${combo:-default}'"
    echo "--- $label ---"
    cargo build $flag --offline $combo >/dev/null 2>&1
    so="target/$profile/libcolourblind_lib.so"
    [ -f "$so" ] || { echo "missing $so"; exit 1; }
    RUST_SO="$(pwd)/$so" C_SO="$C_SO" \
      cargo test $flag --offline $combo 2>&1 \
      | grep -E 'test result:|FAILED|panicked' || true
  done
done

echo
echo "=== 4. symbol parity (nm -D) ==="
cargo build --release --offline >/dev/null 2>&1
diff <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRGS]$/ {print $3}' | sort) \
     <(nm -D --defined-only target/release/libcolourblind_lib.so | awk '$2 ~ /^[TDBRGS]$/ {print $3}' | sort) \
  && echo "    symbol diff EMPTY (C-exported symbols all present in Rust)"

echo
echo "=== 5. binary driver ==="
echo "    none: the project defines no [[bin]] target and c_src has no main() -"
echo "    stdout comparison is N/A."

echo
echo "=== 6. INFORMATIONAL: same C source at other optimisation levels ==="
echo "    (CMakeLists sets no CMAKE_BUILD_TYPE, so the canonical build is -O0;"
echo "     gcc reorders NaN operands at -O1+, so those builds disagree with"
echo "     each other on multi-NaN inputs. See CONFIGS.md.)"
mkdir -p .mutants/copt
for lvl in O0 O1 O2 O3 Os; do
  gcc -shared -fPIC "-$lvl" -I"$ROOT/c_src/include" \
      -o ".mutants/copt/lib_$lvl.so" "$ROOT/c_src/src/lib.c"
  n=$(C_SO="$(pwd)/.mutants/copt/lib_$lvl.so" cargo test --release --offline 2>&1 \
      | grep -c '\.\.\. FAILED' || true)
  printf '    gcc -%-3s -> %s failing test(s)\n' "$lvl" "$n"
done

echo
echo "=== 7. mutation sensitivity of the suite ==="
./mutation_check.sh | tail -12
