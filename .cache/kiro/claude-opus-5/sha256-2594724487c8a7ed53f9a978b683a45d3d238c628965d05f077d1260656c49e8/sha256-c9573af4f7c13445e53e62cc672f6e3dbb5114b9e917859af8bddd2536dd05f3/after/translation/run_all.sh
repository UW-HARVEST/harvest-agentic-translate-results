#!/usr/bin/env bash
# Differential verification driver.
#
# Two things this must guarantee, both of which are easy to get wrong:
#   1. `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so the
#      `.so` MUST be rebuilt explicitly before every test run. Otherwise the
#      suite diffs a stale library and passes for code that was never compiled.
#   2. Every feature combination must be covered. Combinations are extracted
#      from Cargo.toml rather than hard-coded.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
C_BUILD="$ROOT/../c_src/build"
TIMEOUT=${TIMEOUT:-600}
fails=0

# ---- C ground truth -------------------------------------------------------
if [[ ! -d $C_BUILD ]] || ! ls "$C_BUILD"/lib*.so >/dev/null 2>&1; then
  echo "== building C shared library =="
  mkdir -p "$C_BUILD"
  (cd "$C_BUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
C_SO=$(ls "$C_BUILD"/lib*.so | head -1)
echo "C  .so: $C_SO"

# ---- feature combinations -------------------------------------------------
# Powerset of the declared features, plus the default set. `awk` reads the
# [features] section of Cargo.toml; an absent section yields an empty list.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1])
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if ((${#FEATURES[@]} == 0)); then
  # No features declared: the default set and --no-default-features are the
  # same single configuration. Both are still run, to prove it.
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  COMBOS+=("default:")
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (((mask >> b) & 1)) && sel+=("${FEATURES[b]}")
    done
    joined=$(
      IFS=,
      echo "${sel[*]}"
    )
    COMBOS+=("nd[${joined:-none}]:--no-default-features --features=$joined")
  done
fi

echo "feature combinations: ${#COMBOS[@]}"

# ---- run ------------------------------------------------------------------
for profile in debug release; do
  pflag=""
  [[ $profile == release ]] && pflag="--release"
  for combo in "${COMBOS[@]}"; do
    label=${combo%%:*}
    flags=${combo#*:}
    echo
    echo "########## profile=$profile features=$label ##########"
    # shellcheck disable=SC2086
    if ! timeout "$TIMEOUT" cargo build $pflag $flags >/tmp/build.$$.log 2>&1; then
      echo "BUILD FAILED"; tail -20 /tmp/build.$$.log; fails=$((fails + 1)); continue
    fi
    SO="$ROOT/target/$profile/libhdr_bitrate_lib.so"
    [[ -f $SO ]] || { echo "MISSING $SO"; fails=$((fails + 1)); continue; }
    # shellcheck disable=SC2086
    if ! HDR_C_SO="$C_SO" HDR_RUST_SO="$SO" \
         timeout "$TIMEOUT" cargo test $pflag $flags 2>&1 | tail -4; then
      echo "TESTS FAILED"; fails=$((fails + 1))
    fi
  done
done

echo
if ((fails == 0)); then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "$fails CONFIGURATION(S) FAILED"
fi
exit $((fails > 0))
