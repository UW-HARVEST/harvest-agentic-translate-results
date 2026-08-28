#!/usr/bin/env bash
# Phase A / Phase D: mechanically diff the exported symbols of the two .so files.
# Exits non-zero if the Rust .so is missing anything the C .so exports.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | sort | head -n1)"
rust_so=""
for prof in release debug; do
    if [[ -f "$here/target/$prof/libgen_ray_lib.so" ]]; then
        rust_so="$here/target/$prof/libgen_ray_lib.so"
        break
    fi
done

if [[ -z "$c_so" ]]; then
    echo "ERROR: no C .so found under $root/c_src/build" >&2
    echo "  cd c_src && mkdir -p build && cd build && \\" >&2
    echo "  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ." >&2
    exit 1
fi
if [[ -z "$rust_so" ]]; then
    echo "ERROR: no Rust .so found under $here/target/{release,debug}" >&2
    exit 1
fi

echo "C    .so: $c_so"
echo "Rust .so: $rust_so"
echo

# The C runtime / PIC boilerplate the C .so exports but which is not API.
noise='^(_init|_fini|__bss_start|_edata|_end|call_weak_fn|deregister_tm_clones|register_tm_clones|__do_global_dtors_aux|frame_dummy|_dl_relocate_static_pie)$'

syms() { nm -D --defined-only "$1" | awk '{print $NF, $(NF-1)}' \
    | awk '$2=="T"||$2=="t"||$2=="W"||$2=="i" {print $1}' | sort -u; }

c_syms="$(syms "$c_so" | grep -Ev "$noise")"
r_syms="$(syms "$rust_so")"

n_c=$(printf '%s\n' "$c_syms" | grep -c . || true)
n_r=$(printf '%s\n' "$r_syms" | grep -c . || true)

echo "C .so exports $n_c API symbol(s); Rust .so defines $n_r symbol(s)."
echo
echo "=== Symbols in the C .so that are MISSING from the Rust .so ==="
missing="$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))"
if [[ -z "$missing" ]]; then
    echo "(none — diff is EMPTY)"
else
    printf '%s\n' "$missing"
fi

echo
echo "=== Undefined (imported) symbols, C .so ==="
nm -D -u "$c_so" || true

echo
echo "=== Undefined non-libc symbols, Rust .so ==="
nm -D -u "$rust_so" \
    | grep -Ev 'GLIBC|GCC_|_ITM_|__gmon_start__|__cxa|__tls_get_addr|ld-linux' \
    || echo "(none)"

if [[ -n "$missing" ]]; then
    echo
    echo "FAIL: symbol diff is NOT empty."
    exit 1
fi
echo
echo "PASS: symbol diff is empty."
