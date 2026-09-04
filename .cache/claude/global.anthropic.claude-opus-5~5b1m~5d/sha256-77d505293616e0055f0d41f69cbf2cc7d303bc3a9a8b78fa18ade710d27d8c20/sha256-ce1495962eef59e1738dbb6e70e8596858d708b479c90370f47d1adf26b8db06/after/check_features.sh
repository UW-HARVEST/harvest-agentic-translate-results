#!/bin/bash
# Phase D — enumerate every Cargo feature combination and run the full
# differential suite under each one.
#
# This crate declares NO [features] section, so the only configuration is the
# default. The script proves that mechanically rather than by assertion: it
# extracts the feature list from Cargo.toml, builds the power set, and runs
# `cargo check` + the test suite for each combination.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"

# --- extract the [features] table (excluding the implicit "default")
FEATURES=$(python3 - <<'PY'
import re
s = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', s, re.M | re.S)
if not m:
    print('')
else:
    names = re.findall(r'^\s*([A-Za-z0-9_-]+)\s*=', m.group(1), re.M)
    print(' '.join(n for n in names if n != 'default'))
PY
)

echo "=== declared optional features: [${FEATURES:-<none>}] ==="

# --- build the power set of feature combinations
COMBOS=$(python3 - "$FEATURES" <<'PY'
import itertools, sys
feats = sys.argv[1].split()
if not feats:
    print('__default__')
else:
    # default build, then every subset with --no-default-features
    print('__default__')
    for k in range(len(feats) + 1):
        for c in itertools.combinations(feats, k):
            print(','.join(c) if c else '__none__')
PY
)

FAIL=0
for combo in $COMBOS; do
  if [ "$combo" = "__default__" ]; then
    ARGS=""
    LABEL="default features"
  elif [ "$combo" = "__none__" ]; then
    ARGS="--no-default-features"
    LABEL="--no-default-features"
  else
    ARGS="--no-default-features --features $combo"
    LABEL="--no-default-features --features $combo"
  fi

  echo
  echo "----------------------------------------------------------------"
  echo ">>> $LABEL"
  echo "----------------------------------------------------------------"

  if ! cargo check --offline $ARGS 2>&1 | grep -qv . ; then :; fi
  if cargo check --offline $ARGS 2>&1 | grep -E '^error'; then
    echo "!!! cargo check FAILED for: $LABEL"
    FAIL=1
    continue
  fi
  echo "cargo check: OK"

  # rebuild the cdylib the tests dlopen, then run the whole suite
  cargo build --offline --release $ARGS 2>&1 | grep -E '^error' && { FAIL=1; continue; }
  if RUST_MIN_STACK=67108864 cargo test --offline $ARGS -- --test-threads=1 \
       2>&1 | tee /dev/stderr | grep -qE 'test result: FAILED|^error'; then
    echo "!!! tests FAILED for: $LABEL"
    FAIL=1
  else
    echo "tests: OK for $LABEL"
  fi
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "=== ALL FEATURE COMBINATIONS PASSED ==="
else
  echo "=== SOME FEATURE COMBINATIONS FAILED ==="
fi
exit "$FAIL"
