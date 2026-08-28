#!/usr/bin/env bash
# End-to-end verification driver: reproduces every phase in order.
#
#   ./verify.sh          # phases A-D (fast: ~1 minute)
#   ./verify.sh --full   # additionally: 2^32 exhaustive sweeps, mutation check,
#                        #               and the C-optimization-level matrix
set -uo pipefail
cd "$(dirname "$0")"
FULL=0; [ "${1:-}" = "--full" ] && FULL=1
RC=0
step() { echo; echo "=================== $* ==================="; }

step "Phase A.1 - build the C shared library"
mkdir -p ../c_src/build
(cd ../c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null) \
  || { echo "C build FAILED"; exit 1; }
C_SO=$(ls ../c_src/build/lib*.so)
echo "C  .so: $C_SO"

step "Phase A.2 - cargo check + build the Rust cdylib"
cargo check 2>&1 | tail -3
cargo build --release 2>&1 | tail -2
R_SO=target/release/libmax_size_frame_lib.so
echo "Rust .so: $R_SO"

step "Phase A.3 / D - symbol parity (nm -D)"
SYMDIR=$(mktemp -d) || { echo "cannot create temp dir"; exit 1; }
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort > "$SYMDIR/c" || { echo "nm on C .so FAILED"; exit 1; }
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort > "$SYMDIR/r" || { echo "nm on Rust .so FAILED"; exit 1; }
# A silently-empty symbol list must never be reported as a pass.
[ -s "$SYMDIR/c" ] || { echo "ERROR: C .so exports NOTHING -- build is broken"; exit 1; }
[ -s "$SYMDIR/r" ] || { echo "ERROR: Rust .so exports NOTHING -- build is broken"; exit 1; }
echo "C exports:    $(tr '\n' ' ' < "$SYMDIR/c")"
echo "Rust exports: $(tr '\n' ' ' < "$SYMDIR/r")"
MISSING=$(comm -23 "$SYMDIR/c" "$SYMDIR/r")
EXTRA=$(comm -13 "$SYMDIR/c" "$SYMDIR/r")
rm -rf "$SYMDIR"
if [ -n "$MISSING" ]; then echo "MISSING FROM RUST: $MISSING"; RC=1; else echo "symbol diff: EMPTY (0 missing)"; fi
[ -n "$EXTRA" ] && echo "NOTE extra Rust exports: $EXTRA"
echo "Rust undefined non-libc symbols:"
nm -D -u "$R_SO" | awk '{print $NF}' | sed 's/@.*//' \
  | grep -vE '^(_ITM_|__cxa_|__gmon_start__|_Unwind_|__errno_location|__tls_get_addr|__pthread|pthread_|_dl_)' \
  | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)$' \
  | sed 's/^/  /' | grep . || echo "  (none)"

step "Phases B, C, D - differential suite x all feature combos x both profiles"
./run_all_features.sh || RC=1

if [ "$FULL" -eq 1 ]; then
  step "Phase B (deep) - full 2^32 exhaustive single-argument sweeps"
  timeout 600 cargo test --release --test deep_sweeps -- --ignored 2>&1 | grep -E '^test |test result' || RC=1

  step "Meta - mutation sensitivity check (proves the suite detects divergence)"
  ./mutation_check.sh || RC=1

  step "Robustness - C compiled at many optimization levels / standards"
  OUT=$(mktemp -d)
  for flags in "-O0" "-O1" "-O2" "-O3" "-Os" "-Ofast" "-O2 -std=c99" "-O2 -std=c11" "-O2 -fwrapv" "-O2 -ftrapv" "-O3 -march=native"; do
    tag=$(echo "$flags" | tr -d ' -/=')
    so="$OUT/libc_$tag.so"
    if ! eval gcc $flags -fPIC -shared -I../c_src/include -o "$so" ../c_src/src/lib.c 2>/dev/null; then
      echo "  SKIP gcc $flags (cannot build here)"; continue
    fi
    if DIFFTEST_C_SO="$so" timeout 600 cargo test --release >/dev/null 2>&1; then
      echo "  PASS | C built with gcc $flags"
    else
      echo "  FAIL | C built with gcc $flags"; RC=1
    fi
  done
  if command -v clang >/dev/null 2>&1; then
    for o in -O0 -O2 -O3; do
      so="$OUT/libclang$o.so"
      clang $o -fPIC -shared -I../c_src/include -o "$so" ../c_src/src/lib.c 2>/dev/null || continue
      if DIFFTEST_C_SO="$so" timeout 600 cargo test --release >/dev/null 2>&1; then
        echo "  PASS | C built with clang $o"
      else
        echo "  FAIL | C built with clang $o"; RC=1
      fi
    done
  fi
  rm -rf "$OUT"
fi

echo
if [ "$RC" -eq 0 ]; then echo "VERIFICATION PASSED"; else echo "VERIFICATION FAILED"; fi
exit $RC
