#!/bin/bash
# Full verification sweep, in order.  Everything is derived mechanically; the
# individual steps are documented in translation/{SYMBOLS,ERRORS,CONFIGS}.md.
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT" || exit 1
set -o pipefail
FAIL=0

step() { echo; echo "################ $* ################"; }

step "1/6  cargo check, all 48 feature combinations"
./check_all.sh check 2>&1 | tee /tmp/v_check.out | grep -c '^PASS' || FAIL=1
grep '^FAIL' /tmp/v_check.out && FAIL=1

step "2/6  build the C shared objects (CMake trio + flat) for all 48"
./build_c_all.sh  2>&1 | tee /tmp/v_cbuild.out  | grep -c '^PASS' || FAIL=1
./build_c_flat.sh 2>&1 | tee /tmp/v_cflat.out   | grep -c '^PASS' || FAIL=1
grep '^FAIL' /tmp/v_cbuild.out /tmp/v_cflat.out && FAIL=1

step "3/6  build the Rust cdylib for all 48"
./build_rust_all.sh 2>&1 | tee /tmp/v_rbuild.out | grep -c '^PASS' || FAIL=1
grep '^FAIL' /tmp/v_rbuild.out && FAIL=1

step "4/6  symbol parity (Phase D)"
./symdiff.sh 2>&1 | tee /tmp/v_sym.out | awk '{print $1}' | sort | uniq -c
grep 'MISSING' /tmp/v_sym.out && FAIL=1
./gen_symbols.sh

step "5/6  end-to-end KAT transcript digests, all 48 (C driver vs Rust driver)"
./kat_all.sh 2>&1 | tee /tmp/v_kat.out | grep -c '^MATCH' || FAIL=1
grep '^DIFF\|^BUILDFAIL' /tmp/v_kat.out && FAIL=1

step "6/6  differential test suite (Phases B and C), all 48"
./test_all.sh 2>&1 | tee /tmp/v_test.out | tail -3
grep '^FAIL\|^BUILDFAIL' /tmp/v_test.out && FAIL=1

echo
if [ $FAIL -eq 0 ]; then echo "ALL STEPS PASSED"; else echo "SOME STEPS FAILED"; fi
exit $FAIL
