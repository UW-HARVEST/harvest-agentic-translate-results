#!/usr/bin/env bash
# Full verification driver: Phase A (symbol parity), Phase B/C (differential
# tests) across EVERY feature combination and both Rust build profiles.
set -uo pipefail
cd "$(dirname "$0")"
SCRATCH="${TMPDIR:-.}/tr_verify.$$"
mkdir -p "$SCRATCH"
trap 'rm -rf "$SCRATCH"' EXIT

FAIL=0
step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '   [ok]   %s\n' "$*"; }
bad()  { printf '   [FAIL] %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------------------
# Enumerate build-time configurations.
#   * Cargo.toml has no [features] section -> the only feature combination is
#     the empty one (default == no-default-features == all-features).
#   * c_src/CMakeLists.txt has no option()/-D and lib.c has no #if* -> the C
#     library likewise has a single configuration.
# --------------------------------------------------------------------------
step "Enumerating feature combinations"
if grep -q '^\[features\]' Cargo.toml; then
    echo "   Cargo.toml declares [features]; enumerate them here."
    exit 2
fi
COMBOS=("" "--no-default-features" "--all-features")
echo "   feature combinations: (default) --no-default-features --all-features"
if grep -qE 'option\(|target_compile_definitions|add_definitions' c_src/CMakeLists.txt; then
    bad "c_src/CMakeLists.txt has build options that were not enumerated"
else
    ok "C library has exactly one build configuration"
fi

# --------------------------------------------------------------------------
step "Building the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || bad "C build failed"
C_SO=c_src/build/libtranslated_rust.so
[[ -f $C_SO ]] && ok "$C_SO" || bad "missing $C_SO"

# --------------------------------------------------------------------------
step "cargo check for every feature combination"
for f in "${COMBOS[@]}"; do
    if timeout 300 cargo check $f >/dev/null 2>&1; then
        ok "cargo check ${f:-(default)}"
    else
        bad "cargo check ${f:-(default)}"
    fi
done

# --------------------------------------------------------------------------
step "Building the Rust cdylib (release + debug) for every feature combination"
for f in "${COMBOS[@]}"; do
    timeout 300 cargo build --release $f >/dev/null 2>&1 \
        && ok "cargo build --release ${f:-(default)}" \
        || bad "cargo build --release ${f:-(default)}"
    timeout 300 cargo build $f >/dev/null 2>&1 \
        && ok "cargo build ${f:-(default)}" \
        || bad "cargo build ${f:-(default)}"
done

# --------------------------------------------------------------------------
step "Phase A/D: exported-symbol parity (nm -D)"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$SCRATCH/c_syms"
for so in target/release/libsh_puts_lib.so target/debug/libsh_puts_lib.so; do
    [[ -f $so ]] || { bad "missing $so"; continue; }
    nm -D --defined-only "$so" | awk '{print $3}' | sort -u > "$SCRATCH/r_syms"
    MISSING=$(comm -23 "$SCRATCH/c_syms" "$SCRATCH/r_syms")
    EXTRA=$(comm -13 "$SCRATCH/c_syms" "$SCRATCH/r_syms")
    if [[ -z $MISSING ]]; then
        ok "$so exports all $(wc -l < "$SCRATCH/c_syms") C symbols (0 missing)"
    else
        bad "$so is MISSING: $(echo $MISSING | tr '\n' ' ')"
    fi
    [[ -n $EXTRA ]] && echo "   (extra Rust-only exports: $(echo $EXTRA | tr '\n' ' '))"
    rm -f "$SCRATCH/r_syms"
done
step "Undefined non-libc symbols in the Rust .so"
UNDEF=$(nm -D --undefined-only target/release/libsh_puts_lib.so \
        | awk '{print $NF}' | sed 's/@.*//' \
        | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__tls_get_addr|__assert_fail|__errno_location|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|gettid|lseek|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap|mmap64|munmap|open|open64|posix_memalign|printf|pthread_|read|readlink|realloc|realpath|sprintf|stat|stat64|statx|strcmp|strlen|syscall|write|writev|__libc_|_dl_|sigaction|sigaltstack|sysconf|pthread)' || true)
if [[ -z $UNDEF ]]; then ok "0 unresolved non-libc symbols"; else bad "unresolved: $UNDEF"; fi
rm -f "$SCRATCH/c_syms"

# --------------------------------------------------------------------------
step "Phase B/C: differential tests, every feature combination x both profiles"
for f in "${COMBOS[@]}"; do
    for prof in release debug; do
        export TR_RUST_SO="$PWD/target/$prof/libsh_puts_lib.so"
        if timeout 600 cargo test $f -- --test-threads=1 >"$SCRATCH/test.log" 2>&1; then
            N=$(grep -c '^test .* ok$' "$SCRATCH/test.log")
            ok "cargo test ${f:-(default)} against target/$prof (.so) — $N tests passed"
        else
            bad "cargo test ${f:-(default)} against target/$prof"
            tail -40 "$SCRATCH/test.log"
        fi
        rm -f "$SCRATCH/test.log"
    done
done
unset TR_RUST_SO

printf '\n'
if [[ $FAIL -eq 0 ]]; then
    printf '\033[1;32mALL VERIFICATION STEPS PASSED\033[0m\n'
else
    printf '\033[1;31mVERIFICATION FAILED\033[0m\n'
fi
exit $FAIL
