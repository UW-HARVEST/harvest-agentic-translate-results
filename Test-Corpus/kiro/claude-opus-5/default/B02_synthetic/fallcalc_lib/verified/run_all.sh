#!/usr/bin/env bash
# End-to-end verification driver: builds both libraries and runs every phase.
#
#   Phase A  artifacts + symbol parity   (check_symbols.sh)
#   Phase B  valid-path differential     (tests/phase_b_configs.rs)
#   Phase C  error-path differential     (tests/phase_c_errors.rs)
#   Phase D  feature combinations        (check_features.sh)
#            + the same suite run against the RELEASE cdylib
#   plus     mutation check              (mutation_check.sh)
set -uo pipefail
cd "$(dirname "$0")"

rc=0
step() {
  echo
  echo "=============================================================="
  echo "== $1"
  echo "=============================================================="
}

step "Build the C shared library"
( cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -la ../c_src/build/*.so

step "cargo check (default) + build both Rust profiles"
timeout 300 cargo check 2>&1 | tail -3 || rc=1
timeout 300 cargo build --lib --target-dir target/ffi-so 2>&1 | tail -2 || rc=1
timeout 300 cargo build --release 2>&1 | tail -2 || rc=1

step "Phase A -- artifact / test cross-check (every CONFIGS + ERRORS row has a test)"
./check_coverage.sh || rc=1

step "Phase A/D -- symbol parity (debug cdylib)"
./check_symbols.sh || rc=1

step "Phase A/D -- symbol parity (release cdylib)"
./check_symbols.sh target/release/libfallcalc_lib.so || rc=1

# Run the suite and require BOTH test binaries to report 0 failures.
# (A bare `cargo test | tail` would hide one binary's summary line.)
run_suite() {
  local label="$1" log
  log=$(mktemp)
  local st=0
  timeout 600 cargo test --tests > "$log" 2>&1 || st=1
  grep -E '^test result:' "$log" | sed 's/^/  /'
  local n_ok n_bad
  n_ok=$(grep -cE '^test result: ok\..* 0 failed' "$log")
  n_bad=$(grep -cE '^test result: FAILED' "$log")
  # 3 binaries report a summary: the (empty) cdylib unit-test bin, phase_b, phase_c.
  if [ "$st" -ne 0 ] || [ "$n_bad" -ne 0 ] || [ "$n_ok" -lt 2 ]; then
    echo "  $label: FAILED (exit=$st, ok-summaries=$n_ok, failed-summaries=$n_bad)"
    sed -n '1,80p' "$log"
    rm -f "$log"
    return 1
  fi
  echo "  $label: all test binaries reported 0 failures"
  rm -f "$log"
}

step "Phase B + C -- differential tests against the DEBUG cdylib"
run_suite "debug cdylib" || rc=1

step "Phase B + C -- differential tests against the RELEASE cdylib"
FALLCALC_RUST_SO="$PWD/target/release/libfallcalc_lib.so" run_suite "release cdylib" || rc=1

step "Phase D -- every feature combination"
./check_features.sh || rc=1

step "Mutation check -- proves the suite detects divergence"
timeout 900 ./mutation_check.sh | tail -12 || rc=1

echo
if [ "$rc" -eq 0 ]; then
  echo "ALL PHASES PASSED"
else
  echo "FAILURES PRESENT (rc=$rc)"
fi
exit $rc
