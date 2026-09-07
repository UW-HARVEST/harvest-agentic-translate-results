#!/usr/bin/env bash
# Phase D — enumerate every feature combination declared in Cargo.toml and run
# the full differential suite under each one. Nothing is hardcoded: the feature
# list is extracted from the manifest, so a future [features] table is picked up
# automatically.
set -uo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inf = 1; next }
        /^\[/           { inf = 0 }
        inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1]
        }
    ' Cargo.toml
)

echo "Declared features: ${FEATURES[*]:-(none)}"
n=${#FEATURES[@]}
echo "Feature count: $n  =>  $((1 << n)) subset(s) + default"
echo

run() {
    local label="$1"; shift
    echo "======================================================================"
    echo ">>> $label"
    echo ">>> cargo test --release --no-fail-fast $*"
    echo "======================================================================"
    # The cdylib must be rebuilt for the same feature set the tests will load.
    if ! timeout 600 cargo build --release --lib "$@" 2>&1 | tail -3; then
        echo "BUILD FAILED: $label"; return 1
    fi
    if ! timeout 600 cargo test --release --no-fail-fast "$@" 2>&1 | grep -E '^test [a-z0-9_]+ \.\.\.|test result:|^error'; then
        echo "TEST RUN PRODUCED NO RECOGNISED OUTPUT: $label"; return 1
    fi
    if timeout 600 cargo test --release --no-fail-fast "$@" 2>&1 | grep -q 'test result: FAILED'; then
        echo "FAILED: $label"; return 1
    fi
    echo "OK: $label"
    echo
}

status=0

run "default features" || status=1
run "no default features" --no-default-features || status=1

# Every subset of the declared non-default features, on top of --no-default-features.
if (( n > 0 )); then
    for (( mask = 0; mask < (1 << n); mask++ )); do
        combo=()
        for (( i = 0; i < n; i++ )); do
            (( mask & (1 << i) )) && combo+=("${FEATURES[i]}")
        done
        joined="$(IFS=,; echo "${combo[*]}")"
        if [[ -z "$joined" ]]; then
            continue  # already covered by --no-default-features above
        fi
        run "combo: $joined" --no-default-features --features "$joined" || status=1
    done
fi

echo "======================================================================"
if [[ $status -eq 0 ]]; then
    echo "ALL FEATURE COMBINATIONS PASSED"
else
    echo "AT LEAST ONE FEATURE COMBINATION FAILED"
fi
exit $status
