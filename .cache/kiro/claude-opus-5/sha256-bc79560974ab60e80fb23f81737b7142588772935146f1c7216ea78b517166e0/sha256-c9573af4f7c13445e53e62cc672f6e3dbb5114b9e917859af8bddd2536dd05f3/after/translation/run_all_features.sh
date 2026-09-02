#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY cargo feature
# combination declared in Cargo.toml.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.split('#', 1)[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=', 1)[0].strip().strip('"')
    if name and name != 'default':
        print(name)
PY
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} -> ${FEATURES[*]-<none>}"

run() {
  local label="$1"; shift
  echo "=============================================================="
  echo "== $label"
  echo "=============================================================="
  if ! timeout 600 cargo test --release "$@" 2>&1 | grep -E 'test result|FAILED|^error'; then
    echo "FAILED: $label"
    return 1
  fi
}

fail=0

# Default feature set.
run "default features" || fail=1

n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  echo
  echo "No [features] table: the default (empty) feature set is the only"
  echo "configuration, and it has been verified above."
else
  # Full power set of the declared features.
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo+=("${FEATURES[i]}"); fi
    done
    joined=$(IFS=,; echo "${combo[*]}")
    run "--no-default-features --features '${joined}'" \
      --no-default-features ${joined:+--features "$joined"} || fail=1
  done
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$fail"
