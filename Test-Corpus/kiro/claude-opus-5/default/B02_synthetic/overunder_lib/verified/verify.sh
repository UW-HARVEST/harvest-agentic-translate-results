#!/usr/bin/env bash
# Full verification driver. Run from the crate root (translation/).
#
#   ./verify.sh
#
# Phases:
#   0. build the C .so and the Rust .so
#   1. symbol parity (nm -D) — the diff must be empty
#   2. undefined non-libc symbols in the Rust .so — must be empty
#   3. cargo check + full test suite for EVERY feature combination
#   4. re-run the suite against the debug-profile .so (overflow-checks = on)
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
fail=0
step() { printf '\n=== %s ===\n' "$1"; }
ok()   { printf 'PASS  %s\n' "$1"; }
bad()  { printf 'FAIL  %s\n' "$1"; fail=1; }

step "0. build"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
[ -n "$C_SO" ] || { bad "no C .so"; exit 1; }
( cd "$CRATE" && timeout 600 cargo build --release >/dev/null 2>&1 ) || { bad "Rust release build"; exit 1; }
( cd "$CRATE" && timeout 600 cargo build >/dev/null 2>&1 ) || { bad "Rust debug build"; exit 1; }
R_SO="$CRATE/target/release/liboverunder_lib.so"
ok "built $C_SO and $R_SO"

step "1. symbol parity (nm -D --defined-only)"
# Keep only global text/data symbols; drop libc/CRT boilerplate that a Rust
# cdylib always carries (_init/_fini/__bss_start/_edata/_end/rust_*).
filt() {
  nm -D --defined-only "$1" 2>/dev/null \
    | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $3}' \
    | grep -vE '^(_init|_fini|__bss_start|_edata|_end|_?_?rust_[a-z_]*|__rust_[a-z_0-9]*)$' \
    | sort -u
}
filt "$C_SO" > /tmp/c.syms
filt "$R_SO" > /tmp/r.syms
printf 'C exports:    %s\nRust exports: %s\n' "$(wc -l < /tmp/c.syms)" "$(wc -l < /tmp/r.syms)"
MISSING="$(comm -23 /tmp/c.syms /tmp/r.syms)"
if [ -z "$MISSING" ]; then ok "0 C symbols missing from the Rust .so"
else printf 'missing from Rust:\n%s\n' "$MISSING"; bad "symbol diff not empty"; fi
EXTRA="$(comm -13 /tmp/c.syms /tmp/r.syms)"
[ -n "$EXTRA" ] && printf 'note: extra Rust-only symbols (allowed):\n%s\n' "$EXTRA"

step "2. undefined non-libc symbols in the Rust .so"
# Strip the @GLIBC_x.y version suffix, then filter out everything that is
# libc / libm / CRT / unwinder boilerplate. Anything left would be a real
# unresolved dependency.
UND="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u \
  | grep -vE '^(printf|putchar|puts|fputs|fwrite|sqrt|memcpy|memmove|memset|memcmp|bcmp|strlen|strncpy|malloc|free|realloc|calloc|posix_memalign|aligned_alloc|abort|exit|atexit|getenv|environ|write|writev|read|open|open64|close|lseek|lseek64|stat|stat64|fstat|fstat64|lstat|lstat64|statx|readlink|realpath|getcwd|poll|nanosleep|clock_gettime|sched_getaffinity|sched_yield|sysconf|mmap|mmap64|munmap|mprotect|madvise|sigaltstack|sigaction|signal|syscall|gettid|dl_iterate_phdr|dlsym|dladdr|dlopen|dlclose|dlerror|pthread_[a-z_0-9]*|_ITM_[a-zA-Z]*|_Unwind_[a-zA-Z]*|__[a-zA-Z0-9_]*)$' || true)"
if [ -z "$UND" ]; then ok "no unexpected undefined symbols (only libc/libm/CRT)"
else printf '%s\n' "$UND"; bad "unexpected undefined symbols"; fi

step "3. feature combinations"
# Enumerate every feature declared in Cargo.toml.
FEATS="$(cd "$CRATE" && awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&/^[a-zA-Z0-9_-]+[ ]*=/{print $1}' Cargo.toml | tr -d '"')"
if [ -z "$FEATS" ]; then
  echo "Cargo.toml declares no [features] -> the only configurations are the"
  echo "default build and --no-default-features (equivalent here)."
  COMBOS=("" "--no-default-features" "--all-features")
else
  # power set of the declared features
  COMBOS=("" "--no-default-features" "--all-features")
  set -- $FEATS
  n=$#
  for ((mask=1; mask<(1<<n); mask++)); do
    sel=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then eval "f=\${$((i+1))}"; sel="${sel:+$sel,}$f"; fi
    done
    COMBOS+=("--no-default-features --features $sel")
  done
fi
for combo in "${COMBOS[@]}"; do
  label="cargo test --release ${combo:-<default>}"
  ( cd "$CRATE" && timeout 600 cargo check --release $combo >/dev/null 2>&1 ) \
    || { bad "cargo check $combo"; continue; }
  out="$(cd "$CRATE" && timeout 600 cargo test --release $combo 2>&1)"
  n_ok="$(printf '%s' "$out" | grep -cE '^test .* \.\.\. ok$')"
  if printf '%s' "$out" | grep -qE '^test result: FAILED|panicked'; then
    printf '%s\n' "$out" | grep -E 'panicked|^test result|assertion' | head -20
    bad "$label"
  else
    ok "$label ($n_ok tests)"
  fi
done

step "4. same suite against the DEBUG-profile .so (debug_assertions on)"
out="$(cd "$CRATE" && RUST_SO="$CRATE/target/debug/liboverunder_lib.so" timeout 600 cargo test --release 2>&1)"
n_ok="$(printf '%s' "$out" | grep -cE '^test .* \.\.\. ok$')"
if printf '%s' "$out" | grep -qE '^test result: FAILED|panicked'; then
  printf '%s\n' "$out" | grep -E 'panicked|^test result|assertion' | head -20
  bad "debug-profile .so"
else
  ok "debug-profile .so ($n_ok tests)"
fi

printf '\n============================\n'
if [ "$fail" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$fail"
