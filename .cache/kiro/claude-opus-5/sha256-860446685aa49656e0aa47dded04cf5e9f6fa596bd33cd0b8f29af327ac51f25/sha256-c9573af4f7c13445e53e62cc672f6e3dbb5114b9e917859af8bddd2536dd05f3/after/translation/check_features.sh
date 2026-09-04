#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# The feature list is extracted from Cargo.toml rather than hard-coded, so the
# loop keeps working if features are added later. For each combination the
# cdylib is rebuilt (the tests load it through `libloading`, so it must be the
# artifact under test) and both phases are run.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
cd "$here"

# --- make sure the C reference exists -------------------------------------
if [[ ! -f "$root/c_src/build/libdriver.so" ]]; then
  (cd "$root/c_src" && mkdir -p build && cd build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null) || exit 1
fi

# --- enumerate the declared features -------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/           {inside=0}
    inside && /=/   {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of combinations to test: every subset of the declared features
# (with --no-default-features), plus the plain default build.
COMBOS=("DEFAULT")
n=${#FEATURES[@]}
if (( n > 0 )); then
  if (( n > 12 )); then
    echo "too many features for an exhaustive cross-product ($n); testing singletons + all"
    COMBOS+=("NONE")
    for f in "${FEATURES[@]}"; do COMBOS+=("$f"); done
    COMBOS+=("$(IFS=,; echo "${FEATURES[*]}")")
  else
    for ((mask=0; mask < (1<<n); mask++)); do
      sel=()
      for ((i=0; i<n; i++)); do (( mask & (1<<i) )) && sel+=("${FEATURES[i]}"); done
      if (( ${#sel[@]} == 0 )); then COMBOS+=("NONE")
      else COMBOS+=("$(IFS=,; echo "${sel[*]}")"); fi
    done
  fi
fi

fail=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) args=() ; label="(default features)" ;;
    NONE)    args=(--no-default-features) ; label="--no-default-features" ;;
    *)       args=(--no-default-features --features "$combo") ; label="--features $combo" ;;
  esac

  echo
  echo "==================================================================="
  echo "combination: $label"
  echo "==================================================================="

  if ! timeout 600 cargo build --release "${args[@]}" >/dev/null 2>&1; then
    echo "BUILD FAILED: $label"; fail=1; continue
  fi

  ./check_symbols.sh || fail=1

  if ! timeout 600 cargo test --release "${args[@]}" -- --test-threads=1; then
    echo "TESTS FAILED: $label"; fail=1
  fi
done

echo
if (( fail )); then echo "OVERALL: FAIL"; exit 1; fi
echo "OVERALL: PASS across ${#COMBOS[@]} combination(s)"
