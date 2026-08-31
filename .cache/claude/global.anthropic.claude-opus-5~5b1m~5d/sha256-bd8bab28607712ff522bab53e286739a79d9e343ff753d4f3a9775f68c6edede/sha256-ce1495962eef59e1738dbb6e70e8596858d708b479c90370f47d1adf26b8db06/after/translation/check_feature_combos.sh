#!/usr/bin/env bash
# Phase D: verify EVERY cargo feature combination builds and passes its tests.
#
# This crate declares no [features] and contains no #[cfg(feature = ...)], so the
# feature powerset is the single empty set -- `--no-default-features`,
# `--all-features` and the default build are the same configuration. We PROVE
# that here (by enumerating from Cargo.toml and grepping src/) rather than
# assuming it, and we still run all three invocations.
set -uo pipefail
cd "$(dirname "$0")"

echo "===== declared [features] in Cargo.toml ====="
FEATS=$(python3 - <<'PY'
import re
t = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', t, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            print(line.split('=')[0].strip())
PY
)
if [ -z "$FEATS" ]; then echo "(none declared)"; else echo "$FEATS"; fi

echo
echo "===== cfg(feature = ...) occurrences in src/ ====="
if grep -rn 'cfg(feature' src/; then :; else echo "(none)"; fi

echo
echo "===== any #[cfg(...)] at all in src/ ====="
if grep -rn '#\[cfg(' src/; then :; else echo "(none)"; fi

STATUS=0

run_cfg() {
  local label="$1"; shift
  echo
  echo "---------- $label ----------"
  if ! timeout 600 cargo build --release "$@" > build.$$.log 2>&1; then
    echo "BUILD FAILED: $label"
    tail -25 build.$$.log
    STATUS=1
    rm -f build.$$.log
    return
  fi
  rm -f build.$$.log
  if timeout 600 cargo test --release "$@" > test.$$.log 2>&1; then
    grep -E '^test result' test.$$.log | sed 's/^/    /'
    echo "PASS: $label"
  else
    echo "TEST FAILED: $label"
    grep -E '^test result|^failures:|^error' test.$$.log | head -30 | sed 's/^/    /'
    STATUS=1
  fi
  rm -f test.$$.log
}

run_cfg "default features"
run_cfg "--no-default-features" --no-default-features
run_cfg "--all-features" --all-features

# General powerset loop (a no-op while FEATS is empty, correct if features are added).
if [ -n "$FEATS" ]; then
  # shellcheck disable=SC2206
  LIST=($FEATS)
  N=${#LIST[@]}
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if ((mask & (1 << i))); then combo="$combo,${LIST[$i]}"; fi
    done
    combo="${combo#,}"
    run_cfg "--no-default-features --features '$combo'" \
      --no-default-features --features "$combo"
  done
fi

echo
if [ $STATUS -eq 0 ]; then
  echo "===== ALL CONFIGURATIONS PASS ====="
else
  echo "===== SOME CONFIGURATIONS FAILED ====="
fi
exit $STATUS
