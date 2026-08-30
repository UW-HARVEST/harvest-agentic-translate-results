#!/usr/bin/env bash
# Full verification sweep: builds the C reference, then runs the differential
# suite for every feature combination and every profile.
#
#   ./verify.sh
#
# `translation/Cargo.toml` declares no `[features]` table, so the complete set of
# feature combinations is {default, --no-default-features, --all-features}; the
# list below is derived from the manifest rather than hard-coded, so it stays
# correct if features are ever added.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
CARGO_FLAGS="${CARGO_FLAGS:---offline}"
FAILED=0

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
fail() { printf '\033[31mFAIL: %s\033[0m\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. C reference library
# ---------------------------------------------------------------------------
step "Building the C reference library"
mkdir -p "$ROOT/c_src/build" || exit 1
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { fail "C build"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so" || exit 1

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from the manifest
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/ *=.*/,""); gsub(/ /,""); if ($0 != "") print}' Cargo.toml
)

COMBOS=("")                                   # default
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  echo "declared features: ${FEATURES[*]}"
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  # all pairs
  for i in "${!FEATURES[@]}"; do
    for j in "${!FEATURES[@]}"; do
      [ "$i" -lt "$j" ] || continue
      COMBOS+=("--no-default-features --features ${FEATURES[$i]},${FEATURES[$j]}")
    done
  done
else
  echo "Cargo.toml declares no [features]; combos = default / none / all"
fi

# ---------------------------------------------------------------------------
# 3. cargo check for every combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  step "cargo check --tests ${combo:-(default)}"
  # shellcheck disable=SC2086
  cargo check $CARGO_FLAGS --tests $combo 2>&1 | tail -n 3 \
    || fail "cargo check $combo"
done

# ---------------------------------------------------------------------------
# 4. Full differential suite for every combination x profile
# ---------------------------------------------------------------------------
for profile_flag in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="${combo:-(default)} ${profile_flag:-(debug)}"
    step "cargo test $label"
    # The `.so` under test must exist for the profile being exercised.
    # shellcheck disable=SC2086
    cargo build $CARGO_FLAGS $combo $profile_flag 2>&1 | tail -n 2
    # fd 1 is redirected process-wide by the harness, so tests are serialised.
    # shellcheck disable=SC2086
    cargo test $CARGO_FLAGS $combo $profile_flag -- --test-threads=1 2>&1 \
      | grep -E '^(test |error|running|test result|warning: unused)' \
      | grep -vE '^test [a-z_0-9]+ \.\.\. ok$'
    # shellcheck disable=SC2086
    cargo test $CARGO_FLAGS $combo $profile_flag -- --test-threads=1 >/dev/null 2>&1 \
      || fail "cargo test $label"
  done
done

step "SUMMARY"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL COMBINATIONS PASSED\033[0m\n'
else
  printf '\033[31mSOME COMBINATIONS FAILED\033[0m\n'
fi
exit "$FAILED"
