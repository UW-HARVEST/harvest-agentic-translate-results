#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
# Usage: ./verify.sh   (run from the `translation/` directory)
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$HERE")"
C_SO="$ROOT/c_src/build/libdriver.so"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

step "Build C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . ) >/dev/null || { echo "C BUILD FAILED"; exit 1; }
[ -f "$C_SO" ] || { echo "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"="); gsub(/[ \t]/,"",a[1]); if(a[1]!="default") print a[1]}' "$HERE/Cargo.toml")
if [ -z "$FEATURES" ]; then
    echo "Cargo.toml declares no [features] -> single combination (default)"
    COMBOS=("default" "no-default")
else
    echo "features: $FEATURES"
    COMBOS=("default" "no-default")
    for f in $FEATURES; do COMBOS+=("$f"); done
fi

run_combo() {
    local combo="$1" profile="$2" flags=()
    case "$combo" in
        default)    flags=() ;;
        no-default) flags=(--no-default-features) ;;
        *)          flags=(--no-default-features --features "$combo") ;;
    esac
    [ "$profile" = release ] && flags+=(--release)

    step "combo=$combo profile=$profile"
    ( cd "$HERE" && timeout 600 cargo build "${flags[@]}" ) >/dev/null 2>&1 \
        || { echo "  BUILD FAILED"; FAIL=1; return; }

    local rust_so="$HERE/target/$profile/libdriver.so"
    [ -f "$rust_so" ] || { echo "  MISSING $rust_so"; FAIL=1; return; }

    # --- symbol parity -----------------------------------------------------
    local cdefs rdefs diff
    cdefs=$(nm -D --defined-only "$C_SO"    | awk '{print $3}' | grep -E '^(UTIL_|[A-Za-z])' \
              | grep -vE '^(_init|_fini|__|_edata|_end|_IO_stdin_used)' | sort -u)
    rdefs=$(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u)
    diff=$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))
    if [ -n "$diff" ]; then
        echo "  SYMBOL DIFF NOT EMPTY (missing from Rust):"; echo "$diff" | sed 's/^/    /'
        FAIL=1
    else
        echo "  symbol diff: EMPTY ($(echo "$cdefs" | wc -l) C symbol(s) all exported by Rust)"
    fi

    # --- undefined non-libc symbols in the Rust .so ------------------------
    # Anything versioned @GLIBC_*/@GCC_* is libc/libgcc. The three weak
    # toolchain hooks below are emitted by the linker into every .so (they are
    # present in the C .so as well) and are not real dependencies.
    local undef
    undef=$(nm -D --undefined-only "$rust_so" | awk '{print $NF}' \
              | grep -vE '@(GLIBC|GCC)_' \
              | grep -vE '^(_ITM_(de)?registerTMCloneTable|__gmon_start__|__cxa_finalize)$' \
              | sort -u)
    if [ -n "$undef" ]; then
        echo "  UNDEFINED NON-LIBC SYMBOLS:"; echo "$undef" | sed 's/^/    /'; FAIL=1
    else
        echo "  undefined non-libc symbols: none"
    fi

    # --- differential tests against THIS .so -------------------------------
    ( cd "$HERE" && RUST_SO_PATH="$rust_so" C_SO_PATH="$C_SO" \
        timeout 600 cargo test "${flags[@]}" 2>&1 ) > /tmp/verify_$$.log
    if grep -qE '^test result: FAILED|error\[|panicked' /tmp/verify_$$.log; then
        echo "  TESTS FAILED"; tail -40 /tmp/verify_$$.log | sed 's/^/    /'; FAIL=1
    else
        grep -E '^test result:' /tmp/verify_$$.log | sed 's/^/  /'
    fi
    rm -f /tmp/verify_$$.log
}

for combo in "${COMBOS[@]}"; do
    for profile in debug release; do
        run_combo "$combo" "$profile"
    done
done

# ---------------------------------------------------------------------------
step "Binary executable check"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
    echo "  C builds an executable -> stdout comparison REQUIRED"; FAIL=1
else
    echo "  c_src/CMakeLists.txt has no add_executable; translation/Cargo.toml has no [[bin]]"
    echo "  -> no driver binary, stdout comparison N/A"
fi

step "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
