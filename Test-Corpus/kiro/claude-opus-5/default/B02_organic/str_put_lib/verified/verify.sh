#!/usr/bin/env bash
# Phase D driver: rebuild both shared objects, diff their dynamic symbol
# tables, and run the whole differential suite under every feature combination
# declared in Cargo.toml.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

echo "=== 1. build the C shared library ==="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/tmp/cbuild.log 2>&1 || { tail -20 /tmp/cbuild.log; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so | head -1)"
echo "C   .so: $C_SO"

echo "=== 2. build the Rust cdylib ==="
timeout 600 cargo build --release >/tmp/rbuild.log 2>&1 || { tail -20 /tmp/rbuild.log; exit 1; }
R_SO="$ROOT/translation/target/release/libstr_put_lib.so"
echo "RUST .so: $R_SO"

echo "=== 3. symbol parity (nm -D) ==="
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/r_syms.txt
echo "C exports:    $(wc -l < /tmp/c_syms.txt)"
echo "Rust exports: $(grep -cxF -f /tmp/c_syms.txt /tmp/r_syms.txt) of them"
MISSING="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
if [ -n "$MISSING" ]; then
  echo "MISSING FROM RUST:"; echo "$MISSING"; FAIL=1
else
  echo "symbol diff: EMPTY (0 missing)"
fi
echo "--- undefined non-libc symbols in the Rust .so ---"
# Everything the Rust cdylib imports must come from libc, libgcc's unwinder,
# or libpthread -- i.e. no undefined symbol may belong to the library itself.
nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sed '/^$/d' \
  | grep -v -E '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|pthread_)' \
  | grep -v -x -E '(__assert_fail|__errno_location|__libc_start_main|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat|fstat64|getcwd|getenv|gettid|lseek|lseek64|malloc|memcpy|memmove|memset|memcmp|mmap|mmap64|munmap|open|open64|posix_memalign|printf|read|readlink|realloc|realpath|sprintf|stat|stat64|statx|strcmp|strlen|syscall|write|writev)' \
  > /tmp/r_undef.txt
if [ -s /tmp/r_undef.txt ]; then
  echo "UNEXPECTED:"; cat /tmp/r_undef.txt; FAIL=1
else
  echo "none"
fi

echo "=== 4. feature combinations ==="
# Every feature name declared in [features] (excluding the "default" key).
FEATURES="$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/[ \t]/,"",a[1]); if (a[1]!="default") print a[1]}' Cargo.toml)"
if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("")
else
  # power set of the declared features
  COMBOS=("")
  for f in $FEATURES; do
    NEW=()
    for c in "${COMBOS[@]}"; do
      NEW+=("$c")
      if [ -z "$c" ]; then NEW+=("$f"); else NEW+=("$c,$f"); fi
    done
    COMBOS=("${NEW[@]}")
  done
fi

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    LABEL="<default / no-default-features>"
    ARGS=(--no-default-features)
  else
    LABEL="$combo"
    ARGS=(--no-default-features --features "$combo")
  fi
  echo "--- cargo build --release ${ARGS[*]} ($LABEL) ---"
  timeout 600 cargo build --release "${ARGS[@]}" >/tmp/fb.log 2>&1 \
    || { echo "BUILD FAILED"; tail -20 /tmp/fb.log; FAIL=1; continue; }
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/r_syms_c.txt
  M="$(comm -23 /tmp/c_syms.txt /tmp/r_syms_c.txt)"
  [ -n "$M" ] && { echo "MISSING SYMBOLS under $LABEL:"; echo "$M"; FAIL=1; }
  echo "--- cargo test --release ${ARGS[*]} ($LABEL) ---"
  timeout 600 cargo test --release "${ARGS[@]}" >/tmp/ft.log 2>&1 \
    || { echo "TESTS FAILED under $LABEL"; grep -E "^test .* FAILED|panicked" /tmp/ft.log | head -20; FAIL=1; }
  grep -E "test result" /tmp/ft.log
done

echo
if [ "$FAIL" = 0 ]; then echo "PHASE D: ALL CHECKS PASSED"; else echo "PHASE D: FAILURES PRESENT"; fi
exit $FAIL
