#!/usr/bin/env bash
# Full verification run: builds both C configurations and both Rust feature
# combinations, then runs every phase against the matching pair.
#
#   c_src/build       - built exactly as the task specifies (no NDEBUG, so the C
#                       `assert()`s are live)      <-> Rust default features
#   c_ndebug_build    - same sources, -DNDEBUG     <-> Rust --no-default-features
#
# Usage: ./run_all.sh [extra cargo test args...]
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
TRANS="$PWD"
FAIL=0

step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }

# --------------------------------------------------------------------------
# C builds
# --------------------------------------------------------------------------
step "Building C .so (asserts live, as specified)"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build failed"; exit 1; }
C_ASSERT_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)

step "Building C .so (-DNDEBUG, asserts removed)"
cmake -S "$ROOT/c_src" -B "$ROOT/c_ndebug_build" \
      -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DCMAKE_C_FLAGS="-DNDEBUG" >/dev/null \
  && cmake --build "$ROOT/c_ndebug_build" >/dev/null \
  || { echo "C NDEBUG build failed"; exit 1; }
C_NDEBUG_SO=$(ls "$ROOT"/c_ndebug_build/*.so | head -1)

echo "  asserts : $C_ASSERT_SO"
echo "  ndebug  : $C_NDEBUG_SO"

# --------------------------------------------------------------------------
# Feature combinations. The crate has exactly one optional feature
# (`c_asserts`, on by default), so the full combination set is:
#     default (c_asserts) | no-default-features
# --------------------------------------------------------------------------
COMBOS=( "default:${C_ASSERT_SO}:" "no-default-features:${C_NDEBUG_SO}:--no-default-features" )

for combo in "${COMBOS[@]}"; do
    name="${combo%%:*}"
    rest="${combo#*:}"
    cso="${rest%%:*}"
    flags="${rest#*:}"

    step "cargo check  [$name]"
    timeout 300 cargo check --release $flags 2>&1 | tail -3 || FAIL=1

    step "cargo build --release  [$name]"
    timeout 300 cargo build --release $flags 2>&1 | tail -3 || FAIL=1

    step "nm -D symbol diff  [$name]"
    diff <(nm -D --defined-only "$cso" | awk '{print $NF}' | sort) \
         <(nm -D --defined-only "$TRANS/target/release/libconvert_pix_lib.so" \
              | awk '{print $NF}' | sort | grep -v '^_') \
      && echo "  symbol diff empty" || { echo "  SYMBOL DIFF NOT EMPTY"; FAIL=1; }

    for t in phase_d_symbols smoke phase_b_valid phase_b_tables phase_b_zlib \
             phase_c_errors phase_c_subproc; do
        step "cargo test --test $t  [$name]"
        CP_C_SO="$cso" timeout 600 cargo test --release $flags --test "$t" "$@" 2>&1 \
            | tail -6
        # shellcheck disable=SC2181
        if [ "${PIPESTATUS[0]}" -ne 0 ]; then FAIL=1; echo "  FAILED: $t [$name]"; fi
    done
done

# leave the default build in place
step "restoring default build"
cargo build --release >/dev/null 2>&1

printf '\n'
if [ "$FAIL" -eq 0 ]; then
    echo "ALL PHASES PASSED FOR ALL FEATURE COMBINATIONS"
else
    echo "FAILURES PRESENT (see above)"
fi
exit "$FAIL"
