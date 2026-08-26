#!/usr/bin/env bash
# Phase D: every symbol the C .so exports must also be exported by the Rust .so.
# Exits non-zero if the diff is not empty (or if anything went wrong).
set -uo pipefail
cd "$(dirname "$0")"

C_SO=${C_SO_PATH:-c_src/build/libtranslated_rust.so}
RUST_SO=${RUST_SO_PATH:-}
if [[ -z "$RUST_SO" ]]; then
  for c in target/debug/libgotomach_lib.so target/release/libgotomach_lib.so; do
    [[ -f "$c" ]] && RUST_SO="$c" && break
  done
fi

if [[ ! -f "$C_SO" ]]; then
  echo "ERROR: C .so not found at $C_SO" >&2; exit 2
fi
if [[ -z "$RUST_SO" || ! -f "$RUST_SO" ]]; then
  echo "ERROR: Rust .so not found (run 'cargo build')" >&2; exit 2
fi

WORK=$(mktemp -d "${TMPDIR:-/tmp}/symparity.XXXXXX") || { echo "ERROR: mktemp failed" >&2; exit 2; }
trap 'rm -rf "$WORK"' EXIT

syms () { nm -D --defined-only "$1" | awk '{print $3}' | grep -v -E '^(_ITM_|__gmon)' | sort -u; }

syms "$C_SO"    > "$WORK/c"    || { echo "ERROR: nm failed on $C_SO" >&2; exit 2; }
syms "$RUST_SO" > "$WORK/rust" || { echo "ERROR: nm failed on $RUST_SO" >&2; exit 2; }

n_c=$(wc -l < "$WORK/c"); n_r=$(wc -l < "$WORK/rust")
echo "C    .so: $C_SO    ($n_c exported symbols)"
echo "Rust .so: $RUST_SO ($n_r exported symbols)"

# Anti-vacuity guards: an empty symbol list must never be reported as a PASS.
if (( n_c == 0 )); then echo "FAIL: C .so exported 0 symbols - nm/temp-file problem" >&2; exit 2; fi
if (( n_r == 0 )); then echo "FAIL: Rust .so exported 0 symbols" >&2; exit 1; fi

echo
echo "--- symbols in C but MISSING from Rust ---"
missing=$(comm -23 "$WORK/c" "$WORK/rust")
echo "${missing:-<none>}"
echo
echo "--- symbols only in Rust (extra, allowed) ---"
extra=$(comm -13 "$WORK/c" "$WORK/rust")
echo "${extra:-<none>}"
echo
echo "--- non-libc undefined symbols in Rust .so ---"
und=$(nm -D --undefined-only "$RUST_SO" \
  | awk '{print $NF}' | sed 's/@.*//' | sort -u \
  | grep -v -E '^(_Unwind_|__|_ITM_|pthread_)' \
  | grep -v -x -E 'malloc|free|calloc|realloc|posix_memalign|printf|puts|fwrite|memcpy|memmove|memset|memcmp|bcmp|strlen|abort|getenv|getcwd|readlink|realpath|open|open64|close|read|write|writev|lseek|lseek64|fstat|fstat64|stat|stat64|statx|mmap|mmap64|munmap|mprotect|syscall|sysconf|dl_iterate_phdr|dladdr|gettid|sigaction|sigaltstack|getpid|poll|pipe2|fcntl|environ|signal|raise|pthread_self')
echo "${und:-<none>}"

if [[ -n "$missing" ]]; then echo; echo "FAIL: $(echo "$missing" | wc -l) C symbol(s) missing from Rust"; exit 1; fi
if [[ -n "$und" ]]; then echo; echo "FAIL: unexpected undefined symbols"; exit 1; fi
echo
echo "PASS: symbol parity complete ($n_c/$n_c C symbols present in Rust, 0 unexpected undefined)"
