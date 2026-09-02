#!/usr/bin/env bash
# Phase D: run cargo check + the full differential suite under EVERY feature
# combination, and against BOTH the debug and the release Rust .so.
#
# Feature names are extracted mechanically from Cargo.toml, so this stays
# correct if features are added later.
set -euo pipefail

CRATE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$CRATE"

# --- enumerate features from Cargo.toml -----------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name and name != 'default':
                print(name)
PY
)

# Powerset of the declared features, as --features arguments.
COMBOS=()
n=${#FEATURES[@]}
if (( n == 0 )); then
  COMBOS=("")                       # default (no features declared)
else
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do (( mask & (1<<i) )) && sel+=("${FEATURES[$i]}"); done
    COMBOS+=("$(IFS=,; echo "${sel[*]}")")
  done
fi

echo "declared features: ${FEATURES[*]:-<none>}"
echo "combinations to test: ${#COMBOS[@]} (plus an explicit --no-default-features run)"
echo

FAIL=0
run() {  # run <label> <extra cargo args...>
  local label="$1"; shift
  echo "----------------------------------------------------------------"
  echo ">>> $label"
  if ! timeout 600 cargo check --all-targets "$@" >/tmp/fc_check.log 2>&1; then
    echo "    cargo check FAILED"; tail -30 /tmp/fc_check.log; FAIL=1; return
  fi
  echo "    cargo check ok"

  # Build and test against BOTH profiles: the debug build has bounds and
  # overflow checks enabled, the release build has them off, so both must be
  # exercised.
  for profile in debug release; do
    local relflag=()
    [[ $profile == release ]] && relflag=(--release)
    if ! timeout 600 cargo build "${relflag[@]}" "$@" >/tmp/fc_build.log 2>&1; then
      echo "    [$profile] cargo build FAILED"; tail -30 /tmp/fc_build.log; FAIL=1; continue
    fi
    local so="$CRATE/target/$profile/libhalf2float_lib.so"
    if [[ ! -f $so ]]; then
      echo "    [$profile] MISSING $so"; FAIL=1; continue
    fi
    if HALF2FLOAT_RUST_SO="$so" timeout 600 cargo test "${relflag[@]}" "$@" \
         >/tmp/fc_test_$profile.log 2>&1; then
      echo "    [$profile] tests ok: $(grep -c '^test .* ok$' /tmp/fc_test_$profile.log) passed"
    else
      echo "    [$profile] tests FAILED"; tail -40 /tmp/fc_test_$profile.log; FAIL=1
    fi
  done
}

for combo in "${COMBOS[@]}"; do
  if [[ -z $combo ]]; then
    run "features: <default>"
  else
    run "features: $combo" --no-default-features --features "$combo"
  fi
done
run "features: --no-default-features" --no-default-features

echo "================================================================"
if (( FAIL )); then echo "FEATURE MATRIX: FAILED"; exit 1; else echo "FEATURE MATRIX: ALL OK"; fi
