#!/usr/bin/env bash
# Run the full differential test suite under EVERY valid feature combination.
#
#   ./run_all_features.sh
#
# Combinations are derived mechanically from the `[features]` table in
# Cargo.toml: the power set of the non-`default` features, plus the `default`
# combination itself.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$PWD
CARGO_ARGS=(--offline)
LOG="${TMPDIR:-/tmp}/run_all_features.log"

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
CSO="$ROOT/c_src/build/libtranslated_rust.so"
if [ ! -f "$CSO" ]; then
  echo "== building C shared library =="
  mkdir -p "$ROOT/c_src/build" || exit 1
  ( cd "$ROOT/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "C  .so: $CSO"
nm -D --defined-only "$CSO" | sed 's/^/    /'

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations.
# ---------------------------------------------------------------------------
mapfile -t OPTIONAL < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }' Cargo.toml
)

COMBOS=()
N=${#OPTIONAL[@]}
for (( mask = 0; mask < (1 << N); mask++ )); do
  combo=""
  for (( b = 0; b < N; b++ )); do
    if (( mask & (1 << b) )); then
      combo="${combo:+$combo,}${OPTIONAL[b]}"
    fi
  done
  COMBOS+=("$combo")
done
# The `default` feature set is a valid configuration in its own right.
if grep -qE '^default[[:space:]]*=' Cargo.toml; then
  COMBOS+=("default")
fi

echo
echo "== feature combinations (${#COMBOS[@]}) =="
for c in "${COMBOS[@]}"; do echo "    '${c}'"; done

# ---------------------------------------------------------------------------
# 3. cargo check + build + test for each combination.
# ---------------------------------------------------------------------------
FAILED=0
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    FEAT=()
    label="<none>"
  else
    FEAT=(--features "$combo")
    label="$combo"
  fi
  echo
  echo "############################################################"
  echo "# features: $label"
  echo "############################################################"

  for step in check build; do
    if ! timeout 600 cargo "$step" "${CARGO_ARGS[@]}" --no-default-features "${FEAT[@]}" \
           >"$LOG" 2>&1; then
      echo "!! cargo $step FAILED for features '$label'"
      cat "$LOG"
      FAILED=1
    else
      echo "   cargo $step ok"
    fi
  done

  # The differential harness rebuilds the Rust cdylib itself; run it once
  # against the debug artifact and once against the optimized (release,
  # panic=abort) artifact, because optimization can change behaviour whenever
  # the translation relies on wrapping pointer arithmetic.
  for prof in debug release; do
    echo "   -- rust cdylib profile: $prof"
    if DIFFTEST_PROFILE="$prof" timeout 600 cargo test "${CARGO_ARGS[@]}" \
         --no-default-features "${FEAT[@]}" -- --test-threads=4 >"$LOG" 2>&1; then
      grep -E '^(     Running|test result)' "$LOG" | sed 's/^/      /'
    else
      echo "!! cargo test FAILED for features '$label' (rust profile $prof)"
      cat "$LOG"
      FAILED=1
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Implicit-default invocation (what a plain `cargo test` does).
# ---------------------------------------------------------------------------
echo
echo "############################################################"
echo "# implicit default (plain cargo test)"
echo "############################################################"
if timeout 600 cargo test "${CARGO_ARGS[@]}" >"$LOG" 2>&1; then
  grep -E '^(     Running|test result)' "$LOG" | sed 's/^/   /'
else
  echo "!! plain cargo test FAILED"
  cat "$LOG"
  FAILED=1
fi

echo
if [ "$FAILED" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$FAILED"
