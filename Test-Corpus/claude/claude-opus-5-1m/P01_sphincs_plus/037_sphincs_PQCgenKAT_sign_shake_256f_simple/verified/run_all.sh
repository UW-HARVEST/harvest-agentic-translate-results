#!/bin/bash
# Run the C-vs-Rust differential test suite for every (backend, thash, secpar)
# combination.  `cargo test` does not rebuild the cdylib, so each combination is
# built explicitly and the .so is snapshotted to a per-combination path that the
# test harness is pointed at via SPHINCS_RUST_SO.
set -u
R="$(cd "$(dirname "$0")" && pwd)"
export CARGO_NET_OFFLINE=true
T="${TMPDIR:-/var/tmp}"
SO="$T/sphincs_so"; mkdir -p "$SO"
LOG="$T/run_all"; mkdir -p "$LOG"
COMBOS="${COMBOS:-}"
if [ -z "$COMBOS" ]; then
  COMBOS=""
  for bk in haraka sha2 shake blake; do for th in robust simple; do for sp in 128s 128f 192s 192f 256s 256f; do
    COMBOS="$COMBOS $bk:$th:$sp"; done; done; done
fi
TESTARGS="${TESTARGS:-}"
pass=0; fail=0; failed=""
for c in $COMBOS; do
  bk=${c%%:*}; rest=${c#*:}; th=${rest%%:*}; sp=${rest##*:}
  feats="$bk $th $sp"
  cd "$R" || exit 1
  if ! cargo build --release --no-default-features --features "$feats" > "$LOG/$c.build" 2>&1; then
    echo "BUILD FAIL $c"; tail -30 "$LOG/$c.build"; fail=$((fail+1)); failed="$failed $c"; continue
  fi
  cp "$R/target/release/libsphincsplus.so" "$SO/$bk-$th-$sp.so"
  if SPHINCS_RUST_SO="$SO/$bk-$th-$sp.so" SPHINCS_C_DIR="$R/cbuild/$bk-$th-$sp" \
     timeout 1800 cargo test --release --no-default-features --features "$feats" \
        --test differential -- --test-threads=1 $TESTARGS > "$LOG/$c.test" 2>&1; then
    n=$(grep -oE 'test result: ok\. [0-9]+ passed' "$LOG/$c.test" | grep -oE '[0-9]+' | head -1)
    echo "PASS $c ($n tests)"
    pass=$((pass+1))
  else
    echo "FAIL $c  -> $LOG/$c.test"
    grep -E 'MISMATCH|panicked|^test .* FAILED|test result:' "$LOG/$c.test" | head -20
    fail=$((fail+1)); failed="$failed $c"
  fi
done
echo "================================"
echo "run_all: pass=$pass fail=$fail"
[ -n "$failed" ] && echo "failed combos:$failed"
[ "$fail" -eq 0 ]
