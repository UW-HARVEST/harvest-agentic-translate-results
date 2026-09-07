#!/bin/bash
# Run one cargo test invocation for every feature combination.
#
#   ./test_all.sh                 # all 48 combos, all test targets
#   ./test_all.sh --test level0   # all 48 combos, one test target
#   COMBOS="blake,simple,128f" ./test_all.sh   # a subset
#
# The Rust cdylib is rebuilt before each run so the .so the tests dlopen always
# matches the feature set they were compiled for.
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation" || exit 1
FAIL=0
PASSED=0

if [ -n "$COMBOS" ]; then
  LIST="$COMBOS"
else
  LIST=""
  for be in haraka sha2 shake blake; do
    for th in robust simple; do
      for sp in 128s 128f 192s 192f 256s 256f; do
        LIST="$LIST $be,$th,$sp"
      done
    done
  done
fi

for combo in $LIST; do
  tag=$(echo "$combo" | tr ',' '_')
  log="/tmp/test_${tag}.log"
  if ! timeout 600 cargo build --release --no-default-features --features "$combo" \
        > "$log" 2>&1; then
    echo "BUILDFAIL $combo -> $log"; FAIL=1; continue
  fi
  if timeout 600 cargo test --release --no-default-features --features "$combo" "$@" \
        >> "$log" 2>&1; then
    n=$(grep -c '^test .* ok$' "$log")
    echo "PASS $combo ($n tests)"
    PASSED=$((PASSED+1))
  else
    echo "FAIL $combo -> $log"
    grep -E '^(test .* FAILED|failures:|---- )' "$log" | head -20
    FAIL=1
  fi
done
echo "---- $PASSED combination(s) passed ----"
exit $FAIL
