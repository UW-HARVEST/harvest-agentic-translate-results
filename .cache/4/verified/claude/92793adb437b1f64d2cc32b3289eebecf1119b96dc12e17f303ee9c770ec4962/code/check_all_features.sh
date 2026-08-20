#!/usr/bin/env bash
# Enumerate every valid feature combination declared in Cargo.toml and run
# `cargo check` (or an arbitrary cargo subcommand) for each one.
#
# Usage:
#   ./check_all_features.sh            # cargo check for every combo
#   ./check_all_features.sh test       # cargo test  for every combo
set -uo pipefail

CMD="${1:-check}"
shift || true

cd "$(dirname "$0")"

# ---- enumerate declared features (excluding "default") -----------------------
mapfile -t FEATURES < <(
  cargo metadata --offline --no-deps --format-version 1 2>/dev/null |
  python3 -c '
import json,sys
m=json.load(sys.stdin)
for p in m["packages"]:
    if p["name"]=="driver":
        for f in sorted(p["features"]):
            if f!="default":
                print(f)
'
)

n=${#FEATURES[@]}
echo "declared non-default features: $n  (${FEATURES[*]:-<none>})"

COMBOS=()
# power set of FEATURES
for ((mask=0; mask<(1<<n); mask++)); do
  combo=""
  for ((i=0; i<n; i++)); do
    if (( mask & (1<<i) )); then
      combo="${combo:+$combo,}${FEATURES[$i]}"
    fi
  done
  COMBOS+=("$combo")
done

fail=0

# Both optimisation profiles are exercised: `debug-assertions`/`overflow-checks`
# and the optimiser itself can change observable behaviour at the FFI boundary
# (see the note in Cargo.toml), so parity must hold in each.
for profile in dev release; do
  prof_flag=()
  [[ $profile == release ]] && prof_flag=(--release)

  for combo in "${COMBOS[@]}"; do
    label="${combo:-<empty>}"
    echo "=== [$profile] cargo $CMD --no-default-features --features '$label' ==="
    if [[ -z "$combo" ]]; then
      timeout 600 cargo "$CMD" --offline "${prof_flag[@]}" --no-default-features "$@" 2>&1 | tail -25
    else
      timeout 600 cargo "$CMD" --offline "${prof_flag[@]}" --no-default-features \
        --features "$combo" "$@" 2>&1 | tail -25
    fi
    rc=${PIPESTATUS[0]}
    echo "--- rc=$rc for [$profile] combo '$label'"
    (( rc != 0 )) && fail=1
  done

  # and the default feature set
  echo "=== [$profile] cargo $CMD (default features) ==="
  timeout 600 cargo "$CMD" --offline "${prof_flag[@]}" "$@" 2>&1 | tail -25
  rc=${PIPESTATUS[0]}
  echo "--- rc=$rc for [$profile] default"
  (( rc != 0 )) && fail=1
done

echo
if (( fail )); then echo "RESULT: FAILURE"; else echo "RESULT: ALL COMBINATIONS OK"; fi
exit $fail
