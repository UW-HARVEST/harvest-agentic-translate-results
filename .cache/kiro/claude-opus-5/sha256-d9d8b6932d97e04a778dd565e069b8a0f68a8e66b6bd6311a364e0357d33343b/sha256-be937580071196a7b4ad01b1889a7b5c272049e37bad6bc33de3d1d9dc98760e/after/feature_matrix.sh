#!/usr/bin/env bash
# Enumerate every valid feature combination (one OP x one REPEAT, mirroring the
# CMake cache variables) and run an arbitrary cargo subcommand for each.
#
#   ./feature_matrix.sh check
#   ./feature_matrix.sh test --test differential
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT/translation"

OPS=(add sub mul)
REPEATS=(0 1 2 3 4 5 6 7)

fail=0
for op in "${OPS[@]}"; do
  for rep in "${REPEATS[@]}"; do
    combo="$op,$rep"
    echo "######## features: $combo ########"
    if ! MD_OP="$op" MD_REPEAT="$rep" timeout 600 cargo "$@" \
          --no-default-features --features "$combo" 2>&1 | tail -n 25; then
      echo "!!!! FAILED: $combo"
      fail=1
    fi
  done
done

# The no-feature build must also work: the C header falls back to OP=add,
# REPEAT=5 via #ifndef, so `--no-default-features` alone is a valid config.
echo "######## features: <none> (falls back to add,5) ########"
if ! MD_OP=add MD_REPEAT=5 timeout 600 cargo "$@" --no-default-features 2>&1 | tail -n 25; then
  echo "!!!! FAILED: <none>"
  fail=1
fi

echo "######## features: default ########"
if ! MD_OP=add MD_REPEAT=5 timeout 600 cargo "$@" 2>&1 | tail -n 25; then
  echo "!!!! FAILED: default"
  fail=1
fi

exit "$fail"
