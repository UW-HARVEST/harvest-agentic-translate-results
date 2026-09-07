#!/usr/bin/env bash
# Phase D: enumerate every cargo feature combination declared in Cargo.toml and
# run cargo check + the full differential suite under each one.
#
# The crate declares no [features] table, so the enumeration below yields the
# single default configuration; the loop is written generically so that adding a
# feature automatically widens the sweep instead of silently going untested.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
cd "$crate"

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t features < <(
    awk '
        /^\[features\]/ { inside = 1; next }
        /^\[/           { inside = 0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1];
        }
    ' Cargo.toml
)

echo "declared non-default features: ${#features[@]} ${features[*]:-(none)}"

# Build the combination list: the default build, plus --no-default-features with
# every subset of the declared features (powerset).
combos=("default")
if (( ${#features[@]} > 0 )); then
    n=${#features[@]}
    for (( mask = 0; mask < (1 << n); mask++ )); do
        set=()
        for (( i = 0; i < n; i++ )); do
            (( mask & (1 << i) )) && set+=("${features[$i]}")
        done
        combos+=("nodefault:$(IFS=,; echo "${set[*]:-}")")
    done
fi

echo "combinations to verify: ${#combos[@]}"

fail=0
for combo in "${combos[@]}"; do
    if [[ "$combo" == "default" ]]; then
        args=()
        label="default"
    else
        feats="${combo#nodefault:}"
        args=(--no-default-features)
        [[ -n "$feats" ]] && args+=(--features "$feats")
        label="--no-default-features${feats:+ --features $feats}"
    fi

    echo "=============================================================="
    echo "combination: $label"
    echo "=============================================================="

    if ! timeout 300 cargo check "${args[@]}" >/tmp/fc_check.log 2>&1; then
        echo "cargo check FAILED for $label" >&2; tail -20 /tmp/fc_check.log >&2; fail=1; continue
    fi
    if ! timeout 300 cargo build --release "${args[@]}" >/tmp/fc_build.log 2>&1; then
        echo "cargo build FAILED for $label" >&2; tail -20 /tmp/fc_build.log >&2; fail=1; continue
    fi
    if ! ./scripts/symbol_diff.sh >/tmp/fc_sym.log 2>&1; then
        echo "symbol diff FAILED for $label" >&2; cat /tmp/fc_sym.log >&2; fail=1; continue
    fi
    if ! timeout 600 cargo test --test differential "${args[@]}" 2>&1 | tail -5; then
        echo "differential suite FAILED for $label" >&2; fail=1; continue
    fi
    if ! ./scripts/compare_binaries.sh; then
        echo "binary comparison FAILED for $label" >&2; fail=1; continue
    fi
    echo "combination $label: OK"
done

if (( fail )); then
    echo "FEATURE SWEEP FAILED" >&2
    exit 1
fi
echo "FEATURE SWEEP OK across ${#combos[@]} combination(s)"
