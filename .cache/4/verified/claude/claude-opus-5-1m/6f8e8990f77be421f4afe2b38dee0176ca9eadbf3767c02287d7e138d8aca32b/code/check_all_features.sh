#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml (the powerset of
# [features] keys, minus "default") and run `cargo check` for each.
#
# Usage: ./check_all_features.sh [cargo-subcommand ...]     (default: check)
set -uo pipefail
cd "$(dirname "$0")"

CMD=("${@:-check}")

# --- extract feature names mechanically from Cargo.toml -----------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

N=${#FEATURES[@]}
echo "features declared in Cargo.toml: $N ${FEATURES[*]:-(none)}"
echo "feature combinations to check:  $((1 << N))"
echo

FAIL=0
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
  done
  label="${combo:-<none>}"
  printf '==== %s --no-default-features --features "%s" ====\n' "${CMD[*]}" "$label"
  if timeout 600 cargo "${CMD[@]}" --offline --all-targets --no-default-features \
       ${combo:+--features "$combo"}; then
    echo "PASS: $label"
  else
    echo "FAIL: $label"
    FAIL=1
  fi
  echo
done

# `default` is a distinct configuration even when it is empty/absent.
printf '==== %s (default features) ====\n' "${CMD[*]}"
if timeout 600 cargo "${CMD[@]}" --offline --all-targets; then
  echo "PASS: default"
else
  echo "FAIL: default"
  FAIL=1
fi

exit $FAIL
