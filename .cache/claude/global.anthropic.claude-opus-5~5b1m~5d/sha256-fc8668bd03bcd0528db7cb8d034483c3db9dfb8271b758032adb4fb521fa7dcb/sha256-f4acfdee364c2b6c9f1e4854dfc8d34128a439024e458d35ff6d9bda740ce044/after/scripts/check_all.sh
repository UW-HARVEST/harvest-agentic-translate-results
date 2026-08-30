#!/bin/bash
# cargo check every feature combination.
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
cd "$W/translation" || exit 1
fail=0
while IFS= read -r combo; do
  out=$(timeout 300 cargo check --quiet --no-default-features --features "$combo" 2>&1)
  rc=$?
  if [ $rc -ne 0 ]; then
    echo "FAIL  $combo"
    echo "$out" | head -30
    fail=1
  else
    echo "ok    $combo"
  fi
done < <("$W/scripts/all_combos.sh")
exit $fail
