#!/usr/bin/env bash
# Phase D — run the whole differential suite under EVERY feature combination.
#
# Features are extracted mechanically from Cargo.toml rather than hard-coded, so
# this keeps working if a [features] table is added later.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] section of Cargo.toml.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)

echo "=== discovered features: ${#FEATURES[@]} ${FEATURES[*]:-(none)} ==="

# Build the list of configurations to test: default, no-default, and every
# subset of the discovered features (power set).
CONFIGS=("--all-features" "")            # "" == default features
CONFIGS+=("--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ] && [ "$n" -le 12 ]; then
  total=$(( (1 << n) - 1 ))
  for ((mask=1; mask<=total; mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( (mask >> i) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    CONFIGS+=("--no-default-features --features $combo")
  done
fi

fail=0
for cfg in "${CONFIGS[@]}"; do
  label="${cfg:-<default features>}"
  echo
  echo "--- cargo check  [$label]"
  # shellcheck disable=SC2086
  if ! timeout 300 cargo check $cfg >/dev/null 2>&1; then
    echo "CHECK FAILED  [$label]"; fail=1; continue
  fi
  echo "--- cargo build --release  [$label]"
  # shellcheck disable=SC2086
  if ! timeout 300 cargo build --release $cfg >/dev/null 2>&1; then
    echo "BUILD FAILED  [$label]"; fail=1; continue
  fi

  # Symbol parity must hold in EVERY configuration.
  missing=$(comm -23 \
    <(nm -D --defined-only --format=posix ../c_src/build/*.so | awk '{print $1}' | sort -u) \
    <(nm -D --defined-only --format=posix target/release/libupdate_frame_header_lib.so | awk '{print $1}' | sort -u))
  if [ -n "$missing" ]; then
    echo "SYMBOL DIFF NOT EMPTY  [$label]: $missing"; fail=1
  else
    echo "symbol diff empty  [$label]"
  fi

  echo "--- cargo test --release  [$label]"
  # shellcheck disable=SC2086
  if timeout 600 cargo test --release $cfg --tests 2>&1 | tee /tmp/ft.log | grep -q '^test result: FAILED'; then
    echo "TESTS FAILED  [$label]"; grep -E '^(test result|failures:)' /tmp/ft.log; fail=1
  else
    echo "tests passed  [$label]:"; grep -E '^test result' /tmp/ft.log | sed 's/^/    /'
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "=== ALL FEATURE COMBINATIONS PASSED (${#CONFIGS[@]} configurations) ==="
else
  echo "=== FAILURES PRESENT ==="
fi
exit "$fail"
