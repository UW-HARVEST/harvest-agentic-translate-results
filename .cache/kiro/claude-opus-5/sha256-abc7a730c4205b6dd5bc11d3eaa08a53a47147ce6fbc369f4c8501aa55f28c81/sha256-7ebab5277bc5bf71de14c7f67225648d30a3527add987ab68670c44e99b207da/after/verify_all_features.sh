#!/usr/bin/env bash
# Enumerate every valid build-time configuration of the crate -- the cartesian
# product of the features declared in translation/Cargo.toml, times the cargo
# profiles -- and run a cargo command for each.
#
# Usage: ./verify_all_features.sh check
#        ./verify_all_features.sh test
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
CMD=${1:-check}

# ---------------------------------------------------------------------------
# The C shared library is the ground truth; make sure it is present.
# ---------------------------------------------------------------------------
if [[ ! -f "$ROOT/c_src/build/libdriver.so" ]]; then
  echo ">>> building c_src"
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "!!! C build failed"; exit 1; }
fi

cd "$ROOT/translation" || exit 1

# ---------------------------------------------------------------------------
# Feature names declared under [features], excluding the implicit "default".
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "declared features: ${FEATURES[*]:-<none>}"

FEATURE_COMBOS=("__default__")       # cargo's default feature set
n=${#FEATURES[@]}
if (( n > 0 )); then
  FEATURE_COMBOS+=("__none__")       # --no-default-features, nothing enabled
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then combo+="${combo:+,}${FEATURES[$i]}"; fi
    done
    FEATURE_COMBOS+=("$combo")
  done
fi

PROFILES=(dev release)

echo "feature combinations: ${#FEATURE_COMBOS[@]}  x  profiles: ${#PROFILES[@]}"

fail=0
for profile in "${PROFILES[@]}"; do
  for combo in "${FEATURE_COMBOS[@]}"; do
    case "$combo" in
      __default__) fargs=();                                        flabel="default features" ;;
      __none__)    fargs=(--no-default-features);                    flabel="no features" ;;
      *)           fargs=(--no-default-features --features "$combo"); flabel="$combo" ;;
    esac

    pargs=()
    [[ "$profile" != "dev" ]] && pargs=(--profile "$profile")

    echo "=============================================================="
    echo ">>> cargo $CMD [$profile] [$flabel]"

    # The crate is a pure cdylib, so cargo will not build it as a dependency of
    # the integration tests; the harness shells out to do that itself and needs
    # the same feature selection.
    export DRIVER_TEST_CARGO_ARGS="${fargs[*]}"

    if ! timeout 600 cargo "$CMD" "${pargs[@]}" "${fargs[@]}" 2>&1 | tail -n 40; then
      echo "!!! FAILED: cargo $CMD [$profile] [$flabel]"
      fail=1
    fi
  done
done

echo "=============================================================="
if (( fail )); then echo "RESULT: failures present"; else echo "RESULT: all configurations OK"; fi
exit $fail
