#!/usr/bin/env bash
# Phase D helper: diff the dynamic symbol tables of the C and Rust shared libs.
#
# Every symbol exported (defined, dynamic) by the C .so must also be exported by
# the Rust .so under the exact same name.
set -uo pipefail
cd "$(dirname "$0")"

C_SO="c_src/build/libdriver.so"
R_SO="${1:-target/debug/libdriver.so}"

for f in "$C_SO" "$R_SO"; do
  [[ -f "$f" ]] || { echo "missing: $f" >&2; exit 2; }
done

# Defined, non-weak-undefined dynamic symbols; drop the ELF housekeeping ones
# that every shared object emits (they are not part of the library's API).
noise='^(_init|_fini|_edata|_end|__bss_start|__(gnu|odr)_.*|_ITM_.*|__cxa_.*|__gmon_start__)$'

exported() {
  nm -D --defined-only "$1" | awk '{print $3}' | grep -Ev "$noise" | sort -u
}

C_LIST="${TMPDIR:-/tmp}/symdiff_c.$$"
R_LIST="${TMPDIR:-/tmp}/symdiff_r.$$"
exported "$C_SO" > "$C_LIST"
exported "$R_SO" > "$R_LIST"

echo "C   exported symbols: $(wc -l < "$C_LIST")"
echo "Rust exported symbols: $(wc -l < "$R_LIST")"
echo
echo "=== symbols in C .so but MISSING from Rust .so ==="
missing=$(comm -23 "$C_LIST" "$R_LIST")
if [[ -z "$missing" ]]; then echo "(none)"; else echo "$missing"; fi
echo
echo "=== symbols only in Rust .so (extra; allowed but noted) ==="
extra=$(comm -13 "$C_LIST" "$R_LIST")
if [[ -z "$extra" ]]; then echo "(none)"; else echo "$extra"; fi
echo
echo "=== Rust .so undefined symbols that are NOT libc/libgcc ==="
# Anything the Rust .so imports must come from glibc / the GCC unwinder.
nonlibc=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' |
  grep -Ev '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^__cxa_|^_Unwind_|^gettid$|^statx$' | sort -u)
if [[ -z "$nonlibc" ]]; then echo "(none)"; else echo "$nonlibc"; fi

rm -f "$C_LIST" "$R_LIST"
[[ -z "$missing" && -z "$nonlibc" ]] || exit 1
echo
echo "RESULT: symbol parity OK (0 missing, 0 non-libc undefined)"
