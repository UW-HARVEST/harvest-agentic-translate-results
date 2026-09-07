#!/bin/bash
# Enumerate and `cargo check` every valid feature combination.
#
# The Cargo features mirror the CMake cache variables 1:1:
#   -DOP=add|sub|mul  ->  --features add|sub|mul
#   -DREPEAT=0..7     ->  --features 0|1|...|7
# giving 3 x 8 = 24 combinations. Also checked:
#   * the `default` feature set (= add,5, the CMake defaults);
#   * `--no-default-features` with no feature at all, which exercises the
#     `#ifndef OP / #define OP add` + `#ifndef REPEAT / #define REPEAT 5`
#     fallback in mdmacros.h;
#   * over-specified sets (two OP features, or two REPEAT features) that Cargo
#     permits when `default` is left on, to confirm the documented precedence
#     still compiles.
set -u
cd "$(cd "$(dirname "$0")" && pwd)/translation" || exit 1

rc=0
check() { # $1=label  $2...=cargo args
  local label="$1"; shift
  if timeout 300 cargo check --all-targets "$@" > "/tmp/featchk.log" 2>&1; then
    local w; w=$(grep -c '^warning' /tmp/featchk.log)
    printf '%-28s OK (%s warnings)\n' "$label" "$w"
  else
    printf '%-28s FAIL\n' "$label"
    tail -20 /tmp/featchk.log
    rc=1
  fi
}

for op in add sub mul; do
  for r in 0 1 2 3 4 5 6 7; do
    check "$op,$r" --no-default-features --features "$op,$r"
  done
done

check "default"            
check "no features at all"  --no-default-features
check "all OP features"     --no-default-features --features "add,sub,mul,5"
check "all REPEAT features" --no-default-features --features "add,0,1,2,3,4,5,6,7"
check "everything"          --all-features

exit $rc
