#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs `cargo check` and
# `cargo test` for each one, so no configuration is verified by hand.
#
# Usage: ./verify_all_features.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- Build the C reference shared library (default CMake configuration) -------
C_SRC="../c_src"
if [[ ! -f "$C_SRC/build/libdriver.so" ]]; then
    echo "== building C reference library =="
    (mkdir -p "$C_SRC/build" && cd "$C_SRC/build" &&
        cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
        cmake --build . >/dev/null) || exit 1
fi

# --- Enumerate feature combinations ------------------------------------------
# Features are read straight out of Cargo.toml's [features] table so the list
# cannot drift from the manifest.
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/       { in_f = 1; next }
        /^\[/                 { in_f = 0 }
        in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "=");
            gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1];
        }
    ' Cargo.toml
)

COMBOS=("")  # the empty combination: --no-default-features
n=${#FEATURES[@]}
if (( n > 0 )); then
    for (( mask = 1; mask < (1 << n); mask++ )); do
        combo=""
        for (( i = 0; i < n; i++ )); do
            if (( mask & (1 << i) )); then
                combo="${combo:+$combo,}${FEATURES[$i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

echo "== ${#FEATURES[@]} feature(s) declared: ${FEATURES[*]:-<none>} =="
echo "== ${#COMBOS[@]} combination(s) to verify (plus the default set) =="

fail=0
run() {  # run <label> <cargo args...>
    local label="$1"; shift
    echo "-- $label"
    if ! timeout 600 "$@" >/tmp/verify.$$.log 2>&1; then
        echo "   FAILED: $label"
        tail -n 30 /tmp/verify.$$.log
        fail=1
    else
        grep -E "^test result:" /tmp/verify.$$.log | sed 's/^/   /'
    fi
    rm -f /tmp/verify.$$.log
}

for combo in "${COMBOS[@]}"; do
    label="${combo:-<no features>}"
    args=(--no-default-features)
    [[ -n "$combo" ]] && args+=(--features "$combo")
    run "check   [$label]" cargo check "${args[@]}" --all-targets
    run "test    [$label]" cargo test "${args[@]}"
done

# The default feature set as an ordinary user would build it.
run "check   [default]" cargo check --all-targets
run "test    [default]" cargo test
run "test    [default, release]" cargo test --release

if (( fail )); then
    echo "== FAILURES =="
    exit 1
fi
echo "== all combinations passed =="
