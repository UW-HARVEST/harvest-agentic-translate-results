#!/usr/bin/env bash
# Full verification driver: Phase A -> D.
#
#   ./verify.sh
#
# 1. builds the C .so
# 2. enumerates every Cargo feature combination and `cargo check`s each
# 3. for each combination x {debug, release}: builds the Rust .so, asserts
#    symbol parity, and runs the Phase B + Phase C differential suites
set -uo pipefail
cd "$(dirname "$0")" || exit 1

LOG=${TMPDIR:-/tmp}/verify.log
: > "$LOG"
say() { echo "$@" | tee -a "$LOG"; }
rc_all=0

# Defensive: make sure no stale .so from an earlier (possibly mutated) build can
# masquerade as up to date. Cargo keys off mtimes, so bump them.
find src tests -type f -exec touch {} + 2>/dev/null

say "### Phase A.3 - build the C shared library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) >> "$LOG" 2>&1
if [[ ! -f c_src/build/libtranslated_rust.so ]]; then
  say "FAILED to build the C .so - see $LOG"; exit 1
fi
say "    ok: c_src/build/libtranslated_rust.so"

say "### Phase A.1/A.2 - enumerate feature combinations and cargo check each"
timeout 600 ./check_all_features.sh check >> "$LOG" 2>&1
if (( $? != 0 )); then say "    cargo check FAILED for some combination"; rc_all=1; fi
combos=$(grep -c '^### cargo check' "$LOG")
say "    ok: $combos feature combination(s) checked"

# derive the same combination list the checker used
mapfile -t FEATURES < <(
  awk '/^\[features\]/{i=1;next} /^\[/{i=0}
       i && /^[A-Za-z0-9_-]+[[:space:]]*=/{split($0,a,"=");gsub(/[[:space:]]/,"",a[1]);
       if(a[1]!="default")print a[1]}' Cargo.toml
)
n=${#FEATURES[@]}

say "### Phases B, C, D - differential suites per combination x profile"
for (( mask = 0; mask < (1 << n); mask++ )); do
  combo=""
  for (( i = 0; i < n; i++ )); do
    (( mask & (1 << i) )) && combo="${combo:+$combo,}${FEATURES[$i]}"
  done
  label="${combo:-<no-features>}"

  for profile in debug release; do
    flag=""; [[ $profile == release ]] && flag="--release"

    say "--- combo [$label] profile [$profile]"

    timeout 600 cargo build --no-default-features --features "$combo" $flag >> "$LOG" 2>&1
    if (( $? != 0 )); then say "    BUILD FAILED"; rc_all=1; continue; fi

    if timeout 120 ./symbol_parity.sh "$profile" >> "$LOG" 2>&1; then
      say "    symbol parity: OK"
    else
      say "    symbol parity: FAILED"; rc_all=1
    fi

    out=$(timeout 600 cargo test --no-default-features --features "$combo" $flag \
            -- --test-threads=1 2>&1)
    echo "$out" >> "$LOG"
    summary=$(echo "$out" | grep -E '^test result:' | tr '\n' ' ')
    if echo "$out" | grep -qE '^test result: FAILED|panicked'; then
      say "    tests: FAILED -> $summary"
      echo "$out" | grep -E '^(test .* FAILED|---- .* stdout)' | head -20 | tee -a "$LOG"
      rc_all=1
    else
      say "    tests: OK -> $summary"
    fi
  done
done

say "### Phase B/C bookkeeping gate - every CONFIGS.md / ERRORS.md row maps to a"
say "    test that exists and passed, and no test is missing from the tables"
if timeout 600 ./row_coverage.sh debug >> "$LOG" 2>&1; then
  say "    row coverage: OK"
else
  say "    row coverage: FAILED"; rc_all=1
  grep -E '^  ' "$LOG" | tail -20 | tee -a "$LOG"
fi

say "======================================================"
if (( rc_all == 0 )); then
  say "VERIFICATION: ALL PHASES PASSED"
else
  say "VERIFICATION: FAILURES PRESENT (see $LOG)"
fi
exit $rc_all
