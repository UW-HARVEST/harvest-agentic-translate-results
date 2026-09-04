#!/bin/bash
# Differential test: C reference .so vs Rust .so, byte-for-byte.
set -u
cd "$(dirname "$0")"
ROOT=..

gcc -shared -fPIC -I$ROOT/c_src/include -I$ROOT/c_src/src -o libref_c.so $ROOT/c_src/src/lib.c || exit 1
cp $ROOT/translation/target/release/libc_src.so libref_rs.so || exit 1

gcc -o drv_c driver.c -L. -lref_c -Wl,-rpath,'$ORIGIN' || exit 1
gcc -o drv_rs driver.c -L. -lref_rs -Wl,-rpath,'$ORIGIN' || exit 1

fail=0
case_no=0

run_case() {
  case_no=$((case_no+1))
  local desc="$1"; shift
  env -i "$@" ./drv_c  >out_c.txt 2>err_c.txt; rc_c=$?
  env -i "$@" ./drv_rs >out_rs.txt 2>err_rs.txt; rc_rs=$?
  # combined-stream capture too (exercises buffering/interleaving)
  env -i "$@" ./drv_c  >both_c.txt 2>&1
  env -i "$@" ./drv_rs >both_rs.txt 2>&1
  local bad=""
  cmp -s out_c.txt out_rs.txt || bad="$bad stdout"
  cmp -s err_c.txt err_rs.txt || bad="$bad stderr"
  cmp -s both_c.txt both_rs.txt || bad="$bad combined"
  [ "$rc_c" = "$rc_rs" ] || bad="$bad exit($rc_c/$rc_rs)"
  if [ -n "$bad" ]; then
    echo "FAIL [$case_no] $desc ->$bad"
    diff <(head -50 out_c.txt) <(head -50 out_rs.txt) | head -20
    diff err_c.txt err_rs.txt | head -20
    fail=1
  else
    echo "ok   [$case_no] $desc  ($(wc -c <out_c.txt) B stdout, $(wc -c <err_c.txt) B stderr)"
  fi
}

run_case "no env"
run_case "verbose=1" PROG_VERBOSE=1
run_case "verbose=0" PROG_VERBOSE=0
run_case "verbose=abc" PROG_VERBOSE=abc
run_case "verbose=x1x" PROG_VERBOSE=x1x
run_case "verbose=empty" PROG_VERBOSE=
run_case "debug=1" PROG_DEBUG=1
run_case "debug=0" PROG_DEBUG=0
run_case "debug=empty" PROG_DEBUG=
run_case "optimize=empty" PROG_OPTIMIZE=
run_case "optimize=0" PROG_OPTIMIZE=0
run_case "optimize=whatever" PROG_OPTIMIZE=whatever
run_case "v+d" PROG_VERBOSE=1 PROG_DEBUG=1
run_case "v+d+o" PROG_VERBOSE=1 PROG_DEBUG=1 PROG_OPTIMIZE=1
run_case "v+o" PROG_VERBOSE=1 PROG_OPTIMIZE=yes
run_case "d+o" PROG_DEBUG=1 PROG_OPTIMIZE=yes
run_case "base=0" PROG_BASE_OFFSET=0
run_case "base=-100000" PROG_BASE_OFFSET=-100000
run_case "base=comma" PROG_BASE_OFFSET=1,2
run_case "base=semi" PROG_BASE_OFFSET=1\;2
run_case "base=both" PROG_BASE_OFFSET=1\;2,3
run_case "base=junk" PROG_BASE_OFFSET=notanumber
run_case "base=overflow" PROG_BASE_OFFSET=99999999999999999999
run_case "base=neg-overflow" PROG_BASE_OFFSET=-99999999999999999999
run_case "base=leading-space" PROG_BASE_OFFSET="   42abc"
run_case "base=+7" PROG_BASE_OFFSET=+7
run_case "base=0x10" PROG_BASE_OFFSET=0x10
run_case "base=010" PROG_BASE_OFFSET=010
run_case "mult=0" PROG_MULTIPLIER=0
run_case "mult=-3" PROG_MULTIPLIER=-3
run_case "mult=comma" PROG_MULTIPLIER=a,b
run_case "mult=semi" PROG_MULTIPLIER=a\;b
run_case "mult=big" PROG_MULTIPLIER=2147483647
run_case "mult=min" PROG_MULTIPLIER=-2147483648
run_case "all-set" PROG_VERBOSE=1 PROG_DEBUG=1 PROG_OPTIMIZE=1 \
    PROG_BASE_OFFSET=-500 PROG_MULTIPLIER=-9
run_case "all-warn" PROG_VERBOSE=1 PROG_DEBUG=1 \
    PROG_BASE_OFFSET=x,y PROG_MULTIPLIER=x\;y
run_case "extra-names" PROG_VERBOSE=1 PROG_DEBUG=1 \
    PROG_X_COMMA=5,5 PROG_X_SEMI=5\;5 PROG_X_BOTH=5\;5,5 PROG_X_JUNK=zzz \
    PROG_X_EMPTY= PROG_X_BIG=9999999999 PROG_X_NEG=-77 PROG_X_SPACE="  8 " \
    PROG_X_HEX=0xff PATH=/usr/bin
run_case "verbose-only-comma" PROG_VERBOSE=1 PROG_BASE_OFFSET=,
run_case "verbose-only-semi" PROG_VERBOSE=1 PROG_BASE_OFFSET=\;

echo "-----"
if [ $fail -eq 0 ]; then echo "ALL DIFFERENTIAL CASES BYTE-IDENTICAL ($case_no cases)"; else echo "DIFFERENCES FOUND"; fi
exit $fail
