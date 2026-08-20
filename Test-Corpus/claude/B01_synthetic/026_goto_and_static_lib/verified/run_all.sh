#!/usr/bin/env bash
# Full verification sweep: symbol parity + differential rows, for every build
# configuration.
#
#   ./run_all.sh
#
# `Cargo.toml` declares `[features] default = []` and no other feature, and
# `c_src/CMakeLists.txt` has no `option()` / `target_compile_definitions` and the
# sources contain no `#ifdef`, so the complete configuration space is the three
# cargo feature invocations below (x debug/release).
set -euo pipefail

cd "$(dirname "$0")"
CARGO_FLAGS=(--offline)

echo "=============================================================="
echo " 1. Build the C shared library"
echo "=============================================================="
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
C_SO=c_src/build/libdriver.so
ls -l "$C_SO"

echo
echo "=============================================================="
echo " 2. cargo check for every feature combination"
echo "=============================================================="
COMBOS=("--no-default-features" "" "--all-features")
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    # shellcheck disable=SC2086
    if timeout 600 cargo check "${CARGO_FLAGS[@]}" $combo >/dev/null 2>&1; then
        echo "  cargo check $label ... ok"
    else
        echo "  cargo check $label ... FAILED"
        # shellcheck disable=SC2086
        timeout 600 cargo check "${CARGO_FLAGS[@]}" $combo
        exit 1
    fi
done

echo
echo "=============================================================="
echo " 3. Symbol parity: nm -D  (C .so  vs  Rust .so)"
echo "=============================================================="
# shellcheck disable=SC2086
timeout 600 cargo build "${CARGO_FLAGS[@]}" --lib --no-default-features >/dev/null
RUST_SO=target/debug/libdriver.so
c_syms=$(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u)
r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
echo "C   exports: $(echo "$c_syms" | tr '\n' ' ')"
echo "Rust exports (public): $(echo "$r_syms" | tr '\n' ' ')"
missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms") || true)
if [[ -n "$missing" ]]; then
    echo "MISSING FROM RUST .so:"
    echo "$missing"
    exit 1
fi
echo "  symbol diff ... empty (0 missing)"

nonlibc=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location|_Unwind_)' \
    | grep -vxE 'abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev' \
    || true)
if [[ -n "$nonlibc" ]]; then
    echo "UNEXPECTED undefined non-libc symbols in the Rust .so:"
    echo "$nonlibc"
    exit 1
fi
echo "  undefined non-libc symbols ... none"

echo
echo "=============================================================="
echo " 4. Differential rows (Phase B + Phase C) per configuration"
echo "=============================================================="
for combo in "${COMBOS[@]}"; do
    label="${combo:-<default>}"
    echo
    echo "--- cargo test $label (debug) ---"
    # shellcheck disable=SC2086
    timeout 600 cargo test "${CARGO_FLAGS[@]}" $combo --test differential 2>&1 | tail -40
done

echo
echo "--- cargo test --release (release profile, panic = \"abort\") ---"
timeout 600 cargo test "${CARGO_FLAGS[@]}" --release --test differential 2>&1 | tail -40

echo
echo "=============================================================="
echo " ALL CONFIGURATIONS VERIFIED"
echo "=============================================================="
