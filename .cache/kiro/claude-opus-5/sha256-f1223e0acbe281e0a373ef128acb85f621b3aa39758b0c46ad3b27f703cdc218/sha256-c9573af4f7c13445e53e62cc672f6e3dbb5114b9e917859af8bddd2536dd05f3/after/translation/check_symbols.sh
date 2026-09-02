#!/usr/bin/env bash
# Phase D — symbol parity gate.
# Exits 0 only when the Rust .so exports every symbol the C .so exports.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
R_SO="${1:-$ROOT/translation/target/release/libmaxnmin_lib.so}"

[ -f "$C_SO" ] || { echo "C .so not found; build c_src first" >&2; exit 2; }
[ -f "$R_SO" ] || { echo "Rust .so not found at $R_SO" >&2; exit 2; }

echo "C   : $C_SO"
echo "Rust: $R_SO"

dump() { nm -D --defined-only "$1" | awk '$2 ~ /^[TtWwBbDdRr]$/ {print $3}' | sort -u; }

dump "$C_SO" > /tmp/parity_c.txt
dump "$R_SO" > /tmp/parity_r.txt

echo "C defined symbols   : $(wc -l < /tmp/parity_c.txt)"
echo "Rust defined symbols: $(wc -l < /tmp/parity_r.txt)"

MISSING="$(comm -23 /tmp/parity_c.txt /tmp/parity_r.txt)"
EXTRA="$(comm -13 /tmp/parity_c.txt /tmp/parity_r.txt)"

# Undefined imports that are not libc / libgcc-unwinder / ld.so runtime symbols.
BAD_UNDEF="$(nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
  | grep -v -E '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
  | grep -v -E '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_[a-z_]+|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)$' \
  || true)"

RC=0
if [ -n "$MISSING" ]; then echo "MISSING FROM RUST:"; echo "$MISSING"; RC=1; fi
if [ -n "$EXTRA" ]; then echo "EXTRA IN RUST (informational):"; echo "$EXTRA"; fi
if [ -n "$BAD_UNDEF" ]; then echo "NON-LIBC UNDEFINED IN RUST:"; echo "$BAD_UNDEF"; RC=1; fi

if [ "$RC" -eq 0 ]; then echo "SYMBOL PARITY: OK (0 missing, 0 non-libc undefined)"; fi
exit "$RC"
