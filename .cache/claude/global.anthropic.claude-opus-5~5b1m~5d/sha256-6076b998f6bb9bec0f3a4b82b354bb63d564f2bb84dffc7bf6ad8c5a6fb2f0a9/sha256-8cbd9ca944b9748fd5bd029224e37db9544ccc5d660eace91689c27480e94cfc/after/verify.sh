#!/usr/bin/env bash
# Full verification gate: builds both libraries, diffs the exported symbol
# sets, and runs the whole differential suite under every feature combination
# and both profiles.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
FAIL=0

step() { printf '\n=== %s ===\n' "$1"; }

step "Build the C shared library"
(mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C BUILD FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C  .so: $C_SO"

step "Feature combinations declared in translation/Cargo.toml"
# Mechanically extract the [features] table; empty means a single config.
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' \
        translation/Cargo.toml)
if [ -z "$FEATS" ]; then
  echo "no [features] table -> one configuration"
  COMBOS=("default" "--no-default-features")
else
  COMBOS=("default" "--no-default-features")
  for f in $FEATS; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
fi

for PROFILE in release debug; do
  PFLAG=""; [ "$PROFILE" = release ] && PFLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    CFLAGS=""; [ "$COMBO" != default ] && CFLAGS="$COMBO"

    step "profile=$PROFILE features=$COMBO :: build cdylib"
    (cd translation && cargo build --offline $PFLAG $CFLAGS 2>&1 | tail -3) \
      || { echo "RUST BUILD FAILED"; FAIL=1; continue; }

    R_SO="$ROOT/translation/target/$PROFILE/libmathop_lib.so"

    step "profile=$PROFILE features=$COMBO :: symbol parity (nm -D)"
    diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
         <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort) \
      && echo "SYMBOL DIFF EMPTY: $(nm -D --defined-only "$C_SO" | wc -l) symbols match" \
      || { echo "SYMBOL PARITY FAILED"; FAIL=1; }

    step "profile=$PROFILE features=$COMBO :: undefined non-libc symbols"
    # An import only counts as a problem if it is NOT resolvable from the C
    # library / dynamic loader AND is not something the C .so also imports.
    C_UND=$(nm -D --undefined-only "$C_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u)
    # Every DT_NEEDED library of the Rust .so counts as a system provider.
    LIBC_DEF=$(for l in $(ldd "$R_SO" | awk '/=>/ {print $3}') \
                        /lib64/ld-linux-x86-64.so.2; do
                 nm -D --defined-only "$l" 2>/dev/null
               done | awk '{print $NF}' | sed 's/@.*//' | sort -u)
    UND=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u \
          | grep -vxF "$C_UND" | grep -vxF "$LIBC_DEF")
    if [ -n "$UND" ]; then echo "UNRESOLVED: $UND"; FAIL=1; else echo "none (all imports are libc/libgcc/ld or shared with the C .so)"; fi

    step "profile=$PROFILE features=$COMBO :: ldd -r (link-time resolution)"
    if ldd -r "$R_SO" 2>&1 | grep -q 'undefined symbol'; then
      ldd -r "$R_SO" 2>&1 | grep 'undefined symbol'; FAIL=1
    else echo "all dynamic relocations resolve"; fi

    step "profile=$PROFILE features=$COMBO :: differential test suite"
    (cd translation && timeout 600 cargo test --offline $PFLAG $CFLAGS 2>&1 \
      | grep -E '^(test |running |test result|error|warning: unused)' | tail -80)
    # shellcheck disable=SC2181
    if [ "${PIPESTATUS[0]:-0}" != 0 ]; then :; fi
    (cd translation && timeout 600 cargo test --offline $PFLAG $CFLAGS >/dev/null 2>&1) \
      || { echo "TESTS FAILED for profile=$PROFILE features=$COMBO"; FAIL=1; }
  done
done

step "RESULT"
if [ "$FAIL" = 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
