#!/usr/bin/env bash
# Phase D helper: enumerate every cargo feature combination of the crate and run
# `cargo check` plus the differential suite for each one.
#
# The crate declares no `[features]` and contains no `cfg(feature = ...)`, so the
# power set is the single empty combination. The script derives that mechanically
# rather than asserting it, so it stays correct if features are added later.
set -uo pipefail
cd "$(dirname "$0")/.."

manifest_features() {
    # Feature names from the [features] table, plus implicit features from
    # optional dependencies.
    awk '
        /^\[features\]/      { inf = 1; next }
        /^\[/                { inf = 0 }
        inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); if (a[1] != "default") print a[1] }
    ' Cargo.toml
    awk '/optional[[:space:]]*=[[:space:]]*true/ { print }' Cargo.toml >/dev/null
}

mapfile -t FEATURES < <(manifest_features | sort -u)
n=${#FEATURES[@]}
echo "declared features: $n ${FEATURES[*]:-<none>}"
if grep -rqn 'cfg(feature' src/ tests/ 2>/dev/null; then
    echo "WARNING: cfg(feature = ...) found in sources but not enumerated above"
fi

fail=0
total=$(( 1 << n ))
for (( mask = 0; mask < total; mask++ )); do
    combo=()
    for (( i = 0; i < n; i++ )); do
        (( mask & (1 << i) )) && combo+=("${FEATURES[$i]}")
    done
    joined=$(IFS=,; echo "${combo[*]}")
    label=${joined:-"(default/none)"}
    echo "=== combination: $label ==="

    if ! timeout 600 cargo check --release --no-default-features \
            ${joined:+--features "$joined"} >/dev/null 2>&1; then
        echo "  cargo check FAILED"; fail=1; continue
    fi
    if ! timeout 600 cargo build --release --no-default-features \
            ${joined:+--features "$joined"} >/dev/null 2>&1; then
        echo "  cargo build FAILED"; fail=1; continue
    fi

    # Symbol parity for this combination.
    nm -D --defined-only ../c_src/build/libpng.so \
        | awk '$2~/^[TDBR]$/{print $3}' | sort -u > /tmp/.pd_c.$$
    nm -D --defined-only target/release/liblibpng.so \
        | awk '$2~/^[TDBR]$/{print $3}' | sort -u > /tmp/.pd_r.$$
    missing=$(comm -23 /tmp/.pd_c.$$ /tmp/.pd_r.$$ | wc -l)
    extra=$(comm -13 /tmp/.pd_c.$$ /tmp/.pd_r.$$ | wc -l)
    echo "  symbols: C=$(wc -l < /tmp/.pd_c.$$) Rust=$(wc -l < /tmp/.pd_r.$$) missing=$missing extra=$extra"
    [ "$missing" = 0 ] || fail=1
    rm -f /tmp/.pd_c.$$ /tmp/.pd_r.$$

    if timeout 3000 cargo test --release --no-default-features \
            ${joined:+--features "$joined"} --test diff 2>&1 \
            | grep -E 'cases passed|rows covered|diverged'; then
        :
    else
        echo "  differential suite FAILED to run"; fail=1
    fi
done

echo
if [ "$fail" = 0 ]; then
    echo "ALL FEATURE COMBINATIONS OK ($total combination(s))"
else
    echo "FAILURES PRESENT"
fi
exit "$fail"
