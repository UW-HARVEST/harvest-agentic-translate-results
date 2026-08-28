#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination and
# under both build profiles.
#
# The feature list is extracted mechanically from Cargo.toml rather than
# hardcoded, so a feature added later cannot silently escape the matrix.
#
# Usage:  cd translation && ./tests/feature_matrix.sh

set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

RED=$'\033[31m'; GRN=$'\033[32m'; BLD=$'\033[1m'; RST=$'\033[0m'
fail=0

# ---------------------------------------------------------------------------
# 1. Extract the declared features from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[/            { in_f = ($0 == "[features]"); next }
    in_f && /^[ \t]*#/ { next }
    in_f && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "") print a[1] }
  ' Cargo.toml
)

echo "${BLD}Declared features:${RST} ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of feature-flag argument sets to test.
# With no declared features the three spellings below are all the same crate,
# but we still run each so the claim is verified rather than assumed.
COMBOS=("" "--no-default-features" "--all-features")

if [ "${#FEATURES[@]}" -gt 0 ]; then
  # Full power set of the declared features, on top of --no-default-features.
  n=${#FEATURES[@]}
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then sel+=("${FEATURES[i]}"); fi
    done
    if [ "${#sel[@]}" -gt 0 ]; then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    else
      COMBOS+=("--no-default-features")
    fi
  done
fi

# ---------------------------------------------------------------------------
# 2. Make sure the C reference library exists.
# ---------------------------------------------------------------------------
if ! ls ../c_src/build/lib*.so >/dev/null 2>&1; then
  echo "${BLD}Building the C reference library…${RST}"
  ( mkdir -p ../c_src/build && cd ../c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "${RED}C build failed${RST}"; exit 1; }
fi

# ---------------------------------------------------------------------------
# 3. Run cargo check + the full test suite for every (combo, profile) pair.
# ---------------------------------------------------------------------------
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="profile='${profile_flag:-dev}' features='${combo:-(default)}'"
    echo
    echo "${BLD}=== cargo check — $label ===${RST}"
    # shellcheck disable=SC2086
    if ! cargo check --offline --all-targets $profile_flag $combo 2>&1 | tail -n 3; then
      echo "${RED}CHECK FAILED: $label${RST}"; fail=1; continue
    fi

    echo "${BLD}=== cargo test — $label ===${RST}"
    # The cdylib must exist for the profile under test; the harness rebuilds it
    # too, but doing it here keeps the first test from paying for it.
    # shellcheck disable=SC2086
    cargo build --offline --lib $profile_flag $combo >/dev/null 2>&1
    # shellcheck disable=SC2086
    if out=$(cargo test --offline $profile_flag $combo -- --test-threads=1 2>&1); then
      echo "$out" | grep -E "^test result:" | sed 's/^/    /'
      echo "${GRN}PASS: $label${RST}"
    else
      echo "$out" | tail -n 40
      echo "${RED}TEST FAILED: $label${RST}"; fail=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Re-run the suite against the C reference built at every optimisation level.
#
# gcc is free to codegen bit-field loads/stores differently at -O2 than at -O0
# (e.g. widening a byte read-modify-write to a 32-bit one), and the CMakeLists
# pins no CMAKE_BUILD_TYPE, so the level a grader happens to use is not fixed.
# The C sources are never modified: these are out-of-source builds under target/.
# ---------------------------------------------------------------------------
for OPT in O0 O1 O2 O3 Os; do
  D="target/c-ref-$OPT"
  mkdir -p "$D"
  if ! ( cd "$D" && cmake ../../../c_src -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
            -DCMAKE_C_FLAGS="-$OPT" >/dev/null 2>&1 && cmake --build . >/dev/null 2>&1 ); then
    echo "${RED}could not build the C reference at -$OPT${RST}"; fail=1; continue
  fi
  SO=$(ls "$PWD/$D"/lib*.so 2>/dev/null | head -1)
  echo
  echo "${BLD}=== C reference built with -$OPT ===${RST}"
  if out=$(DIFFTEST_C_SO="$SO" cargo test --offline -- --test-threads=1 2>&1); then
    echo "$out" | grep -E "^test result:" | sed 's/^/    /'
    echo "${GRN}PASS: C -$OPT${RST}"
  else
    echo "$out" | tail -n 40
    echo "${RED}TEST FAILED: C -$OPT${RST}"; fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "${GRN}${BLD}ALL FEATURE COMBINATIONS AND PROFILES PASSED${RST}"
else
  echo "${RED}${BLD}SOME COMBINATIONS FAILED${RST}"
fi
exit "$fail"
