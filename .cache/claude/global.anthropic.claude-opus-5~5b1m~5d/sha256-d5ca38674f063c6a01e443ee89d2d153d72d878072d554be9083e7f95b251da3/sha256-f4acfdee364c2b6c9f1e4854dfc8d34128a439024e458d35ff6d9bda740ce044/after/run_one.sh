#!/bin/bash
# Differential test for ONE cargo feature combination.
#   ./run_one.sh <backend> <secpar> <thash> [target-dir-suffix]
#
#   1. builds the C reference .so's + KAT driver for the matching CMake config,
#   2. builds the Rust cdylib + driver with the matching cargo features,
#   3. runs the integration tests, which dlopen BOTH and compare,
#   4. compares the two KAT driver transcripts with each other and with
#      reference_outputs.txt.
R="$(cd "$(dirname "$0")" && pwd)"
b=$1; s=$2; t=$3; slot=${4:-0}

cb=$b; [ "$b" = "shake256" ] && cb=shake
export CARGO_TARGET_DIR="$R/translation/target-$slot"
mkdir -p "$CARGO_TARGET_DIR"

"$R/build_c.sh" "$cb" "$s" "$t" > /dev/null 2>&1 \
  || { echo "FAIL $b/$t/$s: C build"; exit 1; }

cd "$R/translation" || exit 1
if ! cargo build --release --offline --no-default-features --features "$b,$t,$s" > "$CARGO_TARGET_DIR/build.log" 2>&1; then
  echo "FAIL $b/$t/$s: rust build"; tail -20 "$CARGO_TARGET_DIR/build.log"; exit 1
fi

out=$(SPX_C_BUILD="$R/cbuild/$cb-$s-$t" \
      SPX_RUST_SO="$CARGO_TARGET_DIR/release/libsphincs_core_det.so" \
      cargo test --offline --no-default-features --features "$b,$t,$s" -- --test-threads=1 2>&1)
if echo "$out" | grep -qE '^test result: FAILED|^error|SIGSEGV|SIGABRT|panicked'; then
  echo "FAIL $b/$t/$s: tests"
  echo "$out" > "$R/.results/full-$b-$t-$s.log"
  echo "$out" | grep -A3 -E 'panicked|FAILED|^error|SIG' | head -60
  exit 1
fi
n=$(echo "$out" | grep -oE 'ok\. [0-9]+ passed' | awk '{x+=$2} END {print x+0}')
if [ "${n:-0}" -lt 60 ]; then
  echo "FAIL $b/$t/$s: only $n tests ran (expected >= 60)"; exit 1
fi

cdrv=$("$R/cbuild/$cb-$s-$t/app/driver"); cst=$?
rdrv=$("$CARGO_TARGET_DIR/release/driver"); rst=$?
if [ "$cdrv" != "$rdrv" ] || [ "$cst" != "$rst" ]; then
  echo "FAIL $b/$t/$s: driver C='$cdrv'(exit=$cst) Rust='$rdrv'(exit=$rst)"; exit 1
fi
ref=$(grep -P "^\Q$cb $t $s\E\t" "$R/reference_outputs.txt" | cut -f2)
refst=$(grep -P "^\Q$cb $t $s\E\t" "$R/reference_outputs.txt" | cut -f3 | sed 's/exit=//')
if [ -z "$ref" ]; then
  echo "FAIL $b/$t/$s: no reference_outputs.txt entry"; exit 1
fi
if [ "$ref" != "$rdrv" ] || [ "$refst" != "$rst" ]; then
  echo "FAIL $b/$t/$s: driver vs reference: got='$rdrv'(exit=$rst) want='$ref'(exit=$refst)"; exit 1
fi
echo "ok   $b/$t/$s  ($n tests + driver digest == C == reference_outputs.txt)"
