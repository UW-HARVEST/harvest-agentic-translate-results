#!/bin/bash
# One-shot verification entry point.
#
#   tools/verify_all.sh          # build everything + run the Rust test suite
#                                # (debug and release) + feature-combination check
#   tools/verify_all.sh fuzz     # additionally run the standalone python sweeps
set -u
cd "$(dirname "$0")/.."

rc=0
step() {
  echo
  echo "################ $* ################"
  if ! timeout 600 "$@"; then
    echo "FAILED: $*"
    rc=1
  fi
}

step bash tools/build_all.sh debug
step bash tools/build_all.sh release
step bash tools/check_features.sh check
step cargo test --offline
step cargo test --offline --release

if [ "${1:-}" = fuzz ]; then
  step python3 tools/fuzz.py 8000 11
  step python3 tools/fuzz.py 8000 22
  step python3 tools/rounding.py 800 4242
  step python3 tools/exhaustive.py core
  step python3 tools/exhaustive.py wide
  step python3 tools/exhaustive.py words
  step python3 tools/deep_sweep.py hex4
  step python3 tools/deep_sweep.py word4
fi

echo
if [ "$rc" -eq 0 ]; then
  echo "==== VERIFICATION PASSED ===="
else
  echo "==== VERIFICATION FAILED ===="
fi
exit "$rc"
