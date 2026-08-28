#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under EVERY cargo feature
# combination, plus both cdylib optimisation levels.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so a
# future [features] table is picked up automatically.
set -uo pipefail
cd "$(dirname "$0")"
: "${TMPDIR:=/tmp}"

# --- enumerate features ------------------------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name != 'default':
                print(name)
PY
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- build the combo list ----------------------------------------------------
declare -a COMBOS=()
COMBOS+=("")                              # default feature set
COMBOS+=("--no-default-features")
COMBOS+=("--all-features")
n=${#FEATURES[@]}
if (( n > 0 && n <= 12 )); then
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (( mask & (1 << b) )) && sel+=("${FEATURES[$b]}")
    done
    if ((${#sel[@]})); then
      COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    else
      COMBOS+=("--no-default-features")
    fi
  done
fi
# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

echo "feature combinations to verify: ${#COMBOS[@]}"
echo

fail=0

# The suite must report at least this many passing tests; guards against a
# silently-empty run being reported as PASS.
MIN_TESTS=55

run() { # <label> <combo-args> [extra env assignments...]
  local label="$1"; shift
  local combo="$1"; shift
  printf '=== %-52s ' "$label"
  # shellcheck disable=SC2086
  if env "$@" FFI_FEATURE_ARGS="$combo" \
       timeout 600 cargo test --offline $combo \
       >"$TMPDIR/feat.log" 2>&1; then
    local passed failed
    passed=$(awk '/test result:/ { for (i = 1; i < NF; i++) if ($(i+1) ~ /^passed/) s += $i } END { print s + 0 }' "$TMPDIR/feat.log")
    failed=$(awk '/test result:/ { for (i = 1; i < NF; i++) if ($(i+1) ~ /^failed/) s += $i } END { print s + 0 }' "$TMPDIR/feat.log")
    if [[ ${passed:-0} -lt $MIN_TESTS || ${failed:-1} -ne 0 ]]; then
      echo "FAIL (only ${passed:-0} passed, ${failed:-?} failed; expected >= $MIN_TESTS passing)"
      grep -E 'test result|FAILED' "$TMPDIR/feat.log" | tail -20
      fail=$((fail + 1))
    else
      echo "PASS ($passed tests)"
    fi
  else
    echo "FAIL"
    tail -n 40 "$TMPDIR/feat.log"
    fail=$((fail + 1))
  fi
}

# 1. cargo check for every combo (fast compile gate)
for combo in "${COMBOS[@]}"; do
  printf -- '--- cargo check %-44s ' "${combo:-(default)}"
  # shellcheck disable=SC2086
  if timeout 600 cargo check --offline -q --all-targets $combo >"$TMPDIR/chk.log" 2>&1; then
    echo OK
  else
    echo FAIL; tail -n 30 "$TMPDIR/chk.log"; fail=$((fail + 1))
  fi
done
echo

# 2. full suite for every combo, release cdylib (the shipped artifact)
for combo in "${COMBOS[@]}"; do
  run "release cdylib | ${combo:-(default)}" "$combo"
done
echo

# 3. full suite against a DEBUG-built cdylib: different codegen, same required
#    bit-exact behaviour. Catches any reliance on release-only optimisation.
for combo in "${COMBOS[@]}"; do
  dir="target/ffi-debug"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --offline -q --target-dir "$dir" $combo \
        >"$TMPDIR/dbg.log" 2>&1; then
    echo "=== debug cdylib build FAILED for ${combo:-(default)}"; tail -20 "$TMPDIR/dbg.log"
    fail=$((fail + 1)); continue
  fi
  so="$PWD/$dir/debug/libto_barycentric_lib.so"
  if [[ ! -f $so ]]; then
    echo "=== debug cdylib missing: $so"; fail=$((fail + 1)); continue
  fi
  run "debug cdylib   | ${combo:-(default)}" "$combo" "RUST_SO_PATH=$so"
done

echo
if [[ $fail -eq 0 ]]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "$fail configuration(s) FAILED"
fi
exit $fail
