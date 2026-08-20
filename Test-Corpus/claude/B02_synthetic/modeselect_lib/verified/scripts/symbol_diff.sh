#!/usr/bin/env bash
# Phase D: every symbol the C .so exports must also be exported by the Rust .so.
# Prints nothing and exits 0 when the diff is empty.
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

c_so=""
for cand in c_src/build/libtranslated_rust.so c_src/build/*.so; do
    [ -f "$cand" ] && c_so="$cand" && break
done
if [ -z "$c_so" ]; then
    echo "building the C reference library..." >&2
    (mkdir -p c_src/build && cd c_src/build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null) || exit 1
    c_so=$(ls c_src/build/*.so | head -1)
fi

cargo build --quiet || exit 1
rust_sos=(target/debug/libmodeselect_lib.so)
# whatever build.rs produced, too
while IFS= read -r p; do rust_sos+=("$p"); done < <(find target -name 'libmodeselect_lib_*.so' 2>/dev/null)

public() {
    nm -D --defined-only "$1" 2>/dev/null \
        | awk '$2 ~ /^[TDBRGS]$/ {print $3}' \
        | grep -Ev '^(_ZN|_R[A-Za-z0-9]|__rust|rust_|__rdl_|__rg_|_ITM_|__gnu|__cxa|_init|_fini|__bss|_edata|_end)' \
        | sort -u
}

c_syms=$(public "$c_so")
echo "C library:        $c_so ($(echo "$c_syms" | grep -c . ) public symbols)" >&2

for r in "${rust_sos[@]}"; do
    [ -f "$r" ] || continue
    r_all=$(nm -D --defined-only "$r" 2>/dev/null | awk '$2 ~ /^[TDBRGS]$/ {print $3}' | sort -u)
    missing=$(comm -23 <(echo "$c_syms") <(echo "$r_all"))
    if [ -n "$missing" ]; then
        echo "MISSING from $r:" >&2
        echo "$missing" >&2
        fail=1
    else
        echo "OK: $r exports all C symbols" >&2
    fi
done

exit $fail
