#!/usr/bin/env bash
# Phase A + D driver: enumerate every build-time configuration, and for each one
#   1. cargo check          (must compile clean)
#   2. cargo build          (produce the Rust cdylib)
#   3. nm -D symbol diff    (C .so vs Rust .so -- must be empty)
#   4. cargo test           (Phase B + Phase C differential suites)
#
# `Cargo.toml` declares no [features] table and `c_src/CMakeLists.txt` declares
# no options / #ifdefs, so the feature power set is a single (empty) element;
# all three cargo spellings below therefore resolve to the same configuration
# and each is verified explicitly.  The dev and release profiles are both
# exercised because `[profile.release]` sets `panic = "abort"`.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT=$PWD
FAIL=0

echo "=== Cargo features declared ==="
if grep -qE '^\[features\]' Cargo.toml; then
    sed -n '/^\[features\]/,/^\[/p' Cargo.toml
else
    echo "(none -- the feature power set is the single empty set)"
fi
echo
echo "=== CMake build-time options declared ==="
if grep -nE 'option\(|add_definitions|target_compile_definitions' c_src/CMakeLists.txt; then :; else
    echo "(none)"
fi
echo "--- preprocessor conditionals in the C sources (build/ excluded) ---"
grep -rnE '^[[:space:]]*#[[:space:]]*(if|ifdef|ifndef|elif|else)' c_src/include c_src/src \
    || echo "(none)"
echo

# --- build the C reference library once -------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "FATAL: C build failed"; exit 1; }
C_SO=$ROOT/c_src/build/libdriver.so
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/c_syms.txt"
echo "C .so exports $(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") symbols:"
tr '\n' ' ' < "${TMPDIR:-/tmp}/c_syms.txt"; echo; echo

run_combo() {
    local label="$1"; shift
    local profile_dir="$1"; shift
    echo "############################################################"
    echo "# CONFIG: $label"
    echo "############################################################"

    echo "--- cargo check ---"
    cargo check --offline "$@" 2>&1 | tail -3 || FAIL=1
    if ! cargo check --offline "$@" >/dev/null 2>&1; then
        echo "FAILED: cargo check $*"; FAIL=1
    fi

    echo "--- cargo build ---"
    cargo build --offline "$@" 2>&1 | tail -2
    local rs_so="$ROOT/target/$profile_dir/libdriver.so"
    if [ ! -f "$rs_so" ]; then echo "FAILED: no $rs_so"; FAIL=1; return; fi

    echo "--- nm -D symbol diff (C \\ Rust must be empty) ---"
    nm -D --defined-only "$rs_so" | awk '{print $3}' | sort > "${TMPDIR:-/tmp}/rs_syms.txt"
    local missing
    missing=$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/rs_syms.txt")
    if [ -n "$missing" ]; then
        echo "FAILED: missing from Rust .so:"; echo "$missing"; FAIL=1
    else
        echo "OK: 0 missing symbols ($(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") matched)"
    fi
    echo "--- undefined non-libc/non-unwinder symbols in Rust .so ---"
    local dangling
    dangling=$(nm -D --undefined-only "$rs_so" | awk '{print $NF}' \
        | grep -vE 'GLIBC|GCC_|^_ITM_|^__gmon_start__|^__cxa_|^gettid|^statx' || true)
    if [ -n "$dangling" ]; then echo "FAILED: $dangling"; FAIL=1; else echo "OK: none"; fi

    echo "--- cargo test (Phase B + Phase C) ---"
    cargo test --offline "$@" 2>&1 | grep -E 'test result|DIVERGENCE|FAILED|error\[|panicked'
    if ! cargo test --offline "$@" >/dev/null 2>&1; then
        echo "FAILED: cargo test $*"; FAIL=1
    fi
    echo
}

run_combo "default features (dev profile)"          debug
run_combo "--no-default-features (dev profile)"     debug   --no-default-features
run_combo "--all-features (dev profile)"            debug   --all-features
run_combo "default features (release profile)"      release --release
run_combo "--no-default-features (release profile)" release --release --no-default-features
run_combo "--all-features (release profile)"        release --release --all-features

echo "############################################################"
if [ "$FAIL" -eq 0 ]; then
    echo "# ALL CONFIGURATIONS PASSED"
else
    echo "# FAILURES DETECTED"
fi
echo "############################################################"
exit "$FAIL"
