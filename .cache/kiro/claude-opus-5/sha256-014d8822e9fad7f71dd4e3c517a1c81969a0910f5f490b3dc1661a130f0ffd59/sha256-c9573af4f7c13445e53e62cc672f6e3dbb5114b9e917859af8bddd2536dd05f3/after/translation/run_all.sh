#!/usr/bin/env bash
# Run the full differential suite across every feature combination and both
# cdylib build profiles. Feature combos are extracted from Cargo.toml rather than
# hard-coded, so adding a feature later automatically widens the matrix.
set -uo pipefail
cd "$(dirname "$0")"

# ---- enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.M | re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line = line.strip()
    if not line or line.startswith('#'):
        continue
    name = line.split('=')[0].strip()
    if name and name != 'default':
        print(name)
PY
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-（none）}"

# ---- build the combination list --------------------------------------------
# Always include: default features, and no-default-features. Then every subset
# of the declared features (power set), which for an empty feature list reduces
# to just the two baseline runs.
COMBOS=()
COMBOS+=("default|")                  # plain `cargo test`
COMBOS+=("nodefault|")                # `cargo test --no-default-features`

n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    sel=()
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then sel+=("${FEATURES[b]}"); fi
    done
    joined=$(IFS=,; echo "${sel[*]}")
    COMBOS+=("nodefault|$joined")
    COMBOS+=("default|$joined")
  done
fi

# ---- run the matrix ---------------------------------------------------------
fail=0
run=0
for profile in release debug; do
  for combo in "${COMBOS[@]}"; do
    kind="${combo%%|*}"
    feats="${combo#*|}"

    args=(test --release)
    env_no_default=""
    if [ "$kind" = "nodefault" ]; then
      args+=(--no-default-features)
      env_no_default="1"
    fi
    if [ -n "$feats" ]; then
      args+=(--features "$feats")
    fi

    label="cdylib=$profile $kind${feats:+ features=$feats}"
    printf '=== %s\n' "$label"

    if [ -n "$env_no_default" ]; then
      FFI_TEST_PROFILE="$profile" FFI_TEST_NO_DEFAULT_FEATURES=1 \
        FFI_TEST_FEATURES="$feats" timeout 600 cargo "${args[@]}" 2>&1 | tail -4
    else
      FFI_TEST_PROFILE="$profile" FFI_TEST_FEATURES="$feats" \
        timeout 600 cargo "${args[@]}" 2>&1 | tail -4
    fi
    status=${PIPESTATUS[0]}
    run=$((run + 1))
    if [ "$status" -ne 0 ]; then
      echo "FAILED: $label (exit $status)"
      fail=$((fail + 1))
    fi
  done
done

echo
echo "configurations run: $run, failed: $fail"
[ "$fail" -eq 0 ] || exit 1
echo "ALL CONFIGURATIONS PASS"
