#!/usr/bin/env bash
# Phase D driver: run cargo check + the whole differential suite for EVERY
# valid feature combination, in both the dev and the release profile.
#
# `Cargo.toml` has no [features] section, so the complete set of valid feature
# combinations is the single empty combination; the loop below derives that
# mechanically rather than hard-coding it, so it keeps working if features are
# added later.
set -uo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(python3 - <<'PY'
import re, itertools, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        k = line.split('=', 1)[0].strip().strip('"')
        if k != 'default':
            names.append(k)
for n in range(len(names) + 1):
    for combo in itertools.combinations(names, n):
        print(','.join(combo))
PY
)

echo "feature combinations to verify: ${#FEATURES[@]}"
rc=0
for combo in "${FEATURES[@]}"; do
  label="${combo:-<none>}"
  for profile in dev release; do
    flag=""; [ "$profile" = release ] && flag="--release"
    echo
    echo "=============================================================="
    echo "  features = ${label}   profile = ${profile}"
    echo "=============================================================="
    if ! timeout 600 cargo check --no-default-features --features "$combo" $flag 2>&1 | tail -5; then
      echo "CHECK FAILED (features=${label}, profile=${profile})"; rc=1; continue
    fi
    # Force a fresh cdylib so the differential suite can never load a stale .so,
    # and also produce the uplifted target/<profile>/libagglom_lib.so that
    # `nm -D` comparisons use.
    touch src/lib.rs
    timeout 600 cargo build --no-default-features --features "$combo" $flag 2>&1 | tail -1
    if ! timeout 600 cargo test --no-default-features --features "$combo" $flag 2>&1 \
         | grep -E "test result|FAILED|panicked|^error"; then
      echo "TEST RUN PRODUCED NO RESULT (features=${label}, profile=${profile})"; rc=1
    fi
    if timeout 600 cargo test --no-default-features --features "$combo" $flag 2>&1 \
       | grep -q "test result: FAILED"; then
      echo "TESTS FAILED (features=${label}, profile=${profile})"; rc=1
    fi
  done
done
echo
[ $rc -eq 0 ] && echo "ALL FEATURE COMBINATIONS x PROFILES PASSED" || echo "SOME COMBINATIONS FAILED"
exit $rc
