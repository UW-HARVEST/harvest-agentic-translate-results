#!/usr/bin/env bash
# Phase A/D: enumerate EVERY valid Cargo feature combination and `cargo check`
# each one. The feature list is read from Cargo.toml via `cargo metadata`, so
# this keeps working if features are ever added.
set -euo pipefail
cd "$(dirname "$0")/.."

mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 |
    python3 -c '
import json,sys
md = json.load(sys.stdin)
feats = set()
for p in md["packages"]:
    if p["name"] == "driver":
        feats.update(f for f in p["features"] if f != "default")
print("\n".join(sorted(feats)))
'
)

# Drop a single empty element produced when there are no features at all.
if [ "${#FEATURES[@]}" -eq 1 ] && [ -z "${FEATURES[0]}" ]; then
  FEATURES=()
fi

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

n=${#FEATURES[@]}
combos=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((i = 0; i < n; i++)); do
    if ((mask & (1 << i))); then combo+=("${FEATURES[$i]}"); fi
  done
  combos+=("$(
    IFS=,
    echo "${combo[*]}"
  )")
done

fail=0
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  for extra in "--no-default-features" ""; do
    echo "=== cargo check ${extra} --features '${combo}'  (${label}) ==="
    # shellcheck disable=SC2086
    if ! cargo check --all-targets ${extra} --features "${combo}" 2>&1 | tail -n 3; then
      echo "FAILED: ${extra} --features ${combo}"
      fail=1
    fi
  done
done

echo "=== cargo check --all-features ==="
cargo check --all-targets --all-features 2>&1 | tail -n 3

if [ "$fail" -ne 0 ]; then
  echo "FEATURE CHECK FAILED"
  exit 1
fi
echo "ALL FEATURE COMBINATIONS CHECK CLEAN"
