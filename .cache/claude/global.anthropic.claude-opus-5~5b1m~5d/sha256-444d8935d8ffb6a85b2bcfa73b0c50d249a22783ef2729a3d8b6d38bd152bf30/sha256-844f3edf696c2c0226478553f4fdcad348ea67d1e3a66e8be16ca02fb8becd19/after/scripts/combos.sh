#!/usr/bin/env bash
# Enumerate the feature combinations of translation/Cargo.toml.
#
# The CMake cache variables are OP in {add,sub,mul} (default add) and
# REPEAT in {0..7} (default 5); Cargo mirrors each *value* as a feature of the
# same name. A "valid" combination therefore selects at most one OP feature and
# at most one REPEAT feature (selecting none means the CMake default).
#
# Prints one combination per line as: <feature-list>|<OP>|<REPEAT>
# where <feature-list> may be empty (meaning --no-default-features only) and
# <OP>/<REPEAT> are the effective values that the C build must be given.

set -euo pipefail

for op in "" add sub mul; do
  for rep in "" 0 1 2 3 4 5 6 7; do
    feats=""
    [ -n "$op" ] && feats="$op"
    if [ -n "$rep" ]; then
      if [ -n "$feats" ]; then feats="$feats,$rep"; else feats="$rep"; fi
    fi
    eff_op="${op:-add}"
    eff_rep="${rep:-5}"
    echo "$feats|$eff_op|$eff_rep"
  done
done
