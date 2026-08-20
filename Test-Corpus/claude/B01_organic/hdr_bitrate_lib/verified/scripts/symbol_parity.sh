#!/usr/bin/env bash
# Phase D: every symbol the C .so exports must also be exported by the Rust .so
# with the exact same name. The diff must be EMPTY.
set -uo pipefail
cd "$(dirname "$0")/.."

C_SO=$(ls c_src/build/lib*.so 2>/dev/null | head -1)
# Prefer the artifacts the test harness itself builds (guaranteed fresh).
R_SO=""
for cand in target/difftest/release/libhdr_bitrate_lib.so \
            target/difftest/debug/libhdr_bitrate_lib.so \
            target/release/libhdr_bitrate_lib.so \
            target/debug/libhdr_bitrate_lib.so; do
  [ -f "$cand" ] && { R_SO="$cand"; break; }
done

[ -n "$C_SO" ] || { echo "C .so not built; run cmake first"; exit 1; }
[ -n "$R_SO" ] || { echo "Rust .so not built; run cargo build"; exit 1; }
echo "C   : $C_SO"
echo "Rust: $R_SO"
echo

# Exported (defined, global) dynamic symbols, names only.
syms () { nm -D --defined-only "$1" | awk '$2 ~ /^[TWDBRi]$/ {print $3}' | sort -u; }

c_tmp=$(mktemp); r_tmp=$(mktemp); trap 'rm -f "$c_tmp" "$r_tmp"' EXIT
syms "$C_SO"  > "$c_tmp"
syms "$R_SO"  > "$r_tmp"

echo "C exports   ($(wc -l < "$c_tmp")): $(paste -sd' ' < "$c_tmp")"
echo "Rust exports ($(wc -l < "$r_tmp")): $(paste -sd' ' < "$r_tmp")"
echo

missing=$(comm -23 "$c_tmp" "$r_tmp")
if [ -n "$missing" ]; then
  echo "MISSING FROM RUST .so:"; echo "$missing" | sed 's/^/  /'
else
  echo "MISSING FROM RUST .so: (none)"
fi

echo
echo "Undefined NON-libc symbols in the Rust .so:"
nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' \
 | grep -vE '^(_ITM_|__gmon_start__|__cxa_|_Unwind_|__tls_get_addr|__errno_location)' \
 | grep -vxE '(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev)' \
 | sed 's/^/  /' | sort -u > "$c_tmp".u
if [ -s "$c_tmp".u ]; then cat "$c_tmp".u; NONLIBC=1; else echo "  (none)"; NONLIBC=0; fi
rm -f "$c_tmp".u

echo
if [ -z "$missing" ] && [ "$NONLIBC" -eq 0 ]; then
  echo "SYMBOL PARITY: PASS (0 missing, 0 undefined non-libc)"; exit 0
else
  echo "SYMBOL PARITY: FAIL"; exit 1
fi
