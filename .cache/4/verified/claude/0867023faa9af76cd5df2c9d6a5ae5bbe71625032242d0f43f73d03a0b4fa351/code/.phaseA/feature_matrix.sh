#!/usr/bin/env bash
# Enumerate every valid Cargo feature combination and run `cargo check`
# (and optionally `cargo test`) for each.
#
#   .phaseA/feature_matrix.sh check      # cargo check for every combination
#   .phaseA/feature_matrix.sh test       # cargo test  for every combination
#
# Features are read out of Cargo.toml's [features] section, so this stays
# correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

MODE="${1:-check}"

# --- extract feature names from the [features] table -----------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "");
      gsub(/[[:space:]]/, "");
      if ($0 != "default") print
    }
  ' Cargo.toml
)

DEFAULT_PRESENT=$(awk '/^\[features\]/{inf=1;next} /^\[/{inf=0} inf && /^[[:space:]]*default[[:space:]]*=/{print "yes"}' Cargo.toml)

echo "=== Build-time configuration surface ==="
echo "Cargo [features] declared : ${#FEATURES[@]} ${FEATURES[*]:-(none)}"
echo "default feature set       : ${DEFAULT_PRESENT:-(none)}"
echo "cfg(feature = ...) in src : $(grep -rc 'cfg(feature' src/ 2>/dev/null | grep -v ':0$' | wc -l) files"
echo

# --- build the combination list (power set) --------------------------------
COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  # Only one configuration exists: the empty feature set.
  COMBOS+=("")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=== ${#COMBOS[@]} feature combination(s) to verify ==="
fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  printf '\n--- %s --no-default-features --features "%s" ---\n' "$MODE" "$combo"
  if [ -z "$combo" ]; then
    timeout 900 cargo "$MODE" --no-default-features 2>&1 | tail -n 25
  else
    timeout 900 cargo "$MODE" --no-default-features --features "$combo" 2>&1 | tail -n 25
  fi
  rc=${PIPESTATUS[0]}
  if [ "$rc" -ne 0 ]; then
    echo "!!! FAILED: $label (exit $rc)"
    fail=1
  else
    echo "OK: $label"
  fi
done

# Also verify the plain default build (identical to <no features> when the
# crate declares none, but check it explicitly so the claim is tested).
printf '\n--- %s (default features) ---\n' "$MODE"
timeout 900 cargo "$MODE" 2>&1 | tail -n 15
rc=${PIPESTATUS[0]}
if [ "$rc" -ne 0 ]; then
  echo "!!! FAILED: default"
  fail=1
else
  echo "OK: default"
fi

exit "$fail"
