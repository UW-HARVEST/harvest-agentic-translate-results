#!/usr/bin/env bash
# Full verification driver: builds the C .so, the instrumented C harness and the
# Rust cdylib, checks symbol parity, then runs every differential test under
# every feature combination and both cargo profiles.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CARGO="cargo --offline"
FAIL=0

note() { printf '\n=== %s ===\n' "$*"; }
bad()  { printf 'FAIL: %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------- 1. build C
note "building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || bad "C library build"
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
echo "C .so: $C_SO"

note "building instrumented C harness (#includes c_src/src/lib.c verbatim)"
mkdir -p "$CRATE/tests/c_harness/build"
cc -shared -fPIC -O2 -Wno-unused-function \
   -o "$CRATE/tests/c_harness/build/libharness.so" \
   "$CRATE/tests/c_harness/harness.c" -lm || bad "harness build"

# ------------------------------------------------- 2. feature combos x profiles
# The crate declares exactly one feature and no defaults, so the full
# cross-product of feature combinations is {} and {expose_init_test_data}.
COMBOS=("" "expose_init_test_data")

for profile in release debug; do
  # An empty string would be passed to cargo as a bogus argument, so use an array.
  if [ "$profile" = release ]; then PROF=(--release); else PROF=(); fi

  for combo in "${COMBOS[@]}"; do
    if [ -z "$combo" ]; then
      FEAT=(--no-default-features)
      label="<no features>"
    else
      FEAT=(--no-default-features --features "$combo")
      label="$combo"
    fi

    note "profile=$profile features=$label : cargo check"
    $CARGO check "${PROF[@]}" "${FEAT[@]}" --all-targets >/dev/null 2>&1 \
      || bad "cargo check ($profile / $label)"

    note "profile=$profile features=$label : build cdylib"
    # cargo test does NOT refresh the cdylib artifact, so build it explicitly.
    $CARGO build "${PROF[@]}" "${FEAT[@]}" >/dev/null 2>&1 \
      || { bad "cargo build ($profile / $label)"; continue; }
    RS_SO="$CRATE/target/$profile/libjumpnode_lib.so"

    note "profile=$profile features=$label : symbol parity"
    c_syms=$(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u)
    r_syms=$(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
    if [ -n "$missing" ]; then
      bad "symbols exported by C but MISSING from Rust ($profile / $label):"
      echo "$missing"
    else
      echo "OK: every C symbol is exported by Rust"
      echo "  C:    $(echo "$c_syms" | tr '\n' ' ')"
      echo "  Rust: $(echo "$r_syms" | tr '\n' ' ')"
    fi
    # No undefined non-libc symbols in the Rust .so.
    undef=$(nm -D --undefined-only "$RS_SO" | awk '{print $NF}' \
            | sed 's/@.*//' \
            | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|sqrt|sprintf|memrchr|__memcpy|qsort|sysconf|getrandom|clock_gettime|pipe2|sigaction|sigaltstack|__libc_|environ|poll|nanosleep|mprotect|madvise|open|access|fcntl|dlsym|dladdr|dlopen|dlclose|dlerror|_dl_|__stack_chk_fail|__assert_fail)' )
    if [ -n "$undef" ]; then
      bad "unexpected non-libc undefined symbols in Rust .so ($profile / $label):"
      echo "$undef"
    else
      echo "OK: 0 undefined non-libc symbols"
    fi

    note "profile=$profile features=$label : differential tests"
    RUST_SO="$RS_SO" $CARGO test "${PROF[@]}" "${FEAT[@]}" 2>&1 \
      | grep -E '^(test |running |error|warning: unused)|test result' \
      | grep -vE '^test .* \.\.\. ok$'
    # shellcheck disable=SC2181
    RUST_SO="$RS_SO" $CARGO test "${PROF[@]}" "${FEAT[@]}" >/dev/null 2>&1 \
      || bad "differential tests ($profile / $label)"
  done
done

note "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"
