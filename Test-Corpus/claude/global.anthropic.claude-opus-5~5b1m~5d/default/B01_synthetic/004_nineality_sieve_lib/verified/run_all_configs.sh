#!/usr/bin/env bash
# Phase D: run the full differential suite under every build configuration.
#
# Feature combinations are extracted from Cargo.toml rather than hard-coded, so
# a newly added feature is picked up automatically. The crate currently has no
# [features] table, so the combination set is just the default one; the debug
# and release profiles are exercised regardless because `panic = "abort"` and
# optimisation only apply to release.
set -uo pipefail
cd "$(dirname "$0")"

# --- discover feature axes -------------------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name != 'default':
                print(name)
PY
)

# --- build the combination list --------------------------------------------
COMBOS=("")                          # default features
if [ "${#FEATURES[@]}" -gt 0 ]; then
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  # pairwise combinations
  n=${#FEATURES[@]}
  for ((i = 0; i < n; i++)); do
    for ((j = i + 1; j < n; j++)); do
      COMBOS+=("--no-default-features --features ${FEATURES[$i]},${FEATURES[$j]}")
    done
  done
else
  echo "note: Cargo.toml declares no [features]; verifying the single default configuration"
  COMBOS+=("--no-default-features")   # must be a no-op, assert it still works
  COMBOS+=("--all-features")
fi

rc=0
for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${profile:-<debug>} ${combo:-<default-features>}"
    printf '=== %s\n' "$label"
    # shellcheck disable=SC2086
    out=$(timeout 600 cargo test --offline $profile $combo 2>&1)
    status=$?
    echo "$out" | grep -E '^test result:|^error' | sed 's/^/    /'
    if [ $status -ne 0 ]; then
      echo "    FAILED: $label"
      echo "$out" | tail -40 | sed 's/^/    | /'
      rc=1
    fi
  done
done

echo
if [ $rc -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $rc
