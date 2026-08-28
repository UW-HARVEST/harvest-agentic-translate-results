#!/usr/bin/env bash
# Build the C reference library, enumerate every valid Cargo feature
# combination, and run the differential FFI tests for each one.
#
# The crate currently declares no [features], so the only valid combination is
# the empty set; the loop below still derives the list from Cargo.toml so that
# it keeps working if features are added later.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# ---------------------------------------------------------------- C reference
if [[ ! -d ../c_src/build ]] || ! compgen -G '../c_src/build/*.so' >/dev/null; then
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null)
fi
c_so="$(ls ../c_src/build/lib*.so | head -n 1)"
echo "C reference: $c_so"

# Extra C builds at different optimisation levels (does not touch c_src/).
extra=()
for opt in O0 O1 O2 O3 Ofast; do
    out="/tmp/ldexp_q2_c_${opt}.so"
    gcc "-${opt}" -fPIC -shared -I../c_src/include -o "$out" ../c_src/src/lib.c
    extra+=("$out")
done
export LDEXP_Q2_EXTRA_C_LIBS="$(IFS=:; echo "${extra[*]}")"

# ------------------------------------------------------- feature enumeration
mapfile -t features < <(
    awk '
        /^\[features\]/ { in_f = 1; next }
        /^\[/           { in_f = 0 }
        in_f && /=/     { sub(/=.*/, ""); gsub(/[ \t]/, ""); if ($0 != "" && $0 != "default") print }
    ' Cargo.toml
)

combos=("")
if ((${#features[@]} > 0)); then
    n=${#features[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        combo=()
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && combo+=("${features[i]}")
        done
        combos+=("$(IFS=,; echo "${combo[*]}")")
    done
fi

echo "Feature combinations: ${#combos[@]}"

# ------------------------------------------------------------------ run them
for combo in "${combos[@]}"; do
    label="${combo:-<none>}"
    args=(--no-default-features)
    [[ -n "$combo" ]] && args+=(--features "$combo")

    echo "=== cargo check ${label} ==="
    timeout 600 cargo check "${args[@]}" --all-targets

    for profile in dev release; do
        flag=(); [[ $profile == release ]] && flag=(--release)
        echo "=== cargo test ${label} (${profile}) ==="
        # cdylib is not an automatic dependency of integration tests, so build
        # it explicitly before the tests try to dlopen it.
        timeout 600 cargo build "${args[@]}" "${flag[@]}"
        timeout 600 cargo test "${args[@]}" "${flag[@]}"
    done
done

# ------------------------------------------------------------ symbol parity
echo "=== symbol parity ==="
rust_so="target/release/libldexp_q2_lib.so"
c_syms="$(nm -D --defined-only "$c_so" | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u)"
rust_syms="$(nm -D --defined-only "$rust_so" | awk '$2 ~ /^[TDBR]$/ {print $3}' | sort -u)"
missing="$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))"
if [[ -n "$missing" ]]; then
    echo "MISSING from Rust .so:"
    echo "$missing"
    exit 1
fi
echo "all C-exported symbols present in the Rust .so:"
echo "$c_syms" | sed 's/^/  /'

echo
echo "ALL FEATURE COMBINATIONS PASSED"
