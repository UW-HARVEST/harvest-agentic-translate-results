#!/usr/bin/env bash
# Runs the whole differential suite under EVERY Cargo feature combination.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so this
# keeps working if features are ever added.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

C_BUILD_DIR="../c_src/build"

echo "=== building the C reference shared library ==="
(
    cd ../c_src || exit 1
    mkdir -p build
    cd build || exit 1
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null || exit 1
    cmake --build . >/dev/null || exit 1
) || { echo "C build FAILED"; exit 1; }

C_SO="$(find "$C_BUILD_DIR" -maxdepth 1 -name 'lib*.so' | head -n1)"
if [[ -z "$C_SO" ]]; then
    echo "no C .so produced"; exit 1
fi
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate the declared features (everything in the [features] table except
# the implicit "default"), then build the power set of feature combinations.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inf = 1; next }
        /^\[/           { inf = 0 }
        inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "=");
            gsub(/[[:space:]]/, "", a[1]);
            if (a[1] != "default") print a[1];
        }
    ' Cargo.toml
)

echo "=== declared features: ${#FEATURES[@]} (${FEATURES[*]:-none}) ==="

# The combination list always contains the two baseline configurations.
COMBOS=("<default>" "<no-default-features>")

n=${#FEATURES[@]}
if (( n > 0 && n <= 16 )); then
    for (( mask = 1; mask < (1 << n); mask++ )); do
        combo=""
        for (( i = 0; i < n; i++ )); do
            if (( mask >> i & 1 )); then
                combo="${combo:+$combo,}${FEATURES[i]}"
            fi
        done
        COMBOS+=("$combo")
    done
fi

FAILED=0

run_case() {
    local label="$1"; shift
    echo
    echo "------------------------------------------------------------"
    echo ">>> configuration: $label"
    echo "------------------------------------------------------------"

    if ! timeout 600 cargo build --release "$@" 2>&1 | tail -n 3; then
        echo "!!! release build FAILED for $label"
        FAILED=1
        return
    fi
    if ! timeout 600 cargo build "$@" 2>&1 | tail -n 3; then
        echo "!!! debug build FAILED for $label"
        FAILED=1
        return
    fi
    if ! timeout 600 cargo test --release "$@" 2>&1 | tail -n 40; then
        echo "!!! tests FAILED for $label"
        FAILED=1
        return
    fi
    echo "<<< $label OK"
}

for combo in "${COMBOS[@]}"; do
    case "$combo" in
        "<default>")
            run_case "default features" ;;
        "<no-default-features>")
            run_case "--no-default-features" --no-default-features ;;
        *)
            run_case "--no-default-features --features $combo" \
                     --no-default-features --features "$combo" ;;
    esac
done

echo
echo "============================================================"
if (( FAILED )); then
    echo "RESULT: at least one configuration FAILED"
    exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) passed"

# ---------------------------------------------------------------------------
# Symbol parity, printed for the record.
# ---------------------------------------------------------------------------
echo
echo "=== symbol diff (C .so vs Rust .so) ==="
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u > /tmp/.c_syms.$$
nm -D --defined-only target/release/lib*.so | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u > /tmp/.r_syms.$$
missing="$(comm -23 /tmp/.c_syms.$$ /tmp/.r_syms.$$ | grep -vE '^(_init|_fini|__bss_start|_edata|_end)$')"
rm -f /tmp/.c_syms.$$ /tmp/.r_syms.$$
if [[ -n "$missing" ]]; then
    echo "MISSING from the Rust .so:"; echo "$missing"; exit 1
fi
echo "(empty — full parity)"
