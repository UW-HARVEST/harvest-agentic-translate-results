#!/usr/bin/env bash
# Phase D — run the full differential suite under every feature combination and
# both profiles. Feature combinations are extracted from Cargo.toml, not guessed.
set -uo pipefail
cd "$(dirname "$0")"

# --- extract the declared features (excluding "default") -------------------
features=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+ *=/ {gsub(/ *=.*/,""); if ($0 != "default") print}
' Cargo.toml)

combos=()
if [[ -z "$features" ]]; then
  echo "== Cargo.toml declares no [features]; the only configurations are the"
  echo "== default (empty) set and --no-default-features."
  combos+=("")                        # default
  combos+=("--no-default-features")   # explicit empty
else
  # power set of the declared features
  feats=($features)
  n=${#feats[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${feats[i]}")
    done
    if ((${#sel[@]} == 0)); then
      combos+=("--no-default-features")
    else
      combos+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    fi
  done
  combos+=("") # plus the default set
fi

fail=0
for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    label="profile='${profile:-debug}' combo='${combo:-<default features>}'"
    echo "=================================================================="
    echo "== $label"
    echo "=================================================================="
    # the cdylib must be rebuilt for this exact configuration before the tests
    # dlopen it
    if ! timeout 600 cargo build $profile $combo >/dev/null 2>&1; then
      echo "BUILD FAILED: $label"; fail=1; continue
    fi
    if ! timeout 600 cargo check $profile $combo --all-targets >/dev/null 2>&1; then
      echo "CHECK FAILED: $label"; fail=1; continue
    fi
    if timeout 600 cargo test $profile $combo 2>&1 | tee /dev/stderr | grep -qE '^test result: FAILED'; then
      echo "TESTS FAILED: $label"; fail=1
    else
      echo "TESTS PASSED: $label"
    fi
  done
done

echo "=================================================================="
if ((fail)); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi
echo "RESULT: all configurations PASSED"
