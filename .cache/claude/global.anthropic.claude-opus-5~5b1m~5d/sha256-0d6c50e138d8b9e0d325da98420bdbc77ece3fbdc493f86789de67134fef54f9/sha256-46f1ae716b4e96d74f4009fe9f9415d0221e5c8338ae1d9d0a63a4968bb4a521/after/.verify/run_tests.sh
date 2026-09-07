#!/bin/bash
# Differential test runner.
#
#   run_tests.sh                 -> all 48 feature combinations
#   run_tests.sh <b> <t> <s>     -> one combination
#   COMBOS="blake,simple,128f blake,robust,192s" run_tests.sh
#
# Env:
#   TESTS="--test layer0_utils --test errors"   restrict to some test binaries
#   SKIP_DRIVER=1                                skip the binary stdout compare
set -u
ROOT=$HARVEST_WORKDIR
CRATE=$ROOT/translation
CLIBS=$ROOT/.verify/clibs
CBUILDS=$ROOT/.verify/cbuilds
TESTS=${TESTS:-}
SKIP_DRIVER=${SKIP_DRIVER:-0}

if [ $# -eq 3 ]; then
  COMBOS="$1,$2,$3"
elif [ -n "${COMBOS:-}" ]; then
  :
else
  COMBOS=""
  for b in haraka sha2 shake blake; do for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do COMBOS="$COMBOS $b,$t,$s"; done
  done; done
fi

pass=0; fail=0; failed_combos=""
for combo in $COMBOS; do
  b=${combo%%,*}; rest=${combo#*,}; t=${rest%%,*}; s=${rest##*,}
  clib=$CLIBS/libspx_c_${b}_${t}_${s}.so
  if [ ! -f "$clib" ]; then echo "MISSING C LIB $clib"; fail=$((fail+1)); continue; fi

  ok=1
  # --- build the Rust cdylib + driver for this combo -----------------------
  if ! (cd "$CRATE" && cargo build --offline --release --no-default-features \
        --features "$combo" >"$ROOT/.verify/last_build.log" 2>&1); then
    echo "RUSTBUILD FAIL $combo"; tail -20 "$ROOT/.verify/last_build.log"; ok=0
  fi

  # --- Phase B/C differential tests ---------------------------------------
  if [ $ok -eq 1 ]; then
    out=$( cd "$CRATE" && SPX_C_LIB="$clib" \
        SPX_RUST_LIB="$CRATE/target/release/libsphincs_plus.so" \
        timeout 900 cargo test --offline --release --no-default-features \
        --features "$combo" $TESTS 2>&1 )
    if [ $? -ne 0 ]; then
      echo "TEST FAIL $combo"
      printf '%s\n' "$out" | grep -E "panicked|first diff|C = |R = |C \=|test result|FAILED|SIGABRT|SIGSEGV" | head -30
      ok=0
    else
      n=$(printf '%s\n' "$out" | grep -oE '[0-9]+ passed' | awk '{s+=$1} END{print s}')
      echo "TESTS OK $combo ($n assertions/tests passed)"
    fi
  fi

  # --- Phase B row 60: driver stdout byte-for-byte ------------------------
  if [ $ok -eq 1 ] && [ "$SKIP_DRIVER" != "1" ]; then
    cdrv=$CBUILDS/${b}_${t}_${s}/app/driver
    rdrv=$CRATE/target/release/driver
    if [ -x "$cdrv" ]; then
      cout=$(cd "$CBUILDS/${b}_${t}_${s}" && LD_LIBRARY_PATH="app:lib/$b:$ROOT/.verify/osslib" \
              timeout 900 ./app/driver 2>/dev/null); crc=$?
      rout=$(timeout 900 "$rdrv" 2>/dev/null); rrc=$?
      if [ "$cout" != "$rout" ] || [ "$crc" != "$rrc" ]; then
        echo "DRIVER MISMATCH $combo"
        echo "  C   (rc=$crc): $cout"
        echo "  Rust(rc=$rrc): $rout"
        ok=0
      else
        echo "DRIVER OK $combo rc=$crc  $cout"
      fi
    else
      echo "NO C DRIVER $combo"; ok=0
    fi
  fi

  if [ $ok -eq 1 ]; then pass=$((pass+1)); else fail=$((fail+1)); failed_combos="$failed_combos $combo"; fi
done

echo "======================================================"
echo "combinations passed: $pass   failed: $fail"
[ -n "$failed_combos" ] && echo "failed:$failed_combos"
[ $fail -eq 0 ]
