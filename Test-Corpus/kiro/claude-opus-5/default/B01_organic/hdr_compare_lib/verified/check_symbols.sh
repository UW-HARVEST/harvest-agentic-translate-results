#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust .so is missing any symbol the C .so defines.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
c_so="$(ls "$root"/c_src/build/*.so)"
rs_so="$root/translation/target/release/libhdr_compare_lib.so"

[[ -f "$c_so"  ]] || { echo "missing C .so ($c_so)";  exit 1; }
[[ -f "$rs_so" ]] || { echo "missing Rust .so ($rs_so)"; exit 1; }

# Defined (exported) symbols only: nm -D type letter must be upper-case and not
# 'U' (undefined) or 'w'/'v' (weak).  Toolchain/CRT symbols are excluded because
# they are emitted by the linker, not by the translated source.
exclude='^(_ITM_(de)?registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|__gmon_start__|_init|_fini|__bss_start|_edata|_end|gettid|statx)$'

defined() {
  nm -D --defined-only "$1" 2>/dev/null \
    | awk '{print $NF}' | sort -u | grep -Ev "$exclude" | sort -u
}

undef_nonlibc() {
  # Undefined symbols that are NOT resolved from libc/libm/libgcc/ld.
  nm -D --undefined-only "$1" 2>/dev/null | awk '{print $NF}' | sed 's/@.*//' \
    | sort -u \
    | grep -Ev '^(_Unwind_|__|_ITM_|abort$|calloc$|malloc$|realloc$|free$|memcpy$|memmove$|memset$|bcmp$|strlen$|open64$|close$|read$|write$|writev$|lseek64$|fstat64$|stat64$|statx$|mmap64$|munmap$|getcwd$|getenv$|readlink$|realpath$|posix_memalign$|dl_iterate_phdr$|pthread_|syscall$|gettid$)'
}

c_defs="$(defined "$c_so")"
rs_defs="$(defined "$rs_so")"

echo "=== C  .so defined symbols ($c_so) ==="; echo "$c_defs"
echo "=== Rust .so defined symbols ($rs_so) ==="; echo "$rs_defs"

missing="$(comm -23 <(echo "$c_defs") <(echo "$rs_defs"))"
echo "=== symbols in C but MISSING from Rust ==="
echo "${missing:-(none)}"

leftover_undef="$(undef_nonlibc "$rs_so")"
echo "=== non-libc UNDEFINED symbols in the Rust .so ==="
echo "${leftover_undef:-(none)}"

rc=0
if [[ -n "$missing" ]]; then echo "FAIL: symbol diff is not empty"; rc=1; fi
if [[ -n "$leftover_undef" ]]; then echo "FAIL: unresolved non-libc symbols"; rc=1; fi
[[ $rc -eq 0 ]] && echo "PASS: symbol parity (0 missing, 0 non-libc undefined)"
exit $rc
