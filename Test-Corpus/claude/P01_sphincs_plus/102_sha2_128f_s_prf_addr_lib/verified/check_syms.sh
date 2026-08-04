#!/usr/bin/env bash
# Compare exported symbols between C and Rust .so for every feature combo.
set +e
cd "$(dirname "$0")"

any_missing=0
for h in haraka sha2 shake blake; do
    for t in robust simple; do
        for s in 128s 128f 192s 192f 256s 256f; do
            cargo build --release --no-default-features --features "$h,$t,$s" 2>&1 > /tmp/build.out
            if [ ! -f target/release/libsphincs_plus.so ]; then
                echo "[$h/$t/$s] BUILD FAILED"
                cat /tmp/build.out
                any_missing=1
                continue
            fi
            build_dir="c_src/build_${h}_${t}_${s}"
            (nm -D --defined-only "$build_dir/app/libsphincs_core_det.so" 2>/dev/null
             nm -D --defined-only "$build_dir/lib/$h/lib$h.so" 2>/dev/null) \
                | awk '$2=="T" {print $3}' | sort -u > /tmp/c_syms.txt
            nm -D --defined-only target/release/libsphincs_plus.so 2>/dev/null \
                | awk '$2=="T" {print $3}' | sort -u > /tmp/rust_syms.txt
            missing=$(comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt | grep -vE '^(_init|_fini)$' | tr '\n' ' ')
            if [ -z "$missing" ]; then
                echo "[$h/$t/$s] OK"
            else
                echo "[$h/$t/$s] MISSING: $missing"
                any_missing=1
            fi
        done
    done
done
exit $any_missing
