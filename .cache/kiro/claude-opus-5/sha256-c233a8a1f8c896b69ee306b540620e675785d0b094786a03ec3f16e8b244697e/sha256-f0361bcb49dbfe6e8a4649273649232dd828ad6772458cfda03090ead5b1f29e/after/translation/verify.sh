#!/usr/bin/env bash
# Phase D driver: symbol parity + full test suite across every build
# configuration.  Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0

say() { printf '\n=== %s ===\n' "$*"; }

say "1. build C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C BUILD FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C .so: $C_SO"

say "2. cargo check + build (dev and release)"
timeout 600 cargo check --all-targets 2>&1 | tail -3
timeout 600 cargo build          2>&1 | tail -2
timeout 600 cargo build --release 2>&1 | tail -2

say "3. feature combinations declared in Cargo.toml"
FEATS=$(cargo read-manifest | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin)["features"].keys()))')
if [ -z "$FEATS" ]; then
  echo "no [features] declared -> the default build is the only configuration"
else
  echo "features: $FEATS"
fi
for combo in "" "--no-default-features"; do
  echo "--- cargo check $combo"
  timeout 600 cargo check $combo 2>&1 | tail -2 || FAIL=1
done

say "4. symbol parity (nm -D)"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
for prof in debug release; do
  RS="target/$prof/libsh_geti_lib.so"
  nm -D --defined-only "$RS" | awk '{print $3}' | grep -v '^_' | sort -u > "/tmp/r_syms_$prof.txt"
  MISSING=$(comm -23 /tmp/c_syms.txt "/tmp/r_syms_$prof.txt")
  printf 'C exports: %s   Rust(%s) exports (non-underscore): %s\n' \
    "$(wc -l < /tmp/c_syms.txt)" "$prof" "$(wc -l < /tmp/r_syms_$prof.txt)"
  if [ -n "$MISSING" ]; then
    echo "MISSING FROM RUST ($prof):"; echo "$MISSING"; FAIL=1
  else
    echo "symbol diff ($prof): EMPTY - 0 missing"
  fi
  # Everything Rust's std / libgcc-unwind pulls in: leading-underscore names
  # plus the plain libc entry points.  Anything left over is a real dangling ref.
  UNDEF=$(nm -D --undefined-only "$RS" | awk '{print $NF}' | sed 's/@.*//' \
    | grep -v '^_' \
    | grep -vxE 'realpath|realloc|free|malloc|calloc|posix_memalign|memcpy|memmove|memset|bcmp|memcmp|strcmp|strlen|printf|sprintf|abort|write|writev|read|close|open64|lseek64|fstat64|stat64|statx|getcwd|getenv|readlink|mmap64|munmap|syscall|gettid|dl_iterate_phdr|pthread_key_create|pthread_key_delete|pthread_setspecific' || true)
  if [ -n "$UNDEF" ]; then
    echo "NON-LIBC UNDEFINED SYMBOLS ($prof):"; echo "$UNDEF"; FAIL=1
  else
    echo "undefined non-libc symbols ($prof): 0"
  fi
done

say "5. differential test suite"
for prof in debug release; do
  RS="$PWD/target/$prof/libsh_geti_lib.so"
  echo "--- Rust .so = target/$prof/libsh_geti_lib.so"
  LOG="/tmp/cargo_test_$prof.log"
  RUST_SO="$RS" C_SO="$C_SO" timeout 600 cargo test > "$LOG" 2>&1
  RC=$?
  grep -E "^(running|test result:)" "$LOG" | sed "s/^/  $prof: /"
  if [ "$RC" -ne 0 ]; then
    echo "TESTS FAILED for $prof (rc=$RC); see $LOG"
    grep -E "FAILED|panicked|signal:" "$LOG" | head -20
    FAIL=1
  fi
  # cargo test rebuilds target/debug/libsh_geti_lib.so; make sure the release
  # artifact is still current for the next iteration
  timeout 600 cargo build --release >/dev/null 2>&1
done

say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL PHASE D CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
