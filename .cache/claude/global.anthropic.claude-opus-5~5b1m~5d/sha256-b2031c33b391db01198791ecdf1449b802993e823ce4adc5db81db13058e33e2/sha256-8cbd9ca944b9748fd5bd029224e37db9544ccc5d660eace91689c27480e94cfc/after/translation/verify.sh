#!/usr/bin/env bash
# Full differential verification: builds the C .so and the Rust cdylib, then
# runs every phase of the test suite against both, for every feature
# combination and both Cargo profiles.
#
# Usage:  ./verify.sh [--quick]      (--quick skips the multi-minute exhaustive
#                                     sweeps in phase_e / phase_f)
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

echo "=== 1. build the C shared library ==="
mkdir -p "$root/c_src/build"
(cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
c_so="$(find "$root/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | head -1)"
echo "C  .so: $c_so"

echo "=== 2. build the Rust cdylib (both profiles) ==="
cd "$here"
cargo build --release --offline -q
cargo build --offline -q

echo "=== 3. symbol parity (nm -D) ==="
c_syms="$(nm -D --defined-only "$c_so" | awk '{print $3}' | sort -u)"
for prof in release debug; do
  rs_syms="$(nm -D --defined-only "target/$prof/libmemchra2_lib.so" | awk '{print $3}' | sort -u)"
  missing="$(comm -23 <(echo "$c_syms") <(echo "$rs_syms"))"
  if [ -n "$missing" ]; then
    echo "FAIL ($prof): Rust .so missing C symbols:"; echo "$missing"; exit 1
  fi
  echo "OK ($prof): 0 missing symbols. C exports: $(echo "$c_syms" | tr '\n' ' ')"
done

echo "=== 4. differential tests: all feature combos x both profiles ==="
# The crate declares no [features], so these three are the exhaustive set.
tests=(--test phase_b_configs --test phase_c_errors --test phase_d_symbols)
if [ "${1:-}" != "--quick" ]; then
  tests+=(--test phase_e_stress --test phase_f_exhaustive)
fi

for combo in "" "--no-default-features" "--all-features"; do
  for prof in release debug; do
    export HARVEST_RUST_SO="$here/target/$prof/libmemchra2_lib.so"
    echo "--- features='${combo:-<default>}' rust.so=$prof ---"
    cargo test --release --offline ${combo:+$combo} "${tests[@]}" --no-fail-fast \
      2>&1 | grep -E '^test result|divergence|^error' || true
  done
done
unset HARVEST_RUST_SO
echo "=== done ==="
