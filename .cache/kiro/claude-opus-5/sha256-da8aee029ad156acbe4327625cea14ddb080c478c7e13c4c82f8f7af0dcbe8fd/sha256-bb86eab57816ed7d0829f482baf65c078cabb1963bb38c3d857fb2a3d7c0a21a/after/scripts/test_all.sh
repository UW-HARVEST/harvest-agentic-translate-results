#!/bin/bash
# Run the differential test suite for one configuration (or all of them).
#
#   test_all.sh                      -> every configuration
#   test_all.sh blake simple 128f    -> one configuration
#   TESTS="configs errors" test_all.sh ...   -> restrict to some test binaries
#
# `cargo test` does NOT rebuild the `cdylib`, so the library is built
# explicitly first and handed to the harness via SPX_RUST_SO.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT/translation"
TESTS="${TESTS:-smoke isolation configs errors}"
SO="$CRATE/target/release/libsphincs_plus.so"

run_one() {
  local b=$1 t=$2 s=$3
  local feats="$b,$t,$s"

  if [ ! -d "$ROOT/c_build/$b-$t-$s" ]; then
    "$ROOT/scripts/build_c.sh" "$b" "$t" "$s" >/dev/null || { echo "FAIL(cbuild) $b-$t-$s"; return 1; }
  fi

  ( cd "$CRATE" && timeout 600 cargo build --release --no-default-features --features "$feats" \
      > /tmp/spx_build.log 2>&1 ) \
    || { echo "FAIL(build) $b-$t-$s"; tail -20 /tmp/spx_build.log; return 1; }

  local rc=0
  for tb in $TESTS; do
    ( cd "$CRATE" && SPX_RUST_SO="$SO" timeout 900 cargo test --release \
        --no-default-features --features "$feats" --test "$tb" -- --test-threads=1 \
        > "/tmp/spx_test_${b}_${t}_${s}_${tb}.log" 2>&1 ) || {
      echo "FAIL(test:$tb) $b-$t-$s"
      grep -E "^(test .*FAILED|thread .* panicked|assertion|\[.*\])" \
        "/tmp/spx_test_${b}_${t}_${s}_${tb}.log" | head -20
      rc=1
    }
  done
  [ $rc -eq 0 ] && echo "OK   $b-$t-$s ($(for tb in $TESTS; do grep -h "test result:" "/tmp/spx_test_${b}_${t}_${s}_${tb}.log" | sed 's/test result: //;s/;.*//' | tr -d '\n'; echo -n " "; done))"
  return $rc
}

rc=0
if [ $# -eq 3 ]; then
  run_one "$1" "$2" "$3" || rc=1
elif [ $# -eq 1 ]; then
  # single backend, all thash x secpar
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      run_one "$1" "$t" "$s" || rc=1
    done
  done
else
  for b in haraka sha2 shake blake; do
    for t in robust simple; do
      for s in 128s 128f 192s 192f 256s 256f; do
        run_one "$b" "$t" "$s" || rc=1
      done
    done
  done
fi
exit $rc
