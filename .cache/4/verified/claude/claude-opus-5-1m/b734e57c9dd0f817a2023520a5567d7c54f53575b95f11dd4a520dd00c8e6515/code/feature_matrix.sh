#!/usr/bin/env bash
# Phase D: enumerate EVERY valid build-time feature combination from Cargo.toml
# and run `cargo check` for each, then (with --test) the whole differential suite.
#
#   ./feature_matrix.sh          # cargo check every combination
#   ./feature_matrix.sh --test   # also run the full differential suite per combination
#
# The enumeration is mechanical: it parses the [features] table out of Cargo.toml
# and takes the power set of the non-default features, so a feature added later is
# picked up automatically instead of silently escaping the matrix.
set -euo pipefail
cd "$(dirname "$0")"

TD="${TMPDIR:-/tmp}"
mkdir -p "$TD"

python3 - > "$TD/combos.txt" <<'PY'
import itertools, re, sys

text = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', text, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != "default":
            feats.append(name)

if not feats:
    # No [features] table at all: there is exactly ONE valid configuration.
    print("")           # the empty feature set
else:
    for r in range(len(feats) + 1):
        for combo in itertools.combinations(feats, r):
            print(",".join(combo))
sys.stderr.write("features declared: %r\n" % (feats,))
PY

NCOMBO=$(wc -l < "$TD/combos.txt")
echo "=== $NCOMBO valid feature combination(s) enumerated from Cargo.toml ==="
nl -ba "$TD/combos.txt" | sed 's/^\( *[0-9]*\)\t$/\1\t<none (default)>/'
echo

FAIL=0
while IFS= read -r combo; do
  label="${combo:-<none>}"
  echo "----------------------------------------------------------------"
  echo "=== cargo check --no-default-features --features '$label' ==="
  if [ -z "$combo" ]; then
    ARGS=(--offline --no-default-features)
  else
    ARGS=(--offline --no-default-features --features "$combo")
  fi
  if cargo check "${ARGS[@]}" 2>&1 | tail -20; then
    echo "    check OK"
  else
    echo "    CHECK FAILED for features='$label'"
    FAIL=1
  fi
  # the tests link against the cdylib, so type-check them too
  if cargo check "${ARGS[@]}" --tests 2>&1 | tail -20; then
    echo "    check --tests OK"
  else
    echo "    CHECK --tests FAILED for features='$label'"
    FAIL=1
  fi

  if [ "${1:-}" = "--test" ]; then
    echo "=== full differential suite for features='$label' ==="
    if FEATURES="$combo" ./run_difftests.sh; then
      echo "    suite OK"
    else
      echo "    SUITE FAILED for features='$label'"
      FAIL=1
    fi
  fi
done < "$TD/combos.txt"

echo "----------------------------------------------------------------"
if [ "$FAIL" = "0" ]; then
  echo "ALL $NCOMBO feature combination(s) OK"
else
  echo "FAILURES present"
  exit 1
fi
