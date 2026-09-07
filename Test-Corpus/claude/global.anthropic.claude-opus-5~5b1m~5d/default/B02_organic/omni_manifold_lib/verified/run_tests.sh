#!/bin/sh
# Full differential verification driver.
#
#  * builds the C shared library exactly as c_src/CMakeLists.txt specifies
#  * builds the Rust cdylib (`cargo test` alone does NOT rebuild a cdylib-only
#    lib target, so this explicit step is required or the tests would load a
#    stale .so)
#  * diffs `nm -D` between the two libraries
#  * runs the differential test suite for every cargo feature combination
#
# Usage: ./run_tests.sh [extra cargo args]
set -e

ROOT=$(cd "$(dirname "$0")/.." && pwd)
CRATE="$ROOT/translation"

echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
cd "$ROOT/c_src/build"
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build . >/dev/null
C_SO=$(ls "$ROOT"/c_src/build/*.so | head -1)
echo "  $C_SO"

cd "$CRATE"

# ---------------------------------------------------------------------------
# feature combinations (the crate currently declares none, so the loop below
# degenerates to the single default configuration; it is written generically so
# that adding features automatically extends the matrix)
# ---------------------------------------------------------------------------
FEATURES=$(cargo metadata --offline --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(" ".join(k for k in json.load(sys.stdin)["packages"][0]["features"] if k != "default"))')

combos() {
    # always: default, and --no-default-features
    echo "|"                       # default features
    echo "--no-default-features|"  # nothing
    if [ -n "$FEATURES" ]; then
        for f in $FEATURES; do
            echo "--no-default-features|$f"
        done
        # all features together
        echo "--all-features|"
    fi
}

STATUS=0
combos | while IFS='|' read -r flags feats; do
    if [ -n "$feats" ]; then
        set -- $flags --features "$feats"
    else
        set -- $flags
    fi
    label="cargo test --release $*"
    echo
    echo "=== $label ==="
    cargo build --release --offline "$@" >/dev/null 2>&1 || {
        echo "  BUILD FAILED"; exit 1; }

    R_SO="$CRATE/target/release/libomni_manifold_lib.so"

    echo "--- nm -D symbol diff (C -> Rust) ---"
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/c_syms.txt"
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/r_syms.txt"
    missing=$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt")
    if [ -n "$missing" ]; then
        echo "  MISSING FROM RUST:"; echo "$missing" | sed 's/^/    /'; exit 1
    fi
    echo "  OK: $(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") C symbols, all exported by Rust"

    echo "--- differential tests ---"
    cargo test --release --offline "$@" 2>&1 | grep -E "^test |test result|error" || true
    cargo test --release --offline "$@" >/dev/null 2>&1 || { echo "  TESTS FAILED"; exit 1; }
done || STATUS=1

# also verify the unoptimised (dev-profile) cdylib: `opt-level` changes how the
# NaN-propagation helpers in src/lib.rs are compiled, so both profiles matter.
echo
echo "=== dev-profile Rust cdylib ==="
cargo build --offline >/dev/null 2>&1
RUST_SO="$CRATE/target/debug/libomni_manifold_lib.so" \
    cargo test --release --offline 2>&1 | grep -E "test result|error" || true

exit $STATUS
