#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Runs the full differential verification suite across EVERY cargo feature
# combination and BOTH build profiles.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so adding a feature later automatically widens the matrix.
#
# For each (feature-combo, profile):
#   1. cargo build   -- required, because `cargo test` does not emit the cdylib
#                       artifact for a `crate-type = ["cdylib"]` package.
#   2. nm -D parity  -- the C .so's exported symbols must all be in the Rust .so.
#   3. cargo test    -- Phase B (configs), Phase C (errors), Phase D (symbols),
#                       plus the exhaustive sweeps and equivalence proofs.
# ---------------------------------------------------------------------------
set -u
cd "$(dirname "$0")"

CARGO_OFFLINE=${CARGO_OFFLINE:---offline}

# --- enumerate feature combinations --------------------------------------- #
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
for f in feats:
    print(f)
PY
)

declare -a COMBOS=() LABELS=()
add_combo() { LABELS+=("$1"); COMBOS+=("$2"); }

add_combo "default"             ""
add_combo "no-default-features" "--no-default-features"
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  total=$(( 1 << n ))
  for ((mask = 0; mask < total; mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      (( mask & (1 << b) )) && sel+=("${FEATURES[$b]}")
    done
    joined=$(IFS=,; echo "${sel[*]:-}")
    if [ -z "$joined" ]; then
      add_combo "no-default-features (no features)" "--no-default-features"
    else
      add_combo "no-default-features + $joined" "--no-default-features --features $joined"
    fi
  done
else
  echo "note: Cargo.toml declares no [features]; the matrix is {default, --no-default-features}."
fi

# --- run the matrix -------------------------------------------------------- #
fail=0
for profile in "" "--release"; do
  pname=$([ -z "$profile" ] && echo debug || echo release)
  for i in "${!COMBOS[@]}"; do
    label="${LABELS[$i]}"
    flags="${COMBOS[$i]}"
    echo
    echo "==========================================================="
    echo "profile=$pname  features=[$label]  (flags: ${flags:-<none>})"
    echo "==========================================================="

    # shellcheck disable=SC2086
    if ! cargo build $CARGO_OFFLINE $profile $flags 2>&1 | tail -3; then
      echo "BUILD FAILED"; fail=1; continue
    fi

    # Symbol parity, checked outside the test harness as well.
    so_rust="target/$pname/libsynth_pair_lib.so"
    so_c=$(find "target/$pname/build" -name 'lib*.so' -path '*c_build*' 2>/dev/null | head -1)
    if [ -f "$so_rust" ] && [ -n "$so_c" ]; then
      missing=$(comm -23 \
        <(nm -D --defined-only "$so_c"    | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only "$so_rust" | awk '{print $NF}' | sort -u))
      if [ -n "$missing" ]; then
        echo "SYMBOL PARITY FAILED; missing from the Rust .so:"; echo "$missing"; fail=1
      else
        echo "symbol parity: OK ($(nm -D --defined-only "$so_c" | wc -l) C symbol(s), 0 missing)"
      fi
    else
      echo "WARN: could not locate both .so files for the symbol diff"
    fi

    # shellcheck disable=SC2086
    if ! cargo test $CARGO_OFFLINE $profile $flags 2>&1 \
         | grep -E '^(test result|running|error|warning: unused)' ; then
      echo "TEST INVOCATION PRODUCED NO SUMMARY"; fail=1; continue
    fi
    # shellcheck disable=SC2086
    cargo test $CARGO_OFFLINE $profile $flags >/dev/null 2>&1 || { echo "TESTS FAILED"; fail=1; }
  done
done

echo
echo "==========================================================="
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS x BOTH PROFILES: PASS"
else
  echo "FAILURES DETECTED"
fi
echo "==========================================================="
exit "$fail"
