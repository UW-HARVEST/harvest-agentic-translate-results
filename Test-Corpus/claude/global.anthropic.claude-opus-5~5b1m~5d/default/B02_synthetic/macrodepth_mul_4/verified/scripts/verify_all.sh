#!/usr/bin/env bash
# Full verification gate: Phase A/D symbol parity, compile matrix,
# Phases B+C over every feature combination, and mutation negative controls.
set -u
cd "$(dirname "$0")/.."
export CARGO_NET_OFFLINE=true

rc=0
run() { # label script [args...]
  local label="$1"; shift
  echo "############ $label"
  local log="${TMPDIR:-/tmp}/verify_$$.log"
  bash "scripts/$@" >"$log" 2>&1
  local st=$?
  tail -40 "$log"
  if [ $st -ne 0 ]; then echo "^^^^ FAILED: $label"; rc=1; fi
  rm -f "$log"
  echo
}

run "compile matrix (cargo check, 96 feature combos)" check_all_features.sh
run "symbol parity (nm -D, 24 configs)"               diff_symbols.sh
run "whole-program diff (24 configs x 13 argvs)"      diff_binaries.sh
run "differential suite, canonical 24 configs"        test_all_features.sh
run "differential suite, degenerate feature sets"     test_all_features.sh degenerate
run "mutation negative controls"                      negative_control.sh

if [ $rc -eq 0 ]; then echo "ALL VERIFICATION STEPS PASSED"; else echo "VERIFICATION FAILED"; fi
exit $rc
