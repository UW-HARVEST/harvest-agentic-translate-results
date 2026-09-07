#!/usr/bin/env bash
# Phase D — exported-symbol parity between the C and Rust shared objects.
# Exits non-zero if the C .so defines any symbol the Rust .so does not.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
rs_so="$root/translation/target/release/libcolourblind_lib.so"

for f in "$c_so" "$rs_so"; do
    [[ -f "$f" ]] || { echo "MISSING: $f" >&2; exit 2; }
done

echo "C    .so: $c_so"
echo "Rust .so: $rs_so"
echo

# Defined (uppercase type letter) dynamic symbols only. Strip @GLIBC versions.
defined() { nm -D --defined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u; }
# All dynamic symbols, for the informational side of the report.
allsyms() { nm -D "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u; }

c_def="$(defined "$c_so")"
rs_def="$(defined "$rs_so")"

echo "=== C .so defined symbols ($(wc -l <<<"$c_def")) ==="
echo "$c_def"
echo
echo "=== Rust .so defined symbols ($(wc -l <<<"$rs_def")) ==="
echo "$rs_def"
echo

# ELF/CRT boilerplate present in every shared object; not library API.
boiler='^(_init|_fini|_edata|_end|__bss_start|__gmon_start__|_ITM_(de)?registerTMCloneTable|__cxa_finalize|__cxa_thread_atexit_impl|gettid|statx)$'

missing="$(comm -23 <(echo "$c_def") <(echo "$rs_def") | grep -Ev "$boiler" || true)"

echo "=== Symbols defined by C .so but MISSING from Rust .so (excl. boilerplate) ==="
if [[ -z "$missing" ]]; then
    echo "(none)"
else
    echo "$missing"
fi
echo

# Undefined symbols in the Rust .so that are not resolvable from libc/libgcc.
echo "=== Rust .so undefined symbols not satisfied by libc/libgcc_s ==="
unresolved="$(ldd -r "$rs_so" 2>&1 | grep -i 'undefined symbol' || true)"
if [[ -z "$unresolved" ]]; then
    echo "(none)"
else
    echo "$unresolved"
fi
echo

status=0
[[ -n "$missing" ]] && { echo "FAIL: Rust .so is missing $(wc -l <<<"$missing") C symbol(s)"; status=1; }
[[ -n "$unresolved" ]] && { echo "FAIL: Rust .so has unresolved non-libc symbols"; status=1; }
[[ $status -eq 0 ]] && echo "PASS: symbol diff is empty."
exit $status
