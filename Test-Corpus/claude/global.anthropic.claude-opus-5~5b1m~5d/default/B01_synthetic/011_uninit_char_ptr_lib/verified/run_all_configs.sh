#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY feature combination and
# every build profile. Cargo.toml declares no [features] table, so the feature
# powerset is {default} == {no-default-features}; both are still exercised, plus
# debug and release (release adds panic="abort", a different codegen path).
set -uo pipefail
cd "$(dirname "$0")"

FEATS=$(cargo metadata --offline --format-version 1 --no-deps \
        | python3 -c 'import json,sys; print(" ".join(json.load(sys.stdin)["packages"][0]["features"]))')
echo "declared features: [${FEATS:-none}]"

COMBOS=("")                       # default
COMBOS+=("--no-default-features")  # empty feature set
for f in $FEATS; do
  COMBOS+=("--no-default-features --features $f")
  COMBOS+=("--features $f")
done

fail=0
for prof in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test --offline ${prof} ${combo}"
    # The Rust .so under test must be the one for THIS profile/feature combo.
    out=$(timeout 600 cargo test --offline $prof $combo 2>&1)
    if grep -qE '^test result: FAILED|^error' <<<"$out"; then
      echo "FAIL  $label"
      grep -E 'DIVERGENCE|panicked|^test result:|^error' <<<"$out" | head -20
      fail=1
    else
      echo "PASS  $label  ($(grep -cE '\.\.\. ok$' <<<"$out") tests)"
    fi
  done
done
exit $fail
