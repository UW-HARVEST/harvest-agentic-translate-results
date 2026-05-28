#!/usr/bin/env bash
# Run BOTH test files for every feature combo, count pass/fail per file.
set +e
cd "$(dirname "$0")"

total_pass=0
total_fail=0
fail_combos=""
for h in haraka sha2 shake blake; do
    for t in robust simple; do
        for s in 128s 128f 192s 192f 256s 256f; do
            combo="$h,$t,$s"
            output=$(timeout 600 cargo test --release --no-default-features --features "$combo" --tests 2>&1)
            ok_count=$(echo "$output" | grep -c "test result: ok\.")
            fail_count=$(echo "$output" | grep -cE "(test result: FAILED|FAILED \(allowed)")
            if [ "$ok_count" -ge "2" ] && [ "$fail_count" = "0" ]; then
                echo "[$combo] OK"
                total_pass=$((total_pass + 1))
            else
                echo "[$combo] FAIL (ok=$ok_count, fail=$fail_count)"
                total_fail=$((total_fail + 1))
                fail_combos="$fail_combos $combo"
            fi
        done
    done
done
echo "=== Total pass=$total_pass fail=$total_fail ==="
if [ -n "$fail_combos" ]; then
    echo "Failed:$fail_combos"
fi
