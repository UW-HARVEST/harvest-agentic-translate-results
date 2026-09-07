#!/bin/bash
# Run `cargo <cmd>` for every valid feature combination.
# Valid combos = backend x thash x secpar (mirrors the CMake cache variables).
cd "$(dirname "$0")/translation" || exit 1
CMD=${1:-check}
shift
FAIL=0
for be in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      combo="$be,$th,$sp"
      if timeout 600 cargo "$CMD" --no-default-features --features "$combo" "$@" > "/tmp/co_${be}_${th}_${sp}.log" 2>&1; then
        echo "PASS $combo"
      else
        echo "FAIL $combo  (see /tmp/co_${be}_${th}_${sp}.log)"
        FAIL=1
      fi
    done
  done
done
exit $FAIL
