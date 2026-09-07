#!/usr/bin/env bash
# Runs the full differential suite under every cargo feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are added later. With no [features] table the matrix
# is {default, --no-default-features, --all-features}, which is exercised
# explicitly rather than assumed to be redundant.
set -euo pipefail

crate="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$crate"

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S | re.M)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip()
            if name != 'default':
                print(name)
PY
)

echo "features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the combination list: default, no-default, all-features, then each
# individual feature and the full powerset when features exist.
COMBOS=("" "--no-default-features" "--all-features")
if ((${#FEATURES[@]} > 0)); then
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      ((mask & (1 << i))) && sel+=("${FEATURES[i]}")
    done
    COMBOS+=("--no-default-features --features $(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "=============================================================="
  echo "  cargo test $label"
  echo "=============================================================="

  # The cdylib under test must be rebuilt for each combination, since that is
  # the artifact the tests dlopen.
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --release $combo >/tmp/fc_build.log 2>&1; then
    echo "BUILD FAILED ($label)"; tail -20 /tmp/fc_build.log; fail=1; continue
  fi
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test $combo -- --test-threads=1 >/tmp/fc_test.log 2>&1; then
    echo "TESTS FAILED ($label)"; grep -E 'panicked|ROW |test result' /tmp/fc_test.log | head -40; fail=1; continue
  fi
  grep -E '^test result' /tmp/fc_test.log | sed 's/^/  /'
done

echo
if ((fail)); then
  echo "RESULT: at least one feature combination FAILED"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} feature combination(s) passed"
