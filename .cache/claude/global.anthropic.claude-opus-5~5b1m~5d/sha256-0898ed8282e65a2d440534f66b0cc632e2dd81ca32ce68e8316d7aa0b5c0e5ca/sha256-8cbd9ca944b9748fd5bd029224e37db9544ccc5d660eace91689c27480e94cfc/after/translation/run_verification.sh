#!/usr/bin/env bash
# Phase D driver: run the full differential test suite against every
# combination of (cargo feature set) x (build profile of the Rust cdylib).
#
# Usage: ./run_verification.sh
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
FAILED=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
say "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
export C_DRIVER_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_DRIVER_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate the feature combinations declared in Cargo.toml.
#    (The crate declares no [features] table, so the only combination is the
#    default/empty one; the loop is written generically so that adding features
#    later is automatically covered.)
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      if ($0 != "default") print
  }' "$HERE/Cargo.toml")

COMBOS=()
COMBOS+=("--offline")                        # default features
COMBOS+=("--offline --no-default-features")  # no default features
if [ -n "$FEATURES" ]; then
    for f in $FEATURES; do
        COMBOS+=("--offline --no-default-features --features $f")
    done
    ALL=$(echo "$FEATURES" | paste -sd, -)
    COMBOS+=("--offline --no-default-features --features $ALL")
    COMBOS+=("--offline --all-features")
fi
say "Feature combinations to verify (${#COMBOS[@]})"
printf '  cargo test %s\n' "${COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. For each feature combo, build the cdylib in BOTH profiles and run the
#    whole suite against each resulting .so.
#
#    `cargo test` does not build the cdylib itself, so it must be built
#    explicitly and pointed at via RUST_DRIVER_SO.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
    # shellcheck disable=SC2086
    for profile in debug release; do
        if [ "$profile" = release ]; then PROF_FLAG=--release; else PROF_FLAG=; fi

        say "features: [${combo}] | rust cdylib profile: ${profile}"

        ( cd "$HERE" && cargo build $combo $PROF_FLAG ) || {
            echo "!! cargo build failed for [$combo] $profile"; FAILED=1; continue; }

        SO="$HERE/target/$profile/libdriver.so"
        if [ ! -f "$SO" ]; then
            echo "!! expected cdylib missing: $SO"; FAILED=1; continue
        fi
        export RUST_DRIVER_SO="$SO"

        ( cd "$HERE" && cargo test $combo -- --test-threads=1 ) || {
            echo "!! TESTS FAILED for [$combo] profile=$profile"; FAILED=1; }
    done
done

# ---------------------------------------------------------------------------
# 4. Raw symbol diff, independent of the test suite.
# ---------------------------------------------------------------------------
say "Symbol diff (C .so vs Rust .so)"
for profile in debug release; do
    SO="$HERE/target/$profile/libdriver.so"
    [ -f "$SO" ] || continue
    D=$(diff <(nm -D --defined-only "$C_DRIVER_SO" | awk '{print $NF}' | sort -u) \
             <(nm -D --defined-only "$SO" | awk '$2 ~ /^[TDBRGS]$/ {print $NF}' | sort -u) \
         | grep '^<' || true)
    if [ -n "$D" ]; then
        echo "!! $profile: symbols exported by C but MISSING from Rust:"; echo "$D"; FAILED=1
    else
        echo "OK  $profile: every C-exported symbol is exported by the Rust .so"
    fi
done

say "RESULT"
if [ "$FAILED" -eq 0 ]; then
    echo "ALL PHASES PASSED"
else
    echo "FAILURES DETECTED"
fi
exit "$FAILED"
