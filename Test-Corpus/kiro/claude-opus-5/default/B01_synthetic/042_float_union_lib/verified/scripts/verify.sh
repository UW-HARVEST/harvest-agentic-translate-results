#!/usr/bin/env bash
# Full verification run: build both sides, diff the exported symbols, enumerate
# feature combinations, confirm neither project builds a binary, and run the
# whole differential suite against BOTH the debug and the release cdylib.
set -euo pipefail
cd "$(dirname "$0")/.."
CRATE=$PWD
ROOT=$(cd .. && pwd)
C_BUILD=$ROOT/c_src/build

step() { printf '\n=========== %s ===========\n' "$1"; }

step "build the C shared library"
mkdir -p "$C_BUILD"
(cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
ls -l "$C_BUILD/libdriver.so"

step "build the Rust cdylib (debug + release)"
timeout 600 cargo build
timeout 600 cargo build --release
ls -l target/debug/libdriver.so target/release/libdriver.so

step "SYMBOLS.md: exported-symbol diff (must be empty)"
for profile in debug release; do
  echo "--- $profile ---"
  diff <(nm -D --defined-only "$C_BUILD/libdriver.so" | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only "target/$profile/libdriver.so" | awk '{print $NF}' | sort -u) \
    && echo "symbol diff EMPTY ($profile)"
done

step "SYMBOLS.md: undefined non-libc symbols in the Rust .so (must be none)"
nm -D -u target/release/libdriver.so | awk '{print $NF}' | sed 's/@.*//' | sort -u \
  | grep -vE '^(_ITM_(de)?registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__errno_location|__gmon_start__|__tls_get_addr|_Unwind_[A-Za-z]+|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|fwrite|getcwd|getenv|gettid|localeconv|fegetround|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|stdout|strlen|syscall|write|writev)$' \
  && { echo "UNEXPECTED undefined symbols above"; exit 1; } || echo "no unexpected undefined symbols"

step "does either project build a binary/driver executable?"
if grep -qE '^\s*add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "C builds an executable -- stdout comparison REQUIRED"; exit 1
else
  echo "c_src/CMakeLists.txt: no add_executable -> C builds a SHARED LIBRARY only"
fi
if grep -qE '^\s*\[\[bin\]\]' Cargo.toml || [ -f src/main.rs ] || [ -d src/bin ]; then
  echo "Rust builds a binary -- stdout comparison REQUIRED"; exit 1
else
  echo "Cargo.toml: no [[bin]], no src/main.rs, no src/bin/ -> cdylib only"
fi
echo "=> no binary on either side; the byte-for-byte stdout comparison is done"
echo "   through the .so exports instead (that is where all output originates)."

step "feature combinations"
./scripts/check_features.sh

step "differential suite -- DEBUG cdylib"
timeout 600 cargo test 2>&1 | grep -E '^(test |test result|error)' || true
timeout 600 cargo test >/dev/null

step "differential suite -- RELEASE cdylib (panic = abort, the shipping artifact)"
timeout 600 cargo test --release 2>&1 | grep -E '^(test |test result|error)' || true
timeout 600 cargo test --release >/dev/null

step "differential suite -- release cdylib loaded from a debug test binary"
DRIVER_RUST_SO=$CRATE/target/release/libdriver.so timeout 600 cargo test >/dev/null
echo "ok"

step "ALL CHECKS PASSED"
