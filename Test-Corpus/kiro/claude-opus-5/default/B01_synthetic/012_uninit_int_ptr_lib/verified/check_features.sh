#!/usr/bin/env bash
# Phase D: run the whole differential suite under EVERY cargo feature
# combination, for both the debug and the release profile.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are ever added.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

# --- enumerate features declared in Cargo.toml ------------------------------
mapfile -t features < <(
    awk '
        /^\[features\]/ { inside=1; next }
        /^\[/           { inside=0 }
        inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            sub(/[[:space:]]*=.*/, "");
            if ($0 != "default") print
        }
    ' Cargo.toml
)

n=${#features[@]}
echo "features declared in Cargo.toml: $n ${features[*]:-(none)}"

# --- build the list of combinations (powerset) ------------------------------
combos=()
if (( n == 0 )); then
    # No feature table: the default build is the only configuration. Run it both
    # ways so it is on record that --no-default-features is the same build.
    combos+=("--no-default-features" "")
else
    total=$(( 1 << n ))
    for (( mask = 0; mask < total; mask++ )); do
        sel=()
        for (( i = 0; i < n; i++ )); do
            if (( (mask >> i) & 1 )); then sel+=("${features[$i]}"); fi
        done
        if (( ${#sel[@]} == 0 )); then
            combos+=("--no-default-features")
        else
            combos+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
        fi
    done
    # Plus the default feature set as shipped.
    combos+=("")
fi

echo "configurations to verify: ${#combos[@]}"

# --- ensure the C reference library exists ----------------------------------
c_so="$(dirname "$here")/c_src/build/libdriver.so"
if [[ ! -f $c_so ]]; then
    echo "building the C reference library"
    ( cd "$(dirname "$here")/c_src" \
        && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /dev/null \
        && cmake --build . > /dev/null ) || exit 2
fi

rc=0
for profile in debug release; do
    rel_flag=""
    [[ $profile == release ]] && rel_flag="--release"
    for combo in "${combos[@]}"; do
        label="profile=$profile features='${combo:-<default>}'"
        echo
        echo "=============================================================="
        echo "  $label"
        echo "=============================================================="

        # shellcheck disable=SC2086
        if ! timeout 300 cargo build $rel_flag $combo 2>&1 | tail -3; then
            echo "BUILD FAILED: $label" >&2; rc=1; continue
        fi
        # shellcheck disable=SC2086
        if ! timeout 120 cargo clippy $rel_flag $combo --all-targets \
                 -- -D warnings > /dev/null 2>&1; then
            echo "  (clippy unavailable or reported findings; not fatal)"
        fi

        if ! ./check_symbols.sh "$profile"; then
            echo "SYMBOL PARITY FAILED: $label" >&2; rc=1
        fi

        # shellcheck disable=SC2086
        if ! timeout 600 cargo test $rel_flag $combo -- --test-threads=1 2>&1 \
                | grep -E '^(test result|running|error|test .* FAILED)'; then
            echo "TESTS FAILED: $label" >&2; rc=1
        fi
        # grep hides the real exit status, so re-check it explicitly.
        # shellcheck disable=SC2086
        if ! timeout 600 cargo test $rel_flag $combo -- --test-threads=1 \
                > /dev/null 2>&1; then
            echo "TESTS FAILED: $label" >&2; rc=1
        fi
    done
done

echo
if (( rc == 0 )); then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "SOME CONFIGURATIONS FAILED" >&2
fi
exit $rc
