#!/bin/bash
# For every (OP, REPEAT) combination, build the Rust cdylib and verify that
# every symbol exported by the corresponding C .so is also exported by the
# Rust .so (with the same name).
set -u
fail=0
for op in add sub mul; do
    for n in 0 1 2 3 4 5 6 7; do
        cargo build --release --no-default-features --features "${op},${n}" >/dev/null 2>&1
        c_syms=$(nm -D --defined-only "c_libs/lib_${op}_${n}.so" \
            | awk '{print $3}' \
            | grep -vE '^(_init|_fini|_edata|_end|__bss_start)$' \
            | sort -u)
        r_syms=$(nm -D --defined-only "target/release/libdriver.so" \
            | awk '{print $3}' \
            | sort -u)
        missing=""
        for sym in $c_syms; do
            if ! grep -qx -- "$sym" <<< "$r_syms"; then
                missing+=" $sym"
            fi
        done
        if [[ -n "$missing" ]]; then
            echo "*** MISSING in Rust .so for ${op},${n}:${missing}"
            fail=1
        else
            echo "OK ${op},${n}"
        fi
    done
done
if [[ $fail -ne 0 ]]; then
    exit 1
fi
echo "All symbol checks passed."
