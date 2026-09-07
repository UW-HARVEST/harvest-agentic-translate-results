#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
#
# Enumerates the [features] table from Cargo.toml mechanically (no hard-coded
# list), then runs cargo check + the full differential suite for each subset.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
CRATE="$PWD"
ROOT="$(dirname "$CRATE")"
C_SO="$(find "$ROOT/c_src/build" -name '*.so' -type f | sort | head -1)"
RUST_SO="$CRATE/target/release/libtfm_lib.so"
fail=0

echo "=============================================================="
echo "Phase D.1 — symbol parity (nm -D)"
echo "=============================================================="
cargo build --release >/dev/null 2>&1 || { echo "FAIL: cargo build"; exit 1; }
[ -f "$C_SO" ] || { echo "FAIL: C .so not found under $ROOT/c_src/build"; exit 1; }
echo "C    .so: $C_SO"
echo "Rust .so: $RUST_SO"

nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u > /tmp/r_syms.txt
echo "C exports $(wc -l < /tmp/c_syms.txt) symbol(s); Rust exports $(wc -l < /tmp/r_syms.txt)."

MISSING="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
if [ -n "$MISSING" ]; then
    echo "FAIL: symbols exported by C but MISSING from Rust:"
    echo "$MISSING" | sed 's/^/    /'
    fail=1
else
    echo "PASS: symbol diff is empty — every C export is present in the Rust .so."
fi

# Undefined non-libc / non-compiler-runtime symbols in the Rust .so.
UNDEF="$(nm -D -u "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_|_Unwind_|__errno_location|__tls_get_addr|statx|gettid)' \
    | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev|sqrtf' \
    | sort -u)"
if [ -n "$UNDEF" ]; then
    echo "FAIL: unresolved non-libc symbols in the Rust .so:"; echo "$UNDEF" | sed 's/^/    /'; fail=1
else
    echo "PASS: 0 undefined non-libc/non-compiler-runtime symbols in the Rust .so."
fi

echo
echo "=============================================================="
echo "Phase D.2 — binary/driver stdout comparison"
echo "=============================================================="
if grep -qE 'add_executable' "$ROOT/c_src/CMakeLists.txt" \
   || grep -qE '^\[\[bin\]\]' "$CRATE/Cargo.toml" \
   || [ -d "$CRATE/src/bin" ] || [ -f "$CRATE/src/main.rs" ]; then
    echo "FAIL: a binary target exists but this script does not diff it."; fail=1
else
    echo "N/A: c_src/CMakeLists.txt declares only add_library(... SHARED) and the"
    echo "     crate declares no [[bin]]/src/main.rs, so there is no driver"
    echo "     executable whose stdout could be compared."
fi

echo
echo "=============================================================="
echo "Phase D.3 — Phases B+C under EVERY feature combination"
echo "=============================================================="
# Mechanically extract feature names from the [features] section of Cargo.toml.
mapfile -t FEATURES < <(awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
        sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, "");
        if ($0 != "default") print
    }
' Cargo.toml)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

run_combo() {
    local label="$1"; shift
    echo "--------------------------------------------------------------"
    echo ">>> combo: $label"
    # The cdylib must exist for the SAME profile the test binary is built with;
    # the harness refuses to cross-load, so build it explicitly for both.
    for prof in "" "--release"; do
        if ! cargo build $prof "$@" >/tmp/build_$$.log 2>&1; then
            echo "FAIL build ($label $prof)"; tail -20 /tmp/build_$$.log; fail=1; return
        fi
        if ! timeout 600 cargo test $prof "$@" >/tmp/test_$$.log 2>&1; then
            echo "FAIL tests ($label ${prof:-(dev)})"
            grep -E 'FAILED|panicked|test result' /tmp/test_$$.log | head -20; fail=1; return
        fi
        loaded="$(grep -m1 'Rust .so' /tmp/test_$$.log || true)"
        echo "    profile ${prof:-(dev)}:"
        grep -E 'test result' /tmp/test_$$.log | sed 's/^/      /'
    done
    echo "PASS ($label, dev + release)"
}

run_combo "default"
run_combo "--no-default-features" --no-default-features

# Full power set of the non-default features (empty when there are none).
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
    for ((mask=1; mask<(1<<n); mask++)); do
        combo=""
        for ((i=0; i<n; i++)); do
            if (( (mask>>i) & 1 )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
        done
        run_combo "--no-default-features --features $combo" --no-default-features --features "$combo"
    done
else
    echo "(no optional features declared: {default} and {} are the complete set)"
fi

echo
echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "PHASE D FAILURES PRESENT"; fi
echo "=============================================================="
exit "$fail"
