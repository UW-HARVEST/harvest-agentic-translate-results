#!/usr/bin/env bash
# Differential verification of translation/ against c_src/, across every
# build-time configuration.
#
#   1. build the C shared library
#   2. enumerate every valid Cargo feature combination (the powerset of the
#      features declared in translation/Cargo.toml)
#   3. for each combination: cargo check, cargo build, cargo test
#   4. repeat the build + test for the release profile, whose optimisation
#      settings and `panic = "abort"` differ from the dev profile
#
# Usage: ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$ROOT/translation"
TIMEOUT=600
FAILURES=0

step() { printf '\n=== %s ===\n' "$*"; }

run() {
  local desc="$1"; shift
  printf -- '--- %s\n' "$desc"
  if timeout "$TIMEOUT" "$@" > /tmp/verify.log 2>&1; then
    tail -n 3 /tmp/verify.log
  else
    FAILURES=$((FAILURES + 1))
    printf 'FAILED: %s\n' "$desc"
    tail -n 40 /tmp/verify.log
  fi
}

# ---------------------------------------------------------------- C library ---
step "Building C shared library"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" &&
    timeout "$TIMEOUT" cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > /tmp/cmake.log 2>&1 &&
    timeout "$TIMEOUT" cmake --build . >> /tmp/cmake.log 2>&1
) || { echo "C build FAILED"; tail -n 40 /tmp/cmake.log; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
echo "built $C_SO"

# --------------------------------------------------------- feature powerset ---
# Every name in the [features] table of Cargo.toml. `driver` declares none, so
# the powerset is just the empty combination; the loop is written generically so
# it keeps working if features are added later.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      gsub(/[[:space:]]/, "");
      if ($0 != "default") print
    }
  ' "$CRATE/Cargo.toml"
)

step "Feature enumeration"
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "translation/Cargo.toml declares no [features]"
else
  printf 'declared features: %s\n' "${FEATURES[*]}"
fi

# Build the powerset as comma-separated feature lists ("" = no features).
COMBOS=("")
for f in "${FEATURES[@]}"; do
  existing=("${COMBOS[@]}")
  for c in "${existing[@]}"; do
    if [ -z "$c" ]; then COMBOS+=("$f"); else COMBOS+=("$c,$f"); fi
  done
done
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '  [%s]\n' "${c:-<none>}"; done

# ------------------------------------------------------------------- verify ---
cd "$CRATE" || exit 1

for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  feat_args=(--no-default-features)
  [ -n "$combo" ] && feat_args+=(--features "$combo")

  step "Feature combination: $label"
  run "cargo check   [$label]" cargo check "${feat_args[@]}" --all-targets
  run "cargo build   [$label] (dev)" cargo build "${feat_args[@]}"
  run "cargo test    [$label] (dev)" cargo test "${feat_args[@]}"
  run "cargo build   [$label] (release)" cargo build --release "${feat_args[@]}"
  run "cargo test    [$label] (release)" cargo test --release "${feat_args[@]}"

  # Symbol parity, checked directly as well as from inside the test suite.
  for profile in debug release; do
    rust_so="$CRATE/target/$profile/libdriver.so"
    if [ ! -f "$rust_so" ]; then
      echo "FAILED: $rust_so missing [$label/$profile]"
      FAILURES=$((FAILURES + 1))
      continue
    fi
    missing="$(comm -23 \
      <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u) \
      <(nm -D --defined-only "$rust_so" | awk '{print $3}' | sort -u))"
    if [ -n "$missing" ]; then
      echo "FAILED: symbols exported by C but not Rust [$label/$profile]:"
      echo "$missing"
      FAILURES=$((FAILURES + 1))
    else
      echo "symbol parity OK [$label/$profile]"
    fi
  done
done

step "Summary"
if [ "$FAILURES" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "$FAILURES step(s) FAILED"
fi
exit "$FAILURES"
