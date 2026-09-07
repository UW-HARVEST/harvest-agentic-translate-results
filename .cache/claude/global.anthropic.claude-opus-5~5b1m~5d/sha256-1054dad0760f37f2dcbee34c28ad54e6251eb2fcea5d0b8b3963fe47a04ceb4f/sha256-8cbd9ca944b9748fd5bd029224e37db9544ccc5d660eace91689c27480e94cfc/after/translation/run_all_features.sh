#!/usr/bin/env bash
# Build the C .so and the Rust cdylib, then run the differential suite under
# EVERY feature combination the crate has.
#
# `Cargo.toml` declares no `[features]` table and no optional dependencies, so
# the complete set of combinations is: default (empty), --no-default-features,
# and --all-features. This script enumerates them mechanically (from Cargo.toml)
# rather than hard-coding a list, so a future feature cannot be silently skipped.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
CARGO_FLAGS=(--offline)   # the sandbox has no crates.io egress; deps are vendored in ~/.cargo

fail=0
step() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

# --------------------------------------------------------------------------
step "Enumerating feature combinations declared in Cargo.toml"
declare -a FEATURES=()
while IFS= read -r f; do
    [[ -n "$f" ]] && FEATURES+=("$f")
done < <(awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
' "$HERE/Cargo.toml")

if [[ ${#FEATURES[@]} -eq 0 ]]; then
    echo "No [features] declared -> combinations are: default, --no-default-features, --all-features"
    declare -a COMBOS=("default" "--no-default-features" "--all-features")
else
    echo "Declared features: ${FEATURES[*]}"
    declare -a COMBOS=("default" "--no-default-features" "--all-features")
    # power set of the declared features, each with --no-default-features
    n=${#FEATURES[@]}
    for (( mask=1; mask < (1<<n); mask++ )); do
        combo=""
        for (( i=0; i<n; i++ )); do
            if (( mask & (1<<i) )); then
                combo+="${combo:+,}${FEATURES[i]}"
            fi
        done
        COMBOS+=("--no-default-features --features $combo")
    done
fi

# --------------------------------------------------------------------------
for profile in release debug; do
    PROFILE_FLAG=()
    [[ "$profile" == "release" ]] && PROFILE_FLAG=(--release)

    for combo in "${COMBOS[@]}"; do
        COMBO_FLAGS=()
        [[ "$combo" != "default" ]] && read -r -a COMBO_FLAGS <<< "$combo"

        step "cargo build [$profile] [$combo]  (produce the Rust .so under test)"
        ( cd "$HERE" && cargo build "${CARGO_FLAGS[@]}" "${PROFILE_FLAG[@]}" "${COMBO_FLAGS[@]}" ) \
            || { echo "BUILD FAILED: $profile / $combo"; fail=1; continue; }

        step "nm -D symbol parity [$profile] [$combo]"
        diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so"      | awk '{print $2, $3}' | sort) \
             <(nm -D --defined-only "$HERE/target/$profile/libdriver.so"  | awk '{print $2, $3}' | sort) \
            && echo "symbol diff EMPTY (ok)" \
            || { echo "SYMBOL DIFF NON-EMPTY: $profile / $combo"; fail=1; }

        step "cargo test [$profile] [$combo]"
        ( cd "$HERE" && cargo test "${CARGO_FLAGS[@]}" "${PROFILE_FLAG[@]}" "${COMBO_FLAGS[@]}" \
              -- --test-threads=1 ) \
            || { echo "TESTS FAILED: $profile / $combo"; fail=1; }
    done
done

step "SUMMARY"
if [[ $fail -eq 0 ]]; then
    echo "ALL feature combinations x profiles PASSED"
else
    echo "FAILURES were reported above"
fi
exit $fail
