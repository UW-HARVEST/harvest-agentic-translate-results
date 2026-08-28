#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY cargo feature
# combination, rather than assuming the default one is the only one.
#
# The feature list is extracted from Cargo.toml at run time, so if a feature is
# ever added this script picks it up automatically instead of silently testing
# only the default build.
set -uo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname -- "$here")"
cd "$crate" || exit 1

# --- enumerate declared features -------------------------------------------
mapfile -t features < <(
    awk '
        /^\[features\]/ { inside = 1; next }
        /^\[/           { inside = 0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
        }
    ' Cargo.toml | grep -v '^default$' | sort -u
)

echo "declared non-default features: ${#features[@]} ${features[*]:-(none)}"

# Build the list of combinations to test: always the default build and the
# no-default-features build, plus the full power set of declared features.
combos=()
combos+=("DEFAULT")
combos+=("NONE")
n="${#features[@]}"
if [ "$n" -gt 0 ]; then
    if [ "$n" -gt 12 ]; then
        echo "FAIL: $n features means $((1 << n)) combinations; refusing to run blind." >&2
        exit 1
    fi
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=""
        for ((i = 0; i < n; i++)); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${features[i]}"
            fi
        done
        combos+=("$combo")
    done
fi

echo "combinations to verify: ${#combos[@]}"
echo

# The C side is identical for every combination; build it once.
root="$(dirname -- "$crate")"
if [ ! -d "$root/c_src/build" ]; then
    mkdir -p "$root/c_src/build"
    ( cd "$root/c_src/build" \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null )
fi

status=0
for combo in "${combos[@]}"; do
    case "$combo" in
        DEFAULT) args=() ;;
        NONE)    args=(--no-default-features) ;;
        *)       args=(--no-default-features --features "$combo") ;;
    esac

    for profile in debug release; do
        pargs=()
        [ "$profile" = "release" ] && pargs=(--release)

        label="features=$combo profile=$profile"
        echo "=== cargo check   $label"
        if ! timeout 600 cargo check "${args[@]}" "${pargs[@]}" --all-targets >/dev/null 2>&1; then
            echo "FAIL: cargo check failed for $label"
            status=1
            continue
        fi

        echo "=== cargo test    $label"
        log="$(mktemp)"
        if ! timeout 600 cargo test "${args[@]}" "${pargs[@]}" >"$log" 2>&1; then
            echo "FAIL: cargo test failed for $label"
            grep -E '^test .* FAILED|^test result|panicked|^error' "$log" | head -n 40
            rm -f "$log"
            status=1
            continue
        fi
        grep -E '^test result' "$log" | sed 's/^/    /'
        # Guard against a silently empty run.
        if ! grep -qE '^test result: ok\. [1-9]' "$log"; then
            echo "FAIL: $label ran no tests at all"
            status=1
        fi
        rm -f "$log"

        echo "=== symbol parity $label"
        if ! PROFILE="$profile" "$here/symbol_parity.sh"; then
            echo "FAIL: symbol parity failed for $label"
            status=1
        fi
        echo
    done
done

if [ "$status" -eq 0 ]; then
    echo "PASS: all ${#combos[@]} feature combination(s) verified in both profiles."
else
    echo "FAIL: at least one feature combination failed."
fi
exit "$status"
