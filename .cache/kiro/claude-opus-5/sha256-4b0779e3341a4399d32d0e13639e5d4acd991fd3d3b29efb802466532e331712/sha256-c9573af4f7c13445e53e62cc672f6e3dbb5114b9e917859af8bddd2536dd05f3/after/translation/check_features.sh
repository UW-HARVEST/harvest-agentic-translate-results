#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so a
# feature added later is picked up automatically. With no [features] section
# there is exactly one configuration (the default), and `--no-default-features`
# is still exercised to prove it is equivalent.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t features < <(
    awk '
        /^\[features\]/ { inside=1; next }
        /^\[/           { inside=0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1]
        }
    ' Cargo.toml
)

echo "features declared in Cargo.toml: ${#features[@]} ${features[*]:-(none)}"

# Build the list of configurations: default, no-default, and every subset of the
# optional features on top of --no-default-features.
configs=("" "--no-default-features")
n=${#features[@]}
if [ "$n" -gt 0 ]; then
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${features[$i]}"
            fi
        done
        configs+=("--no-default-features --features $combo")
        configs+=("--features $combo")
    done
fi

rc=0
for cfg in "${configs[@]}"; do
    label="${cfg:-<default>}"
    echo
    echo "==================================================================="
    echo "CONFIG: $label"
    echo "==================================================================="

    # The cdylib the tests dlopen must be rebuilt for each configuration.
    if ! timeout 600 cargo build --release $cfg > /tmp/fc_build.log 2>&1; then
        echo "FAIL: cargo build --release $cfg"; tail -n 25 /tmp/fc_build.log; rc=1; continue
    fi
    if ! ./check_symbols.sh; then
        echo "FAIL: symbol parity under $label"; rc=1
    fi
    if ! timeout 600 cargo test $cfg -- --test-threads=4 > /tmp/fc_test.log 2>&1; then
        echo "FAIL: cargo test $cfg"; tail -n 40 /tmp/fc_test.log; rc=1; continue
    fi
    grep -E '^test result' /tmp/fc_test.log
done

echo
if [ $rc -eq 0 ]; then
    echo "ALL CONFIGURATIONS PASSED (${#configs[@]} configs)"
else
    echo "SOME CONFIGURATIONS FAILED"
fi
exit $rc
