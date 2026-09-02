#!/usr/bin/env bash
# Run the whole differential suite for EVERY feature combination.
#
# For each combination: build the cdylib + binary at that configuration, then
# run the three test targets. The harness (tests/support/mod.rs) derives the
# active OP/REPEAT from the same cfg features and picks the matching C
# reference .so / executable, and asserts both libraries report the same
# compiled-in OP name so a stale build cannot pass silently.
set -u
cd "$(dirname "$0")"

CB=../cbuild
if [[ ! -d $CB/lib ]]; then
  echo "C reference libraries missing; building them now"
  mkdir -p $CB/lib $CB/exe
  for OP in add sub mul; do for R in 0 1 2 3 4 5 6 7; do
    gcc -O2 -fPIC -shared -DOP=$OP -DREPEAT=$R -I../c_src/src \
        -o $CB/lib/libmd_${OP}_${R}.so ../c_src/src/mdcore.c || exit 1
    gcc -O2 -DOP=$OP -DREPEAT=$R -I../c_src/src \
        -o $CB/exe/driver_${OP}_${R} ../c_src/src/mdcore.c ../c_src/src/mdmain.c || exit 1
  done; done
fi

fail=0
run_combo() {
  local feats="$1" label="$2"
  if ! timeout 300 cargo build --release --no-default-features --features "$feats" \
        >/tmp/rc_build.log 2>&1; then
    echo "BUILD FAIL   [$label]"; tail -20 /tmp/rc_build.log; fail=1; return
  fi
  if ! timeout 300 cargo test --release --no-default-features --features "$feats" \
        -- --test-threads=1 >/tmp/rc_test.log 2>&1; then
    echo "TEST FAIL    [$label]"
    grep -E '^(test .* FAILED|---- |assertion|  left|  right|thread)' /tmp/rc_test.log | head -30
    fail=1; return
  fi
  local n
  n=$(awk '/^test result: ok\./ {s+=$4} END {print s+0}' /tmp/rc_test.log)
  if [[ "$n" -lt 31 ]]; then
    echo "SUSPICIOUS   [$label] only $n tests ran"; fail=1; return
  fi
  printf 'PASS %-28s (%s tests)\n' "$label" "$n"
}

echo "=== canonical OP x REPEAT (24) ==="
for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    run_combo "$OP,repeat_$R" "$OP/$R"
  done
done

echo "=== alias spellings (op_<x> and bare \"<n>\") ==="
for OP in op_add op_sub op_mul; do
  for R in 0 3 5 7; do
    run_combo "$OP,$R" "$OP/\"$R\""
  done
done

echo "=== #ifndef fallbacks (missing OP => add, missing REPEAT => 5) ==="
for R in 0 1 2 3 4 5 6 7; do run_combo "repeat_$R" "<no OP>/$R"; done
for OP in add sub mul; do run_combo "$OP" "$OP/<no REPEAT>"; done
run_combo "" "<no OP>/<no REPEAT>"

echo "=== everything enabled at once (documented precedence: sub, repeat_0) ==="
ALL="add,sub,mul,op_add,op_sub,op_mul,0,1,2,3,4,5,6,7,repeat_0,repeat_1,repeat_2,repeat_3,repeat_4,repeat_5,repeat_6,repeat_7"
run_combo "$ALL" "all-features"

echo
if [[ $fail -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit $fail
