#!/bin/bash
# Runs `cargo check`, `cargo build` and `cargo test` for every valid feature
# combination of this crate.
#
# The crate has no `[features]` section and no optional dependencies (and the C
# side has no `option()` / `#ifdef` either - see CONFIGS.md), so the complete
# matrix is the three equivalent ways of spelling "the one configuration".  The
# feature list is read out of Cargo.toml mechanically, so the matrix grows by
# itself if a feature is ever added.
set -u
cd "$(dirname "$0")" || exit 1

features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{sub(/ *=.*/,"");print}' Cargo.toml)
if [ -n "$features" ]; then
    echo "declared features: $(echo "$features" | paste -sd' ')"
else
    echo "declared features: (none)"
fi

combos=("" "--no-default-features" "--all-features")
for f in $features; do
    combos+=("--no-default-features --features $f")
done
if [ -n "$features" ]; then
    combos+=("--no-default-features --features $(echo "$features" | paste -sd,)")
fi

./build_c_lib.sh > /dev/null || exit 1

log=$(mktemp)
fail=0
for combo in "${combos[@]}"; do
    label="${combo:-<default features>}"
    for cmd in check build test; do
        printf '=== cargo %s %s ... ' "$cmd" "$label"
        # shellcheck disable=SC2086
        if cargo $cmd $combo > "$log" 2>&1; then
            echo "OK"
            grep -E "^test [a-z_]+ \.\.\." "$log" | sed 's/^/      /'
        else
            echo "FAILED"
            fail=1
            tail -n 40 "$log" | sed 's/^/      /'
        fi
    done
done
rm -f "$log"

if [ "$fail" -ne 0 ]; then
    echo "SOME COMBINATIONS FAILED"
    exit 1
fi
echo "ALL FEATURE COMBINATIONS OK"
