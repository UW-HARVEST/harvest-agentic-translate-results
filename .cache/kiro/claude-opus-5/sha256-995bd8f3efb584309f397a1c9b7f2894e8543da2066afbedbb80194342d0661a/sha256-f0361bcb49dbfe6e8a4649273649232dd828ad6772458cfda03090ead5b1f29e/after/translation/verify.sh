#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared libraries, diffs their
# exported symbols, and runs every differential test under every Cargo feature
# combination. Exits non-zero on any failure.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAIL=1; }

step "Build the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || fail "C build"
C_SO=$(ls "$ROOT"/c_src/build/*.so)
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations straight out of Cargo.toml instead of
# hard-coding them.
# ---------------------------------------------------------------------------
step "Enumerate Cargo feature combinations"
FEATURES=$(cd "$CRATE" && cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c 'import json,sys; print(" ".join(k for k in json.load(sys.stdin)["packages"][0]["features"] if k != "default"))')
echo "non-default features: [${FEATURES:-<none>}]"

COMBOS=()
if [ -z "${FEATURES// /}" ]; then
    # No [features] table: the default build is the only configuration, but run
    # --no-default-features and --all-features too so the claim is mechanical.
    COMBOS+=("" "--no-default-features" "--all-features")
else
    COMBOS+=("" "--no-default-features" "--all-features")
    for f in $FEATURES; do
        COMBOS+=("--no-default-features --features $f")
        COMBOS+=("--features $f")
    done
fi

for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    step "Configuration: $label"

    # shellcheck disable=SC2086
    ( cd "$CRATE" && timeout 600 cargo build --release $combo >/dev/null 2>&1 ) \
        || { fail "cargo build $label"; continue; }

    R_SO="$CRATE/target/release/libomni_manifold_lib.so"

    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > /tmp/c_syms.txt
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > /tmp/r_syms.txt
    missing=$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)
    extra=$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)
    printf 'symbols: C=%s Rust=%s\n' "$(wc -l < /tmp/c_syms.txt)" "$(wc -l < /tmp/r_syms.txt)"
    if [ -n "$missing" ]; then
        fail "$label: symbols exported by C but MISSING from Rust:"; echo "$missing"
    fi
    if [ -n "$extra" ]; then
        printf 'note: %s: extra Rust exports (allowed): %s\n' "$label" "$(echo "$extra" | tr '\n' ' ')"
    fi

    # Undefined symbols in the Rust .so must all be libc / runtime imports.
    stray=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
        | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__errno_location)' \
        | sed 's/@.*//' \
        | grep -vE '^(malloc|free|calloc|realloc|posix_memalign|memcpy|memmove|memset|bcmp|strlen|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|dl_iterate_phdr|gettid|syscall|pthread_key_create|pthread_key_delete|pthread_setspecific|sqrtf)$' \
        || true)
    if [ -n "$stray" ]; then
        fail "$label: unresolved non-libc symbols in the Rust .so:"; echo "$stray"
    fi

    # shellcheck disable=SC2086
    ( cd "$CRATE" && timeout 600 cargo test --release $combo -- --test-threads=1 ) \
        || fail "cargo test $label"
done

step "Result"
if [ "$FAIL" -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "FAILURES PRESENT"
fi

# ---------------------------------------------------------------------------
# Optional independent randomized sweep: SWEEP=<n> re-runs the whole suite with
# n different global seeds (DIFF_SEED), so the fixed seeds cannot hide a
# divergence by luck.
# ---------------------------------------------------------------------------
if [ "${SWEEP:-0}" -gt 0 ] && [ "$FAIL" -eq 0 ]; then
    step "Independent seed sweep (${SWEEP} seeds)"
    for s in $(seq 1 "${SWEEP}"); do
        if ! ( cd "$CRATE" && DIFF_SEED="$s" timeout 600 cargo test --release -- \
                 --test-threads=1 >/tmp/sweep.log 2>&1 ); then
            fail "seed $s"
            grep -A12 '^---- ' /tmp/sweep.log | head -30
        fi
    done
    [ "$FAIL" -eq 0 ] && echo "SWEEP PASSED (${SWEEP} seeds)"
fi

exit "$FAIL"
