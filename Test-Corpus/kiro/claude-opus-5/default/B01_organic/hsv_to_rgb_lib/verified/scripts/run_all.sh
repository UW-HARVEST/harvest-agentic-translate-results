#!/usr/bin/env bash
# One-shot full verification: build both libraries, diff exported symbols, run
# every differential test in both Rust profiles and every feature combination,
# and finish with the mutation check that proves the suite is not vacuous.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "########## 1/3  symbols ##########"
"$here/build_and_diff_symbols.sh"

echo
echo "########## 2/3  differential tests (all feature combos x both profiles) ##########"
"$here/check_all_features.sh"

echo
echo "########## 3/3  mutation check ##########"
"$here/mutation_check.sh"

echo
echo "ALL VERIFICATION STEPS PASSED ✅"
