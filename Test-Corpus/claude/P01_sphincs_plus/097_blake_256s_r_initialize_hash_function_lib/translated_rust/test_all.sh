#!/usr/bin/env bash
# Run cargo test against every cargo feature combination.
set +e
cd "$(dirname "$0")"

pass=0
fail=0
fail_combos=""
for h in haraka sha2 shake blake; do
    for t in robust simple; do
        for s in 128s 128f 192s 192f 256s 256f; do
            combo="$h,$t,$s"
            echo "===== $combo ====="
            timeout 600 cargo test --release --no-default-features --features "$combo" --test ffi_compare 2>&1 | tail -5
            if [ ${PIPESTATUS[0]} -eq 0 ]; then
                pass=$((pass+1))
            else
                fail=$((fail+1))
                fail_combos="$fail_combos $combo"
            fi
        done
    done
done
echo "==== SUMMARY: pass=$pass fail=$fail ===="
echo "Failed: $fail_combos"
