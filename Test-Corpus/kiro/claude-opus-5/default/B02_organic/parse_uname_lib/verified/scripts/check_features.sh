#!/usr/bin/env bash
# Phase D — enumerate every feature combination declared in Cargo.toml and run
# cargo check / build / test / symbol-parity for each. Never hand-repeat.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t feats < <(
    awk '
        /^\[features\]/ { inf=1; next }
        /^\[/           { inf=0 }
        inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
    ' Cargo.toml | grep -v '^default$' | sort -u
)

echo "declared features: ${feats[*]:-(none)}"

# Build the list of combinations: default, no-default-features, and (if any
# features exist) every subset of the declared features.
combos=()
combos+=("DEFAULT")
combos+=("NONE")
n=${#feats[@]}
if (( n > 0 )); then
    for (( mask=1; mask < (1<<n); mask++ )); do
        sel=()
        for (( i=0; i<n; i++ )); do
            (( mask & (1<<i) )) && sel+=("${feats[$i]}")
        done
        combos+=("$(IFS=,; echo "${sel[*]}")")
    done
fi

echo "combinations to verify: ${#combos[@]}"

status=0
for combo in "${combos[@]}"; do
    case "$combo" in
        DEFAULT) args=() ; label="default" ;;
        NONE)    args=(--no-default-features) ; label="--no-default-features" ;;
        *)       args=(--no-default-features --features "$combo") ; label="--features $combo" ;;
    esac

    echo
    echo "############ combination: $label ############"

    if ! timeout 600 cargo check "${args[@]}" 2>&1 | tail -3; then
        echo "FAIL: cargo check failed for $label" >&2; status=1; continue
    fi
    if ! timeout 600 cargo build "${args[@]}" 2>&1 | tail -3; then
        echo "FAIL: cargo build failed for $label" >&2; status=1; continue
    fi
    if ! timeout 600 cargo build --release "${args[@]}" 2>&1 | tail -3; then
        echo "FAIL: cargo build --release failed for $label" >&2; status=1; continue
    fi
    for profile in debug release; do
        if ! ./scripts/symbols.sh "$profile" | tail -6; then
            echo "FAIL: symbol parity failed for $label ($profile)" >&2; status=1
        fi
    done
    if ! timeout 600 cargo test "${args[@]}" 2>&1 | grep -E "test result|^error"; then
        echo "FAIL: cargo test failed for $label" >&2; status=1
    fi
    if ! timeout 600 cargo test --release "${args[@]}" 2>&1 | grep -E "test result|^error"; then
        echo "FAIL: cargo test --release failed for $label" >&2; status=1
    fi
done

echo
if [[ $status -eq 0 ]]; then
    echo "ALL ${#combos[@]} FEATURE COMBINATION(S): OK"
else
    echo "SOME FEATURE COMBINATIONS FAILED" >&2
fi
exit $status
