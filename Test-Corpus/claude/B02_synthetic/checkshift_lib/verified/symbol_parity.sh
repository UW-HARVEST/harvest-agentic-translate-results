#!/usr/bin/env bash
# Phase D gate: every symbol exported by the C .so must also be exported by the
# Rust .so, and the Rust .so must import nothing outside libc/libgcc.
set -uo pipefail
cd "$(dirname "$0")" || exit 1

PROFILE=${1:-debug}
C_SO=c_src/build/libtranslated_rust.so
R_SO=target/$PROFILE/libcheckshift_lib.so

for f in "$C_SO" "$R_SO"; do
  [[ -f $f ]] || { echo "missing $f"; exit 1; }
done

tmp=${TMPDIR:-/tmp}
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > "$tmp/c.defined"
nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u > "$tmp/r.defined"

echo "== C exports ($(wc -l < "$tmp/c.defined")) =="
cat "$tmp/c.defined"
echo "== Rust exports ($(wc -l < "$tmp/r.defined")) =="
cat "$tmp/r.defined"

echo "== symbols in C but MISSING from Rust =="
missing=$(comm -23 "$tmp/c.defined" "$tmp/r.defined")
if [[ -n $missing ]]; then echo "$missing"; else echo "(none)"; fi

echo "== undefined non-libc symbols in Rust .so =="
# allow-list: glibc, libgcc unwinder, weak ITM/gmon/cxa hooks
bad=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u |
  grep -Ev '^(_Unwind_[A-Za-z]+|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|puts|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|write|writev|_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__)$')
if [[ -n $bad ]]; then echo "$bad"; else echo "(none)"; fi

rc=0
[[ -n $missing ]] && rc=1
[[ -n $bad ]] && rc=1
if (( rc == 0 )); then echo "SYMBOL PARITY: OK ($PROFILE)"; else echo "SYMBOL PARITY: FAILED ($PROFILE)"; fi
exit $rc
