#!/usr/bin/env bash
# Full verification run: build the C .so, then run every test under EVERY
# feature combination declared in Cargo.toml, in both the dev and release
# profile. Nothing here is hand-repeated per configuration.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
cd "$here" || exit 1

CARGO_FLAGS=(--offline)
TIMEOUT=${TIMEOUT:-600}
fail=0

# Logs go somewhere guaranteed writable (some sandboxes mount /tmp read-only).
LOGDIR="${TMPDIR:-$here/target}/verify-logs"
mkdir -p "$LOGDIR" || LOGDIR="$here/target"

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
find "$root/c_src/build" -maxdepth 1 -name '*.so' -printf '  built %p\n'

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations straight out of Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
    awk '
        /^\[features\]/ { inf=1; next }
        /^\[/           { inf=0 }
        inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
            split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
        }
    ' Cargo.toml | grep -v '^default$'
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
    echo "  Cargo.toml declares no [features] table."
    echo "  → the only combinations that exist are the default one and"
    echo "    --no-default-features (which is identical here)."
    COMBOS+=("DEFAULT")
    COMBOS+=("NODEFAULT")
else
    echo "  found features: ${FEATURES[*]}"
    COMBOS+=("DEFAULT")
    COMBOS+=("NODEFAULT")
    # every non-empty subset of the declared features
    n=${#FEATURES[@]}
    for ((mask = 1; mask < (1 << n); mask++)); do
        sel=()
        for ((i = 0; i < n; i++)); do
            (((mask >> i) & 1)) && sel+=("${FEATURES[i]}")
        done
        COMBOS+=("FEAT:$(
            IFS=,
            echo "${sel[*]}"
        )")
    done
    COMBOS+=("ALL")
fi
echo "  → ${#COMBOS[@]} combination(s): ${COMBOS[*]}"

combo_args() {
    case "$1" in
    DEFAULT) printf '%s' "" ;;
    NODEFAULT) printf '%s' "--no-default-features" ;;
    ALL) printf '%s' "--all-features" ;;
    FEAT:*) printf '%s' "--no-default-features --features ${1#FEAT:}" ;;
    esac
}

# ---------------------------------------------------------------------------
# 3. cargo check every combination first (fast failure)
# ---------------------------------------------------------------------------
step "cargo check, every combination"
for combo in "${COMBOS[@]}"; do
    args="$(combo_args "$combo")"
    printf '  %-28s' "$combo"
    # shellcheck disable=SC2086
    if timeout "$TIMEOUT" cargo check "${CARGO_FLAGS[@]}" $args --all-targets \
        >"$LOGDIR/check.log" 2>&1; then
        echo "ok"
    else
        echo "FAILED"
        tail -n 30 "$LOGDIR/check.log"
        fail=1
    fi
done

# ---------------------------------------------------------------------------
# 4. Build the cdylib + run the whole test suite for every combination,
#    in both profiles.
# ---------------------------------------------------------------------------
for profile in release dev; do
    prof_flag=""
    [[ $profile == release ]] && prof_flag="--release"
    for combo in "${COMBOS[@]}"; do
        args="$(combo_args "$combo")"
        step "profile=$profile combo=$combo"

        # The tests dlopen target/<profile>/libgen_ray_lib.so, so build it first
        # with the very same feature set.
        # shellcheck disable=SC2086
        if ! timeout "$TIMEOUT" cargo build "${CARGO_FLAGS[@]}" $prof_flag $args \
            >"$LOGDIR/build.log" 2>&1; then
            echo "  cdylib build FAILED"
            tail -n 30 "$LOGDIR/build.log"
            fail=1
            continue
        fi

        # shellcheck disable=SC2086
        if timeout "$TIMEOUT" cargo test "${CARGO_FLAGS[@]}" $prof_flag $args \
            >"$LOGDIR/test.log" 2>&1; then
            grep -E '^test result:' "$LOGDIR/test.log" | sed 's/^/  /'
        else
            echo "  TESTS FAILED"
            grep -E '^(test result:|---- |thread )' "$LOGDIR/test.log" \
                | head -n 60 | sed 's/^/  /'
            fail=1
        fi
    done
done

# ---------------------------------------------------------------------------
# 5. Symbol parity (both profiles' artifacts)
# ---------------------------------------------------------------------------
step "Symbol parity"
timeout "$TIMEOUT" cargo build "${CARGO_FLAGS[@]}" --release >/dev/null 2>&1
bash "$here/check_symbols.sh" || fail=1

# ---------------------------------------------------------------------------
# 6. Verification-of-the-verification (optional; set MUTATE=0 to skip)
# ---------------------------------------------------------------------------
if [[ "${MUTATE:-1}" == "1" ]]; then
    step "Mutation check (does the suite actually detect divergences?)"
    bash "$here/mutation_check.sh" | tail -n 6 || fail=1
fi

# ---------------------------------------------------------------------------
step "SUMMARY"
if [[ $fail -eq 0 ]]; then
    echo "ALL CONFIGURATIONS PASSED"
else
    echo "THERE WERE FAILURES"
fi
exit "$fail"
