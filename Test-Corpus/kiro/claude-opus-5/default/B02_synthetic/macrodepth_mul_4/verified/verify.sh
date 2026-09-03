#!/usr/bin/env bash
# Single entry point for verifying the translation against the C reference.
#
#   ./verify.sh            # phases A-D: feature checks, symbol parity, full sweep
#   ./verify.sh --mutate   # additionally run the anti-vacuity mutation check
#
# Everything is derived from c_src/ (read-only); C artifacts land in ../cbuild/.
set -u
cd "$(dirname "$0")"
root="$(cd .. && pwd)"

want_mutate=0
[[ "${1:-}" == "--mutate" ]] && want_mutate=1

step() { printf '\n========== %s ==========\n' "$1"; }

fail=0
note_fail() { echo ">>> FAILED: $1"; fail=1; }

step "Build the C reference (.so + driver) for all 24 OP x REPEAT configs"
timeout 600 "$root/build_c.sh" || note_fail "C reference build"

step "Phase A/2 - cargo check every valid feature combination"
timeout 600 ./check_all_features.sh || note_fail "cargo check sweep"

step "Phase D - nm -D symbol parity, all 24 configs"
timeout 600 "$root/check_symbols.sh" | tail -3 || note_fail "symbol parity"

step "Phases B+C - differential tests, all 26 configurations"
timeout 900 ./sweep_so.sh | tail -6 || note_fail "differential sweep"

step "Executable-level sweep (driver vs driver, all 24 configs)"
timeout 600 "$root/sweep_exe.sh" || note_fail "executable sweep"

if [[ $want_mutate -eq 1 ]]; then
  step "Anti-vacuity - injected divergences must be caught"
  timeout 900 "$root/mutation_check.sh" | tail -20 || note_fail "mutation check"
fi
echo
if [[ $fail -eq 0 ]]; then
  echo "VERIFICATION COMPLETE - all phases green"
else
  echo "VERIFICATION INCOMPLETE - see failures above"
fi
exit $fail
