#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml (via cargo
# metadata, so nothing is hard-coded) and run cargo check + the full
# differential suite for each one. Also runs the debug-profile cdylib.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(cd "$here/.." && pwd)"
cd "$crate"

ulimit -c 0 || true   # the null-pointer tests fork children that SIGSEGV

# --- enumerate declared features -------------------------------------------
mapfile -t FEATURES < <(
  cargo metadata --no-deps --format-version 1 2>/dev/null \
  | python3 -c '
import json,sys
m=json.load(sys.stdin)
for p in m["packages"]:
    if p["name"]=="translation":
        for f in sorted(p["features"]):
            if f!="default":
                print(f)
'
)

echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]-}"

# Build the power set of declared features (plus the default build).
COMBOS=("--all-features" "" "--no-default-features")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    sel=()
    for (( i=0; i<n; i++ )); do
      (( mask & (1<<i) )) && sel+=("${FEATURES[i]}")
    done
    joined="$(IFS=,; echo "${sel[*]-}")"
    COMBOS+=("--no-default-features --features $joined")
  done
fi

# Deduplicate.
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default>}"
  echo
  echo "############ feature combo: $label ############"
  # shellcheck disable=SC2086
  if ! cargo check $combo >/dev/null 2>&1; then
    echo "  cargo check FAILED"; fail=1; continue
  fi
  echo "  cargo check ok"

  for profile in release debug; do
    pflag=""; [[ $profile == release ]] && pflag="--release"
    # The cdylib must be rebuilt explicitly: `cargo test` does not build it.
    # shellcheck disable=SC2086
    if ! cargo build $pflag $combo >/dev/null 2>&1; then
      echo "  [$profile] cargo build FAILED"; fail=1; continue
    fi
    so="$crate/target/$profile/libhsv_to_rgb_lib.so"
    # shellcheck disable=SC2086
    if HARVEST_RUST_SO="$so" cargo test $pflag $combo -- --test-threads=8 >/tmp/ft.$$ 2>&1; then
      npass="$(grep -hoE '[0-9]+ passed' /tmp/ft.$$ | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')"
      echo "  [$profile] tests: $npass passed  ✅"
    else
      echo "  [$profile] tests FAILED"; tail -30 /tmp/ft.$$; fail=1
    fi
  done
done
rm -f /tmp/ft.$$
echo
if (( fail )); then echo "RESULT: FAILURES"; exit 1; fi
echo "RESULT: all feature combinations pass ✅"
