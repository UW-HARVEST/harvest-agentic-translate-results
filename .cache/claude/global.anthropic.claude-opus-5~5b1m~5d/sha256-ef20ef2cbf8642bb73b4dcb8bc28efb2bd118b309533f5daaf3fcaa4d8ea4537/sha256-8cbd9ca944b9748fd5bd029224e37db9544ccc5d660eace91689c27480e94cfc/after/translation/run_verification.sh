#!/usr/bin/env bash
# Full verification sweep: rebuilds both .so files, then runs every phase's
# differential tests under every Cargo feature combination and both profiles.
#
# Usage: ./run_verification.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
FAILED=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- 1. C library
say "Building the C shared library (ground truth)"
mkdir -p "$ROOT/c_src/build" || exit 1
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so" || exit 1

# ------------------------------------------------- 2. Enumerate feature combos
# Read [features] out of Cargo.toml; if there are none, the only combination is
# the empty set, which is what --no-default-features gives.
FEATURES=$(awk '
  /^\[features\]/ { inf=1; next }
  /^\[/           { inf=0 }
  inf && /^[a-zA-Z0-9_-]+ *=/ { sub(/ *=.*/, ""); print }
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features] -> the only feature combination is the empty set."
  COMBOS=("")
else
  # Power set of the declared features.
  COMBOS=("")
  for f in $FEATURES; do
    new=()
    for c in "${COMBOS[@]}"; do
      new+=("$c")
      if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
    done
    COMBOS=("${new[@]}")
  done
fi
echo "Feature combinations to verify: ${#COMBOS[@]}"

# ------------------------------------------------------- 3. Run the full matrix
for profile in release debug; do
  if [ "$profile" = release ]; then PROF_FLAG="--release"; else PROF_FLAG=""; fi

  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-<none>}]"
    say "$label"

    # Rebuild the cdylib for this configuration so the tests dlopen the
    # matching artifact.
    # shellcheck disable=SC2086
    if ! cargo build $PROF_FLAG --offline --no-default-features \
          ${combo:+--features "$combo"} 2>&1 | tail -2; then
      echo "BUILD FAILED: $label"; FAILED=1; continue
    fi

    # shellcheck disable=SC2086
    if ! cargo check $PROF_FLAG --offline --no-default-features \
          ${combo:+--features "$combo"} --all-targets 2>&1 | tail -2; then
      echo "CHECK FAILED: $label"; FAILED=1
    fi

    # Phases D, B, C. --test-threads=1 because the harness redirects the
    # process-wide fd 1 and forks.
    for t in symbol_parity valid_paths error_paths; do
      echo "--- $t ($label)"
      log="${TMPDIR:-/tmp}/verif_${profile}_${t}.log"
      # Run ONCE: capture to a log, report from it, and use the real exit status.
      # shellcheck disable=SC2086
      timeout 600 cargo test $PROF_FLAG --offline --no-default-features \
            ${combo:+--features "$combo"} --test "$t" \
            -- --test-threads=1 >"$log" 2>&1
      status=$?
      grep -E 'test result|^test .*FAILED|panicked' "$log" | head -20
      if [ "$status" -ne 0 ]; then
        echo "TEST FAILED (status $status): $t ($label)"; FAILED=1
      fi
    done
  done
done

say "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAILED"
